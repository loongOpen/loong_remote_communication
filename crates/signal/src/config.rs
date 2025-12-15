#[derive(Debug, Clone)]
pub struct MqttConfig {
    pub broker_host: String,
    pub broker_port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub keep_alive: u64,
    pub clean_session: bool,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker_host: "localhost".to_string(),
            broker_port: 1883,
            username: None,
            password: None,
            keep_alive: 60,
            clean_session: true,
        }
    }
}
