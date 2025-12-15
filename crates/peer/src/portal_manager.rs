use crate::config::PeerConfig;
use crate::portal::{Portal, PortalEvent};
use anyhow::{anyhow, Result};
use signal::{MqttConfig, Signal, SignalEvent, SignalRole};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Notify, RwLock};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{debug, error, info, trace, warn};

pub struct PortalManager {
    pub local_id: String,
    pub signal: Arc<Signal>,
    pub config: PeerConfig,
    portals: Arc<RwLock<HashMap<String, Arc<Portal>>>>,
    online_notifiers: Arc<RwLock<HashMap<String, Arc<Notify>>>>,
    portal_event_tx: mpsc::UnboundedSender<PortalEvent>,
}

/// Builder for PortalManager
#[derive(Default)]
pub struct PortalManagerBuilder {
    local_id: Option<String>,
    mqtt_config: Option<MqttConfig>,
    peer_config: Option<PeerConfig>,
}

impl PortalManagerBuilder {
    pub fn local_id(mut self, id: impl Into<String>) -> Self {
        self.local_id = Some(id.into());
        self
    }

    pub fn mqtt(mut self, config: MqttConfig) -> Self {
        self.mqtt_config = Some(config);
        self
    }

    pub fn peer(mut self, config: PeerConfig) -> Self {
        self.peer_config = Some(config);
        self
    }

    /// Build and start the PortalManager
    pub async fn run(self) -> Result<(Arc<PortalManager>, JoinHandle<()>)> {
        let local_id = self.local_id.ok_or_else(|| anyhow!("local_id is required"))?;
        let mqtt_config = self.mqtt_config.ok_or_else(|| anyhow!("mqtt config is required"))?;
        let peer_config = self.peer_config.unwrap_or_default();

        let (signal, signal_event_rx) =
            Signal::new(local_id.clone(), SignalRole::Caller, mqtt_config).await?;

        let signal = Arc::new(signal);
        let portals = Arc::new(RwLock::new(HashMap::new()));
        let online_notifiers = Arc::new(RwLock::new(HashMap::new()));
        let (portal_event_tx, portal_event_rx) = mpsc::unbounded_channel();

        let manager = Arc::new(PortalManager {
            local_id,
            signal,
            config: peer_config,
            portals,
            online_notifiers,
            portal_event_tx,
        });

        let m = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            m.event_loop(signal_event_rx, portal_event_rx).await;
        });

        Ok((manager, handle))
    }
}

impl PortalManager {
    /// Create a builder for PortalManager
    pub fn builder() -> PortalManagerBuilder {
        PortalManagerBuilder::default()
    }

    pub async fn create_portal(&self, remote_id: &str, addr_uri: String) -> Result<Arc<Portal>> {
        debug!("Creating portal for remote: {}, addr: {}", remote_id, addr_uri);

        if let Some(portal) = self.portals.read().await.get(remote_id).map(Arc::clone) {
            debug!("Portal to {} already exists, reusing", remote_id);
            return Ok(portal);
        }

        self.wait_remote_online(remote_id).await?;

        let portal = Portal::new(
            self.local_id.clone(),
            remote_id.to_string(),
            addr_uri,
            self.config.clone(),
            self.portal_event_tx.clone(),
        )
        .await?;

        {
            let mut portals = self.portals.write().await;
            portals.insert(remote_id.to_string(), Arc::clone(&portal));
            info!("Portal added: {}, total: {}", remote_id, portals.len());
        }

        if let Err(e) = portal.wait_connected().await {
            let mut portals = self.portals.write().await;
            portals.remove(remote_id);
            info!("Portal removed (connect failed): {}, total: {}", remote_id, portals.len());
            return Err(e);
        }

        Ok(portal)
    }

    pub async fn remove_portal(&self, remote_id: &str) -> Result<()> {
        debug!("Removing portal for: {}", remote_id);

        let portal = {
            let mut portals = self.portals.write().await;
            let p = portals.remove(remote_id);
            if p.is_some() {
                info!("Portal removed: {}, total: {}", remote_id, portals.len());
            }
            p
        };
        if let Some(p) = portal {
            p.close().await.ok();
        }

        if let Err(e) = self.signal.unsubscribe_remote_status(remote_id, SignalRole::Callee).await {
            warn!("Failed to unsubscribe remote status for {}: {}", remote_id, e);
        }

        let mut notifiers = self.online_notifiers.write().await;
        if notifiers.remove(remote_id).is_some() {
            debug!("Online notifier removed: {}, total: {}", remote_id, notifiers.len());
        }
        Ok(())
    }

    async fn event_loop(
        &self,
        mut signal_event_rx: mpsc::UnboundedReceiver<SignalEvent>,
        mut portal_event_rx: mpsc::UnboundedReceiver<PortalEvent>,
    ) {
        loop {
            tokio::select! {
                Some(event) = signal_event_rx.recv() => {
                    if self.handle_signal_event(event).await {
                        break;
                    }
                }
                Some(event) = portal_event_rx.recv() => {
                    self.handle_portal_event(event).await;
                }
            }
        }
        debug!("PortalManager event loop exited");
    }

    async fn handle_signal_event(&self, event: SignalEvent) -> bool {
        match event {
            SignalEvent::RemoteOnline(remote_id) => {
                debug!("Remote {} is online", remote_id);
                if let Some(notifier) = self.online_notifiers.read().await.get(&remote_id) {
                    notifier.notify_one();
                }
            }
            SignalEvent::RemoteOffline(remote_id) => {
                let mut portals = self.portals.write().await;
                if portals.remove(&remote_id).is_some() {
                    info!("Portal removed (offline): {}, total: {}", remote_id, portals.len());
                }
            }
            SignalEvent::SignalMessage(msg) => {
                trace!("Received signal message from {}", msg.from_id);
                if let Some(portal) = self.portals.read().await.get(&msg.from_id) {
                    if let Err(e) = portal.handle_signal_message(msg).await {
                        error!("Failed to handle signal message: {:?}", e);
                    }
                } else {
                    warn!("No portal found for: {}", msg.from_id);
                }
            }
            SignalEvent::Connected => debug!("Signal connected"),
            SignalEvent::Disconnected => {
                warn!("Signal disconnected, PortalManager exiting");
                return true;
            }
        }
        false
    }

    async fn handle_portal_event(&self, event: PortalEvent) {
        match event {
            PortalEvent::Offer { remote_id, payload } => {
                trace!("Sending offer to: {}", remote_id);
                if let Err(e) = self
                    .signal
                    .publish_signal_message(&remote_id, &payload, SignalRole::Callee)
                    .await
                {
                    error!("Failed to send offer to {}: {}", remote_id, e);
                }
            }
            PortalEvent::Candidate { remote_id, payload } => {
                trace!("Sending candidate to: {}", remote_id);
                if let Err(e) = self
                    .signal
                    .publish_signal_message(&remote_id, &payload, SignalRole::Callee)
                    .await
                {
                    error!("Failed to send candidate to {}: {}", remote_id, e);
                }
            }
            PortalEvent::Connected { remote_id } => debug!("{} connected", remote_id),
            PortalEvent::Closed { remote_id } => {
                let mut portals = self.portals.write().await;
                portals.remove(&remote_id);
                info!("Portal {} closed, removed, total: {}", remote_id, portals.len());
            }
        }
    }

    async fn wait_remote_online(&self, remote_id: &str) -> Result<()> {
        let notify = Arc::new(Notify::new());
        {
            let mut notifiers = self.online_notifiers.write().await;
            notifiers.insert(remote_id.to_string(), notify.clone());
            debug!("Online notifier added: {}, total: {}", remote_id, notifiers.len());
        }
        self.signal.subscribe_remote_status(remote_id, SignalRole::Callee).await?;

        match timeout(self.config.online_timeout, notify.notified()).await {
            Ok(_) => {
                debug!("Remote {} is now online", remote_id);
                Ok(())
            }
            Err(_) => {
                let mut notifiers = self.online_notifiers.write().await;
                notifiers.remove(remote_id);
                debug!(
                    "Online notifier removed (timeout): {}, total: {}",
                    remote_id,
                    notifiers.len()
                );
                Err(anyhow!(
                    "Timeout waiting for remote {} ({}s)",
                    remote_id,
                    self.config.online_timeout.as_secs()
                ))
            }
        }
    }
}
