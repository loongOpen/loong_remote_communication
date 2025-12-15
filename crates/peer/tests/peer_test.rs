use anyhow::{anyhow, Result};
use peer::portal_manager::PortalManager;
use peer::proxy_manager::ProxyManager;
use peer::PeerConfig;
use signal::MqttConfig;
use std::time::Duration;
use tokio::time::timeout;
use tracing::info;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info,webrtc=off,webrtc_sctp=off,turn=error")
        .try_init();

    rustls::crypto::ring::default_provider().install_default().ok();
}

fn test_mqtt_config() -> MqttConfig {
    MqttConfig {
        broker_host: "127.0.0.1".to_string(),
        broker_port: 1883,
        username: None,
        password: None,
        keep_alive: 60,
        clean_session: true,
    }
}

fn test_peer_config() -> PeerConfig {
    PeerConfig {
        online_timeout: Duration::from_secs(5),
        connect_timeout: Duration::from_secs(10),
        ..Default::default()
    }
}

#[tokio::test]
async fn test_portal_proxy_connection() -> Result<()> {
    init_tracing();

    // 启动 ProxyManager (被调用方)
    let proxy_addr = "127.0.0.1:19000";
    let (_proxy_manager, _) = ProxyManager::builder()
        .local_id("test_proxy_1")
        .mqtt(test_mqtt_config())
        .peer(test_peer_config())
        .target_addr(proxy_addr)
        .run()
        .await?;

    info!("ProxyManager started: {}", proxy_addr);

    // 启动 PortalManager (调用方)
    let (portal_manager, _) = PortalManager::builder()
        .local_id("test_portal_1")
        .mqtt(test_mqtt_config())
        .peer(test_peer_config())
        .run()
        .await?;

    info!("PortalManager started");

    // 创建 Portal 连接到 Proxy
    let portal_addr = "127.0.0.1:19001";
    let result = timeout(
        Duration::from_secs(15),
        portal_manager.create_portal("test_proxy_1", portal_addr.to_string()),
    )
    .await;

    match result {
        Ok(Ok(portal)) => {
            info!("Portal created successfully: {}", portal.addr_uri);
            Ok(())
        }
        Ok(Err(e)) => {
            info!("Connection failed: {}", e);
            Err(e)
        }
        Err(_) => {
            info!("Connection timeout");
            Err(anyhow!("Connection timeout"))
        }
    }
}

#[tokio::test]
async fn test_portal_timeout() -> Result<()> {
    init_tracing();

    // 启动 PortalManager，不启动 ProxyManager
    let (portal_manager, _handle) = PortalManager::builder()
        .local_id("test_portal_timeout")
        .mqtt(test_mqtt_config())
        .peer(PeerConfig {
            online_timeout: Duration::from_secs(2), // 短超时
            ..Default::default()
        })
        .run()
        .await?;

    // 尝试连接一个不存在的 Proxy
    let result =
        portal_manager.create_portal("nonexistent_proxy", "127.0.0.1:19999".to_string()).await;

    match result {
        Err(e) if e.to_string().contains("Timeout") => {
            info!("Timeout error caught as expected: {}", e);
            Ok(())
        }
        Ok(_) => Err(anyhow!("Should have timed out")),
        Err(e) => Err(anyhow!("Unexpected error: {}", e)),
    }
}
