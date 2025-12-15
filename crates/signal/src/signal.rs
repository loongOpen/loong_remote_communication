use anyhow::Result;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::config;
use crate::message;
use crate::topics;

#[derive(Debug)]
pub enum SignalEvent {
    SignalMessage(message::SignalPayload),
    RemoteOnline(String),
    RemoteOffline(String),
    Connected,
    Disconnected,
}

pub struct Signal {
    id: String,
    client: AsyncClient,
    event_loop_handle: JoinHandle<()>,
}

impl Signal {
    pub async fn new(
        id: String,
        role: message::SignalRole,
        config: config::MqttConfig,
    ) -> Result<(Self, mpsc::UnboundedReceiver<SignalEvent>)> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let status_topic = topics::get_status_topic(&id, role);
        let signal_topic = topics::get_signal_topic(&id, role);
        let client_id = format!("{}_{:?}", id, role);

        let mut mqtt_options = MqttOptions::new(client_id, &config.broker_host, config.broker_port);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(config.keep_alive));
        mqtt_options.set_clean_session(config.clean_session);

        if let (Some(ref user), Some(ref pass)) = (&config.username, &config.password) {
            mqtt_options.set_credentials(user, pass);
        }

        mqtt_options.set_last_will(rumqttc::LastWill {
            topic: status_topic.clone(),
            message: message::PeerStatus::Offline.as_ref().as_bytes().to_vec().into(),
            qos: QoS::ExactlyOnce,
            retain: true,
        });

        let (client, event_loop) = AsyncClient::new(mqtt_options, 10);

        let event_loop_handle = Self::start_event_loop(
            event_loop,
            event_tx,
            client.clone(),
            status_topic,
            signal_topic,
        );

        Ok((Self { id, client, event_loop_handle }, event_rx))
    }

    pub async fn subscribe_remote_status(
        &self,
        remote_id: &str,
        remote_role: message::SignalRole,
    ) -> Result<()> {
        let status_topic = topics::get_status_topic(remote_id, remote_role);
        self.client.subscribe(status_topic, QoS::ExactlyOnce).await?;
        Ok(())
    }

    pub async fn unsubscribe_remote_status(
        &self,
        remote_id: &str,
        remote_role: message::SignalRole,
    ) -> Result<()> {
        let status_topic = topics::get_status_topic(remote_id, remote_role);
        self.client.unsubscribe(status_topic).await?;
        Ok(())
    }

    pub async fn publish_signal_message(
        &self,
        remote_id: &str,
        msg: &message::SignalPayload,
        remote_role: message::SignalRole,
    ) -> Result<()> {
        let topic = topics::get_signal_topic(remote_id, remote_role);
        let payload = serde_json::to_string(msg)?;
        self.client.publish(topic, QoS::ExactlyOnce, false, payload.as_bytes()).await?;
        Ok(())
    }

    fn start_event_loop(
        mut event_loop: EventLoop,
        event_tx: mpsc::UnboundedSender<SignalEvent>,
        client: AsyncClient,
        status_topic: String,
        signal_topic: String,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match event_loop.poll().await {
                    Ok(event) => match event {
                        Event::Incoming(Packet::ConnAck(_)) => {
                            if let Err(e) = client
                                .publish(
                                    &status_topic,
                                    QoS::ExactlyOnce,
                                    true,
                                    message::PeerStatus::Online.as_ref().as_bytes(),
                                )
                                .await
                            {
                                tracing::error!("Failed to publish online status: {}", e);
                                break;
                            }
                            if let Err(e) = client.subscribe(&signal_topic, QoS::ExactlyOnce).await
                            {
                                tracing::error!("Failed to subscribe signal topic: {}", e);
                                break;
                            }
                            let _ = event_tx.send(SignalEvent::Connected);
                        }
                        Event::Incoming(Packet::Publish(p)) => {
                            Self::handle_publish(&event_tx, p);
                        }
                        Event::Incoming(Packet::Disconnect) => {
                            tracing::warn!("Disconnected from MQTT broker");
                            let _ = event_tx.send(SignalEvent::Disconnected);
                        }
                        _ => {}
                    },
                    Err(e) => {
                        tracing::error!("MQTT event loop error: {}", e);
                        break;
                    }
                }
            }
            tracing::error!("MQTT event loop exited");
            let _ = event_tx.send(SignalEvent::Disconnected);
        })
    }

    fn handle_publish(event_tx: &mpsc::UnboundedSender<SignalEvent>, p: rumqttc::Publish) {
        if let Some(remote_id) = topics::split_status_topic(&p.topic) {
            let status = String::from_utf8_lossy(&p.payload);
            let event = match status.parse::<message::PeerStatus>() {
                Ok(message::PeerStatus::Online) => SignalEvent::RemoteOnline(remote_id),
                Ok(message::PeerStatus::Offline) => SignalEvent::RemoteOffline(remote_id),
                Err(_) => return,
            };
            let _ = event_tx.send(event);
        } else if topics::split_signal_topic(&p.topic).is_some() {
            if let Ok(msg) = serde_json::from_slice(&p.payload) {
                let _ = event_tx.send(SignalEvent::SignalMessage(msg));
            }
        } else {
            tracing::warn!("Unknown topic: {}", &p.topic);
        }
    }
}

impl Drop for Signal {
    fn drop(&mut self) {
        tracing::info!("Dropping Signal instance for {}", self.id);
        self.event_loop_handle.abort();
    }
}
