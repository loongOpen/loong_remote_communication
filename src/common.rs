use anyhow::Result;
use clap::Args;
use peer::PeerConfig;
use signal::MqttConfig;
use std::time::Duration;

#[derive(Args, Debug, Clone)]
pub struct MqttArgs {
    /// MQTT broker URL (mqtt://host:port)
    #[arg(short, long, default_value = "mqtt://localhost:1883")]
    pub mqtt_broker: String,

    /// MQTT username, Optional
    #[arg(long)]
    pub mqtt_username: Option<String>,

    /// MQTT password, Optional
    #[arg(long)]
    pub mqtt_password: Option<String>,
}

impl MqttArgs {
    pub fn to_config(&self) -> Result<MqttConfig> {
        let url = self.mqtt_broker.trim();
        let without_proto = url
            .strip_prefix("mqtt://")
            .ok_or_else(|| anyhow::anyhow!("Broker URL must start with mqtt://"))?;

        let parts: Vec<&str> = without_proto.split(':').collect();
        let host = parts.first().ok_or_else(|| anyhow::anyhow!("Invalid broker URL"))?.to_string();
        let port = parts.get(1).map(|p| p.parse()).transpose()?.unwrap_or(1883);

        Ok(MqttConfig {
            broker_host: host,
            broker_port: port,
            username: self.mqtt_username.clone(),
            password: self.mqtt_password.clone(),
            keep_alive: 60,
            clean_session: true,
        })
    }
}

#[derive(Args, Debug, Clone)]
pub struct PeerArgs {
    /// STUN server URLs (e.g., stun:stun.l.google.com:19302), can specify multiple
    #[arg(long, default_values_t = vec!["stun:stun.l.google.com:19302".to_string()])]
    pub peer_stun: Vec<String>,

    /// TURN server URLs (e.g., turn:user:pass@host:port), can specify multiple
    #[arg(long)]
    pub peer_turn: Vec<String>,

    /// Timeout for waiting remote online (seconds)
    #[arg(long, default_value = "5")]
    pub online_timeout: u64,

    /// Timeout for WebRTC connection (seconds)
    #[arg(long, default_value = "5")]
    pub connect_timeout: u64,
}

impl PeerArgs {
    pub fn to_config(&self) -> PeerConfig {
        use peer::config::IceServer;

        let mut ice_servers: Vec<IceServer> =
            self.peer_stun.iter().map(|s| IceServer::stun(s)).collect();

        for turn_url in &self.peer_turn {
            if let Some(server) = Self::parse_turn_url(turn_url) {
                ice_servers.push(server);
            }
        }

        PeerConfig {
            ice_servers,
            online_timeout: Duration::from_secs(self.online_timeout),
            connect_timeout: Duration::from_secs(self.connect_timeout),
            ..Default::default()
        }
    }

    /// Parse TURN URL: turn:user:pass@host:port or turn:host:port
    fn parse_turn_url(url: &str) -> Option<peer::config::IceServer> {
        let url = url.strip_prefix("turn:").unwrap_or(url);

        if let Some((credentials, host)) = url.rsplit_once('@') {
            // turn:user:pass@host:port
            let parts: Vec<&str> = credentials.splitn(2, ':').collect();
            let (user, pass) = match parts.as_slice() {
                [u, p] => (*u, *p),
                [u] => (*u, ""),
                _ => return None,
            };
            Some(peer::config::IceServer::turn(&format!("turn:{}", host), user, pass))
        } else {
            // turn:host:port (no credentials)
            Some(peer::config::IceServer::turn(&format!("turn:{}", url), "", ""))
        }
    }
}

pub fn init_runtime() {
    // Windows: pause on exit for debugging
    #[cfg(windows)]
    {
        let _ = ctrlc::set_handler(move || {
            println!("\nExiting...");
            std::process::exit(0);
        });
        std::panic::set_hook(Box::new(|info| {
            eprintln!("\nPanic: {}", info);
            use std::io::{self, Write};
            print!("\nPress Enter to exit...");
            let _ = io::stdout().flush();
            let _ = io::stdin().read_line(&mut String::new());
        }));
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,webrtc=off,webrtc_sctp=off,turn=error".into()),
        )
        .init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install CryptoProvider");
}
