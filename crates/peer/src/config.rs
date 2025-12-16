use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, trace};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::api::API;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;

pub static RTC_API: Lazy<Arc<API>> = Lazy::new(|| {
    trace!("Initializing RTC API");
    let registry = Registry::new();
    let mut m = MediaEngine::default();
    let _ = m.register_default_codecs();

    match register_default_interceptors(registry, &mut m) {
        Ok(registry) => {
            let api =
                APIBuilder::new().with_media_engine(m).with_interceptor_registry(registry).build();
            Arc::new(api)
        }
        Err(e) => {
            error!("Failed to register default interceptors: {}", e);
            let registry = Registry::new();
            let api =
                APIBuilder::new().with_media_engine(m).with_interceptor_registry(registry).build();
            Arc::new(api)
        }
    }
});

#[derive(Debug, Clone)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

impl IceServer {
    pub fn stun(url: &str) -> Self {
        Self { urls: vec![url.to_string()], username: None, credential: None }
    }

    pub fn turn(url: &str, username: &str, credential: &str) -> Self {
        Self {
            urls: vec![url.to_string()],
            username: Some(username.to_string()),
            credential: Some(credential.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PeerConfig {
    pub ice_servers: Vec<IceServer>,
    pub online_timeout: Duration,
    pub connect_timeout: Duration,
    pub datachannel_timeout: Duration,
    pub ice_gathering_timeout: Duration,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            ice_servers: vec![IceServer::stun("stun:stun.l.google.com:19302")],
            online_timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(5),
            datachannel_timeout: Duration::from_secs(5),
            ice_gathering_timeout: Duration::from_secs(5),
        }
    }
}

impl PeerConfig {
    pub fn to_rtc_configuration(&self) -> RTCConfiguration {
        let ice_servers = self
            .ice_servers
            .iter()
            .map(|s| RTCIceServer {
                urls: s.urls.clone(),
                username: s.username.clone().unwrap_or_default(),
                credential: s.credential.clone().unwrap_or_default(),
            })
            .collect();

        RTCConfiguration { ice_servers, ..Default::default() }
    }
}
