use crate::binder::spawn_dc_socket_bridge;
use crate::config::{PeerConfig, RTC_API};
use anyhow::Result;
use signal::{SignalPayload, SignalType};
use std::sync::Arc;
use tokio::net::TcpStream;
#[cfg(unix)]
use tokio::net::UnixStream;
use tokio::sync::{mpsc, Notify};
use tokio::time::timeout;
use tracing::{debug, error, trace, warn};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

#[derive(Debug, Clone)]
pub enum ProxyEvent {
    Candidate { remote_id: String, payload: SignalPayload },
    Answer { remote_id: String, payload: SignalPayload },
    Connected { remote_id: String },
    Closed { remote_id: String },
}

#[allow(dead_code)]
pub struct Proxy {
    pub local_id: String,
    pub remote_id: String,
    pub addr_uri: String,
    pub config: PeerConfig,
    pc: Arc<RTCPeerConnection>,
    connected_notify: Arc<Notify>,
}

impl Proxy {
    pub async fn new(
        local_id: String,
        remote_id: String,
        addr_uri: String,
        config: PeerConfig,
        event_tx: mpsc::UnboundedSender<ProxyEvent>,
        offer: SignalPayload,
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
        Self::setup_data_channel_callback(&pc, addr_uri.clone());

        let desc = RTCSessionDescription::offer(offer.payload)?;
        pc.set_remote_description(desc).await?;

        let answer = pc.create_answer(None).await?;
        pc.set_local_description(answer.clone()).await?;

        let payload = SignalPayload {
            from_id: local_id.clone(),
            payload: answer.sdp,
            signal_type: SignalType::Answer,
        };
        event_tx.send(ProxyEvent::Answer { remote_id: remote_id.clone(), payload })?;

        let proxy = Arc::new(Self { local_id, remote_id, addr_uri, config, pc, connected_notify });
        Ok(proxy)
    }

    #[allow(dead_code)]
    pub async fn wait_connected(&self) -> Result<()> {
        match timeout(self.config.connect_timeout, self.connected_notify.notified()).await {
            Ok(_) => {
                debug!("Proxy connected to {}", self.remote_id);
                Ok(())
            }
            Err(_) => Err(anyhow::anyhow!(
                "Timeout waiting for connection to {} ({}s)",
                self.remote_id,
                self.config.connect_timeout.as_secs()
            )),
        }
    }

    pub async fn handle_signal_message(&self, msg: SignalPayload) -> Result<()> {
        match msg.signal_type {
            SignalType::Candidate => {
                let candidate =
                    RTCIceCandidateInit { candidate: msg.payload, ..Default::default() };
                self.pc.add_ice_candidate(candidate).await?;
                trace!("ICE candidate added for {}", self.remote_id);
            }
            _ => warn!("Unexpected message type: {:?} from {}", msg.signal_type, msg.from_id),
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.pc.connection_state() == RTCPeerConnectionState::Connected
    }

    pub fn is_active(&self) -> bool {
        !matches!(
            self.pc.connection_state(),
            RTCPeerConnectionState::Failed
                | RTCPeerConnectionState::Closed
                | RTCPeerConnectionState::Disconnected
        )
    }
}

impl Proxy {
    fn setup_ice_candidate_callback(
        pc: &RTCPeerConnection,
        event_tx: mpsc::UnboundedSender<ProxyEvent>,
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
                        let _ = event_tx.send(ProxyEvent::Candidate { remote_id, payload });
                    }
                }
            })
        }));
    }

    fn setup_connection_state_callback(
        pc: &RTCPeerConnection,
        notify: Arc<Notify>,
        event_tx: mpsc::UnboundedSender<ProxyEvent>,
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
                        let _ = event_tx.send(ProxyEvent::Connected { remote_id: rid });
                    }
                    RTCPeerConnectionState::Disconnected => {
                        debug!("PeerConnection disconnected for {}", rid);
                        let _ = event_tx.send(ProxyEvent::Closed { remote_id: rid });
                    }
                    RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed => {
                        let _ = event_tx.send(ProxyEvent::Closed { remote_id: rid.clone() });
                        if state == RTCPeerConnectionState::Failed {
                            warn!("PeerConnection failed for {}", rid);
                        }
                    }
                    _ => {}
                }
            })
        }));
    }

    fn setup_data_channel_callback(pc: &RTCPeerConnection, addr_uri: String) {
        pc.on_data_channel(Box::new(move |dc| {
            let addr_uri = addr_uri.clone();
            Box::pin(async move {
                if dc.label() == "DEFAULT" {
                    return;
                }
                debug!("New DataChannel: {}", dc.label());
                Self::connect_and_bridge(dc, &addr_uri).await;
            })
        }));
    }

    async fn connect_and_bridge(dc: Arc<webrtc::data_channel::RTCDataChannel>, addr_uri: &str) {
        if addr_uri.starts_with("unix://") {
            #[cfg(unix)]
            {
                let socket_path = addr_uri.trim_start_matches("unix://");
                match UnixStream::connect(socket_path).await {
                    Ok(stream) => {
                        debug!("Connected to Unix socket: {}", socket_path);
                        spawn_dc_socket_bridge(dc, stream);
                    }
                    Err(e) => error!("Failed to connect to Unix socket {}: {}", socket_path, e),
                }
            }
            #[cfg(not(unix))]
            error!("Unix socket not supported on this platform");
        } else {
            match TcpStream::connect(addr_uri).await {
                Ok(stream) => {
                    debug!("Connected to TCP: {}", addr_uri);
                    spawn_dc_socket_bridge(dc, stream);
                }
                Err(e) => error!("Failed to connect to {}: {}", addr_uri, e),
            }
        }
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        let pc = self.pc.clone();
        let remote_id = self.remote_id.clone();
        tokio::spawn(async move {
            if let Err(e) = pc.close().await {
                warn!("Failed to close PeerConnection for {}: {}", remote_id, e);
            }
        });

        debug!("Proxy dropped for {}", self.remote_id);
    }
}
