use crate::config::PeerConfig;
use crate::proxy::{Proxy, ProxyEvent};
use anyhow::{anyhow, Result};
use signal::{MqttConfig, Signal, SignalEvent, SignalRole, SignalType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace, warn};

pub struct ProxyManager {
    pub local_id: String,
    pub signal: Arc<Signal>,
    pub config: PeerConfig,
    pub target_addr: String,
    proxies: Arc<RwLock<HashMap<String, Arc<Proxy>>>>,
    proxy_event_tx: mpsc::UnboundedSender<ProxyEvent>,
}

/// Builder for ProxyManager
#[derive(Default)]
pub struct ProxyManagerBuilder {
    local_id: Option<String>,
    mqtt_config: Option<MqttConfig>,
    peer_config: Option<PeerConfig>,
    target_addr: Option<String>,
}

impl ProxyManagerBuilder {
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

    pub fn target_addr(mut self, addr: impl Into<String>) -> Self {
        self.target_addr = Some(addr.into());
        self
    }

    /// Build and start the ProxyManager
    pub async fn run(self) -> Result<(Arc<ProxyManager>, JoinHandle<()>)> {
        let local_id = self.local_id.ok_or_else(|| anyhow!("local_id is required"))?;
        let mqtt_config = self.mqtt_config.ok_or_else(|| anyhow!("mqtt config is required"))?;
        let peer_config = self.peer_config.unwrap_or_default();
        let target_addr = self.target_addr.ok_or_else(|| anyhow!("target_addr is required"))?;

        #[cfg(not(unix))]
        if target_addr.starts_with("unix://") {
            return Err(anyhow!("Unix socket not supported on this platform"));
        }

        let (signal, signal_event_rx) =
            Signal::new(local_id.clone(), SignalRole::Callee, mqtt_config).await?;

        let signal = Arc::new(signal);
        let proxies = Arc::new(RwLock::new(HashMap::new()));
        let (proxy_event_tx, proxy_event_rx) = mpsc::unbounded_channel();

        let manager = Arc::new(ProxyManager {
            local_id,
            signal,
            config: peer_config,
            target_addr,
            proxies,
            proxy_event_tx,
        });

        let m = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            m.event_loop(signal_event_rx, proxy_event_rx).await;
        });

        Ok((manager, handle))
    }
}

impl ProxyManager {
    /// Create a builder for ProxyManager
    pub fn builder() -> ProxyManagerBuilder {
        ProxyManagerBuilder::default()
    }

    pub async fn connection_count(&self) -> usize {
        self.proxies.read().await.len()
    }

    async fn event_loop(
        &self,
        mut signal_event_rx: mpsc::UnboundedReceiver<SignalEvent>,
        mut proxy_event_rx: mpsc::UnboundedReceiver<ProxyEvent>,
    ) {
        loop {
            tokio::select! {
                Some(event) = signal_event_rx.recv() => {
                    if self.handle_signal_event(event).await {
                        break;
                    }
                }
                Some(event) = proxy_event_rx.recv() => {
                    self.handle_proxy_event(event).await;
                }
            }
        }
        debug!("ProxyManager event loop exited");
    }

    async fn handle_signal_event(&self, event: SignalEvent) -> bool {
        match event {
            SignalEvent::SignalMessage(msg) => {
                if let Err(e) = self.handle_signal_message(msg).await {
                    error!("Error handling signal message: {}", e);
                }
            }
            SignalEvent::Connected => debug!("Signal connected"),
            SignalEvent::Disconnected => {
                warn!("Signal disconnected, ProxyManager exiting");
                return true;
            }
            _ => {}
        }
        false
    }

    async fn handle_signal_message(&self, msg: signal::SignalPayload) -> Result<()> {
        match msg.signal_type {
            SignalType::Offer => {
                let remote_id = msg.from_id.clone();
                debug!("Received offer from: {}", remote_id);

                let proxy = Proxy::new(
                    self.local_id.clone(),
                    remote_id.clone(),
                    self.target_addr.clone(),
                    self.config.clone(),
                    self.proxy_event_tx.clone(),
                    msg,
                )
                .await?;

                let mut proxies = self.proxies.write().await;
                proxies.insert(remote_id.clone(), proxy);
                info!(
                    "Proxy created: {} -> {} (target: {}), total: {}",
                    self.local_id,
                    remote_id,
                    self.target_addr,
                    proxies.len()
                );
                // info!("Proxy added: {}, total: {}", remote_id, proxies.len());
            }
            SignalType::Candidate => {
                trace!("Received candidate from: {}", msg.from_id);
                if let Some(proxy) = self.proxies.read().await.get(&msg.from_id) {
                    proxy.handle_signal_message(msg).await?;
                } else {
                    warn!("No proxy found for {}, ignoring candidate", msg.from_id);
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_proxy_event(&self, event: ProxyEvent) {
        match event {
            ProxyEvent::Answer { remote_id, payload } => {
                trace!("Sending answer to: {}", remote_id);
                if let Err(e) = self
                    .signal
                    .publish_signal_message(&remote_id, &payload, SignalRole::Caller)
                    .await
                {
                    error!("Failed to send answer to {}: {}", remote_id, e);
                }
            }
            ProxyEvent::Candidate { remote_id, payload } => {
                trace!("Sending candidate to: {}", remote_id);
                if let Err(e) = self
                    .signal
                    .publish_signal_message(&remote_id, &payload, SignalRole::Caller)
                    .await
                {
                    error!("Failed to send candidate to {}: {}", remote_id, e);
                }
            }
            ProxyEvent::Connected { remote_id } => debug!("{} connected", remote_id),
            ProxyEvent::Closed { remote_id } => {
                self.try_remove_proxy(&remote_id).await;
            }
        }
    }

    async fn try_remove_proxy(&self, remote_id: &str) {
        let mut proxies = self.proxies.write().await;
        if let Some(proxy) = proxies.get(remote_id) {
            if !proxy.is_active() {
                proxies.remove(remote_id);
                info!("{} disconnected, count: {}", remote_id, proxies.len());
            } else {
                debug!("{} close event ignored, still active", remote_id);
            }
        }
    }
}
