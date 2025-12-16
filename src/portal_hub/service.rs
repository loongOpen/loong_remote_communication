use anyhow::{anyhow, Result};
use peer::portal_manager::PortalManager;
use peer::PeerConfig;
use signal::MqttConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Portal creation request
#[derive(Debug, Clone)]
pub struct CreatePortalRequest {
    pub user_id: String,
    pub robot_id: String,
    pub service_name: String,
    pub portal_type: PortalType,
    pub inet_port: Option<String>,
    pub unix_file: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub enum PortalType {
    #[default]
    Inet,
    Unix,
}

/// Portal creation response
#[derive(Debug, Clone)]
pub struct CreatePortalResponse {
    pub uri: String,
}

struct ManagedPortalManager {
    manager: Arc<PortalManager>,
    #[allow(dead_code)]
    event_loop_handle: tokio::task::JoinHandle<()>,
}

/// Shared service for managing WebRTC portals
pub struct PortalHubService {
    default_user_id: String,
    mq_cfg: MqttConfig,
    peer_cfg: PeerConfig,
    managers: RwLock<HashMap<String, Arc<ManagedPortalManager>>>,
}

impl PortalHubService {
    pub fn new(default_user_id: String, mq_cfg: MqttConfig, peer_cfg: PeerConfig) -> Self {
        Self { default_user_id, mq_cfg, peer_cfg, managers: RwLock::new(HashMap::new()) }
    }

    /// Create a portal to remote service
    pub async fn create_portal(&self, req: CreatePortalRequest) -> Result<CreatePortalResponse> {
        let user_id = self.resolve_user_id(&req.user_id)?;
        let remote_id = Self::build_remote_id(&req.robot_id, &req.service_name);
        let addr_uri = Self::build_addr_uri(&req);

        let manager = self.get_or_create_manager(&user_id).await?;

        match manager.create_portal(&remote_id, addr_uri).await {
            Ok(portal) => {
                let uri = portal.addr_uri.clone();
                info!("✅ Portal {} created, URI: {}", remote_id, uri);
                Ok(CreatePortalResponse { uri })
            }
            Err(e) => {
                warn!("❌ Failed to create portal {}: {}", remote_id, e);
                Err(e)
            }
        }
    }

    /// Destroy a portal
    pub async fn destroy_portal(&self, req: CreatePortalRequest) -> Result<()> {
        let user_id = self.resolve_user_id(&req.user_id)?;
        let remote_id = Self::build_remote_id(&req.robot_id, &req.service_name);

        let managers = self.managers.read().await;
        if let Some(managed) = managers.get(&user_id) {
            managed.manager.remove_portal(&remote_id).await?;
            info!("🧹 Destroyed portal: user={}, remote={}", user_id, remote_id);
        }

        Ok(())
    }

    fn resolve_user_id(&self, request_user_id: &str) -> Result<String> {
        if !request_user_id.is_empty() {
            return Ok(request_user_id.to_string());
        }
        if !self.default_user_id.is_empty() {
            return Ok(self.default_user_id.clone());
        }
        Err(anyhow!("User id is empty"))
    }

    async fn get_or_create_manager(&self, user_id: &str) -> Result<Arc<PortalManager>> {
        {
            let managers = self.managers.read().await;
            if let Some(managed) = managers.get(user_id) {
                return Ok(Arc::clone(&managed.manager));
            }
        }

        let mut managers = self.managers.write().await;
        if let Some(managed) = managers.get(user_id) {
            return Ok(Arc::clone(&managed.manager));
        }

        let (manager, event_loop_handle) = PortalManager::builder()
            .local_id(user_id)
            .mqtt(self.mq_cfg.clone())
            .peer(self.peer_cfg.clone())
            .run()
            .await?;

        managers.insert(
            user_id.to_string(),
            Arc::new(ManagedPortalManager { manager: Arc::clone(&manager), event_loop_handle }),
        );

        info!("Created PortalManager for user: {}", user_id);
        Ok(manager)
    }

    fn build_remote_id(robot_id: &str, service_name: &str) -> String {
        format!("{}-{}", robot_id, service_name)
    }

    fn build_addr_uri(req: &CreatePortalRequest) -> String {
        match req.portal_type {
            PortalType::Unix => {
                if let Some(ref file) = req.unix_file {
                    if !file.is_empty() {
                        return format!("unix://{}", file);
                    }
                }
                format!("unix:///tmp/lrc_{}_{}.sock", req.robot_id, req.service_name)
            }
            PortalType::Inet => {
                let port = req.inet_port.as_deref().unwrap_or("0");
                format!("0.0.0.0:{}", port)
            }
        }
    }
}
