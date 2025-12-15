use crate::binder::spawn_dc_socket_bridge;
use crate::config::{PeerConfig, RTC_API};
use anyhow::Result;
use chrono::Utc;
use signal::{SignalPayload, SignalType};
use std::sync::Arc;
use tokio::net::TcpListener;
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::sync::{mpsc, Notify};
use tokio::task::AbortHandle;
use tokio::time::timeout;
use tracing::{debug, error, trace, warn};
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

#[derive(Debug, Clone)]
pub enum PortalEvent {
    Candidate { remote_id: String, payload: SignalPayload },
    Offer { remote_id: String, payload: SignalPayload },
    Connected { remote_id: String },
    Closed { remote_id: String },
}

pub struct Portal {
    pub local_id: String,
    pub remote_id: String,
    pub addr_uri: String,
    pub config: PeerConfig,
    pc: Arc<RTCPeerConnection>,
    connected_notify: Arc<Notify>,
    listener_handle: AbortHandle,
}

impl Portal {
    pub async fn new(
        local_id: String,
        remote_id: String,
        addr_uri: String,
        config: PeerConfig,
        event_tx: mpsc::UnboundedSender<PortalEvent>,
    ) -> Result<Arc<Self>> {
        let rtc_config = config.to_rtc_configuration();
        let pc = Arc::new(RTC_API.new_peer_connection(rtc_config).await?);
        let connected_notify = Arc::new(Notify::new());

        Self::setup_ice_candidate_callback(
            &pc,
            event_tx.clone(),
            local_id.clone(),
            remote_id.clone(),
        );
        Self::setup_connection_state_callback(
            &pc,
            connected_notify.clone(),
            event_tx.clone(),
            remote_id.clone(),
        );

        let dc = pc.create_data_channel("DEFAULT", None).await?;
        dc.on_open(Box::new(|| Box::pin(async {})));

        let offer = pc.create_offer(None).await?;
        pc.set_local_description(offer.clone()).await?;
        let payload = SignalPayload {
            from_id: local_id.clone(),
            payload: offer.sdp,
            signal_type: SignalType::Offer,
        };
        event_tx.send(PortalEvent::Offer { remote_id: remote_id.clone(), payload })?;

        let listener_handle =
            Self::start_listener(&addr_uri, pc.clone(), local_id.clone(), remote_id.clone())
                .await?;

        let portal = Arc::new(Self {
            local_id,
            remote_id,
            addr_uri,
            config,
            pc,
            connected_notify,
            listener_handle,
        });

        debug!("Portal created for {}", portal.remote_id);
        Ok(portal)
    }

    pub async fn wait_connected(&self) -> Result<()> {
        match timeout(self.config.connect_timeout, self.connected_notify.notified()).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!(
                "Timeout waiting for connection to {} ({}s)",
                self.remote_id,
                self.config.connect_timeout.as_secs()
            )),
        }
    }

    pub async fn create_data_channel(&self) -> Result<Arc<RTCDataChannel>> {
        let label = format!("{}-{}", self.local_id, Utc::now().timestamp_millis());
        let dc = self.pc.create_data_channel(&label, None).await?;
        Ok(dc)
    }

    pub async fn handle_signal_message(&self, msg: SignalPayload) -> Result<()> {
        match msg.signal_type {
            SignalType::Answer => {
                let answer = RTCSessionDescription::answer(msg.payload)?;
                self.pc.set_remote_description(answer).await?;
                trace!("Answer set for {}", self.remote_id);
            }
            SignalType::Candidate => {
                let candidate =
                    RTCIceCandidateInit { candidate: msg.payload, ..Default::default() };
                self.pc.add_ice_candidate(candidate).await?;
                trace!("ICE candidate added for {}", self.remote_id);
            }
            _ => warn!("Unexpected message type: {:?}", msg.signal_type),
        }
        Ok(())
    }

    pub async fn close(&self) -> Result<()> {
        self.listener_handle.abort();
        self.pc.close().await?;
        debug!("Closed PeerConnection for {}", self.remote_id);
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.pc.connection_state() == RTCPeerConnectionState::Connected
    }
}

impl Portal {
    async fn start_listener(
        addr_uri: &str,
        pc: Arc<RTCPeerConnection>,
        local_id: String,
        remote_id: String,
    ) -> Result<AbortHandle> {
        let abort_handle = if addr_uri.starts_with("unix://") {
            #[cfg(unix)]
            {
                let socket_path = addr_uri.trim_start_matches("unix://");
                let _ = std::fs::remove_file(socket_path);
                let listener = UnixListener::bind(socket_path)?;
                debug!("Portal listening on unix://{}", socket_path);
                let handle =
                    tokio::spawn(Self::accept_loop_unix(listener, pc, local_id, remote_id));
                handle.abort_handle()
            }
            #[cfg(not(unix))]
            return Err(anyhow::anyhow!("Unix socket not supported"));
        } else {
            let listener = TcpListener::bind(addr_uri).await?;
            debug!("Portal listening on {}", addr_uri);
            let handle = tokio::spawn(Self::accept_loop_tcp(listener, pc, local_id, remote_id));
            handle.abort_handle()
        };
        Ok(abort_handle)
    }

    async fn accept_loop_tcp(
        listener: TcpListener,
        pc: Arc<RTCPeerConnection>,
        local_id: String,
        remote_id: String,
    ) {
        loop {
            let (socket, addr) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    error!("TCP accept failed: {:?}", e);
                    continue;
                }
            };
            debug!("New TCP connection from {} for {}", addr, remote_id);
            if !Self::handle_new_connection(&pc, &local_id, socket).await {
                break;
            }
        }
    }

    #[cfg(unix)]
    async fn accept_loop_unix(
        listener: UnixListener,
        pc: Arc<RTCPeerConnection>,
        local_id: String,
        remote_id: String,
    ) {
        loop {
            let (socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    error!("Unix socket accept failed: {:?}", e);
                    continue;
                }
            };
            debug!("New Unix socket connection for {}", remote_id);
            if !Self::handle_new_connection(&pc, &local_id, socket).await {
                break;
            }
        }
    }

    async fn handle_new_connection<S>(pc: &RTCPeerConnection, local_id: &str, socket: S) -> bool
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + 'static,
    {
        let state = pc.connection_state();
        if state == RTCPeerConnectionState::Closed || state == RTCPeerConnectionState::Failed {
            warn!("PeerConnection closed/failed, stopping accept loop");
            return false;
        }

        let label = format!("{}-{}", local_id, Utc::now().timestamp_millis());
        let dc = match pc.create_data_channel(&label, None).await {
            Ok(dc) => dc,
            Err(e) => {
                error!("create_data_channel failed: {:?}", e);
                return true;
            }
        };

        spawn_dc_socket_bridge(dc, socket);
        true
    }

    fn setup_ice_candidate_callback(
        pc: &RTCPeerConnection,
        event_tx: mpsc::UnboundedSender<PortalEvent>,
        local_id: String,
        remote_id: String,
    ) {
        pc.on_ice_candidate(Box::new(move |c| {
            let event_tx = event_tx.clone();
            let local_id = local_id.clone();
            let remote_id = remote_id.clone();
            Box::pin(async move {
                if let Some(candidate) = c {
                    if let Ok(json) = candidate.to_json() {
                        let payload = SignalPayload {
                            from_id: local_id,
                            payload: json.candidate,
                            signal_type: SignalType::Candidate,
                        };
                        let _ = event_tx.send(PortalEvent::Candidate { remote_id, payload });
                    }
                }
            })
        }));
    }

    fn setup_connection_state_callback(
        pc: &RTCPeerConnection,
        notify: Arc<Notify>,
        event_tx: mpsc::UnboundedSender<PortalEvent>,
        remote_id: String,
    ) {
        pc.on_peer_connection_state_change(Box::new(move |state| {
            let notify = notify.clone();
            let event_tx = event_tx.clone();
            let rid = remote_id.clone();
            Box::pin(async move {
                trace!("PeerConnection state for {}: {:?}", rid, state);
                match state {
                    RTCPeerConnectionState::Connected => {
                        notify.notify_one();
                        let _ = event_tx.send(PortalEvent::Connected { remote_id: rid });
                    }
                    RTCPeerConnectionState::Disconnected => {
                        debug!("PeerConnection disconnected for {}", rid);
                        let _ = event_tx.send(PortalEvent::Closed { remote_id: rid });
                    }
                    RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed => {
                        let _ = event_tx.send(PortalEvent::Closed { remote_id: rid.clone() });
                        if state == RTCPeerConnectionState::Failed {
                            warn!("PeerConnection failed for {}", rid);
                        }
                    }
                    _ => {}
                }
            })
        }));
    }
}

impl Drop for Portal {
    fn drop(&mut self) {
        self.listener_handle.abort();

        #[cfg(unix)]
        if self.addr_uri.starts_with("unix://") {
            let socket_path = self.addr_uri.trim_start_matches("unix://");
            let _ = std::fs::remove_file(socket_path);
        }

        let pc = self.pc.clone();
        let remote_id = self.remote_id.clone();
        tokio::spawn(async move {
            if let Err(e) = pc.close().await {
                warn!("Failed to close PeerConnection for {}: {}", remote_id, e);
            }
        });

        debug!("Portal dropped for {}", self.remote_id);
    }
}
