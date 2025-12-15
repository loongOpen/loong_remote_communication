use signal::{
    MqttConfig, Signal, SignalEvent, SignalPayload as SignalMessage, SignalRole, SignalType,
};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::info;

fn init_tracing() {
    let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
}

fn test_config() -> MqttConfig {
    MqttConfig {
        broker_host: "127.0.0.1".to_string(),
        broker_port: 1883,
        username: None,
        password: None,
        keep_alive: 60,
        clean_session: true,
    }
}

/// 等待远端上线事件
async fn wait_for_remote_online(
    event_rx: &mut mpsc::UnboundedReceiver<SignalEvent>,
    expected_id: &str,
) {
    info!("Waiting for remote online: {}", expected_id);
    loop {
        match timeout(Duration::from_secs(5), event_rx.recv()).await {
            Ok(Some(SignalEvent::RemoteOnline(id))) if id == expected_id => {
                info!("Remote {} is now online!", id);
                break;
            }
            Ok(Some(event)) => {
                info!("Received unrelated event: {:?}", event);
            }
            Ok(None) => {
                panic!("Channel closed unexpectedly");
            }
            Err(_) => {
                panic!("Timed out waiting for remote online: {}", expected_id);
            }
        }
    }
}

/// 等待远端下线事件
async fn wait_for_remote_offline(
    event_rx: &mut mpsc::UnboundedReceiver<SignalEvent>,
    expected_id: &str,
) {
    info!("Waiting for remote offline: {}", expected_id);
    loop {
        match timeout(Duration::from_secs(5), event_rx.recv()).await {
            Ok(Some(SignalEvent::RemoteOffline(id))) if id == expected_id => {
                info!("Remote {} is now offline!", id);
                break;
            }
            Ok(Some(event)) => {
                info!("Received unrelated event: {:?}", event);
            }
            Ok(None) => {
                panic!("Channel closed unexpectedly");
            }
            Err(_) => {
                panic!("Timed out waiting for remote offline: {}", expected_id);
            }
        }
    }
}

#[tokio::test]
async fn test_caller_callee_exchange() {
    init_tracing();

    // 创建 caller 和 callee
    let (caller, mut caller_rx) =
        Signal::new("caller1".to_string(), SignalRole::Caller, test_config())
            .await
            .expect("Failed to create caller");

    let (callee, mut callee_rx) =
        Signal::new("callee1".to_string(), SignalRole::Callee, test_config())
            .await
            .expect("Failed to create callee");

    // 等待连接
    assert!(matches!(
        timeout(Duration::from_secs(5), caller_rx.recv()).await,
        Ok(Some(SignalEvent::Connected))
    ));
    assert!(matches!(
        timeout(Duration::from_secs(5), callee_rx.recv()).await,
        Ok(Some(SignalEvent::Connected))
    ));
    info!("Both connected");

    // Caller 订阅 callee 的状态
    caller
        .subscribe_remote_status("callee1", SignalRole::Callee)
        .await
        .expect("Failed to subscribe");

    // Callee 订阅 caller 的状态
    callee
        .subscribe_remote_status("caller1", SignalRole::Caller)
        .await
        .expect("Failed to subscribe");

    // 等待对方上线事件
    wait_for_remote_online(&mut caller_rx, "callee1").await;
    wait_for_remote_online(&mut callee_rx, "caller1").await;

    info!("Both detected each other online");

    // Caller 发送 offer
    let offer = SignalMessage {
        from_id: "caller1".to_string(),
        payload: "offer_sdp".to_string(),
        signal_type: SignalType::Offer,
    };

    caller
        .publish_signal_message("callee1", &offer, SignalRole::Callee)
        .await
        .expect("Failed to send offer");
    info!("Caller sent offer");

    // Callee 收到 offer
    if let Ok(Some(SignalEvent::SignalMessage(msg))) =
        timeout(Duration::from_secs(5), callee_rx.recv()).await
    {
        assert_eq!(msg.from_id, "caller1");
        assert_eq!(msg.signal_type, SignalType::Offer);
        assert_eq!(msg.payload, "offer_sdp");
        info!("Callee received offer");

        // Callee 发送 answer
        let answer = SignalMessage {
            from_id: "callee1".to_string(),
            payload: "answer_sdp".to_string(),
            signal_type: SignalType::Answer,
        };

        callee
            .publish_signal_message("caller1", &answer, SignalRole::Caller)
            .await
            .expect("Failed to send answer");
        info!("Callee sent answer");
    } else {
        panic!("Callee didn't receive offer");
    }

    // Caller 收到 answer
    if let Ok(Some(SignalEvent::SignalMessage(msg))) =
        timeout(Duration::from_secs(5), caller_rx.recv()).await
    {
        assert_eq!(msg.from_id, "callee1");
        assert_eq!(msg.signal_type, SignalType::Answer);
        assert_eq!(msg.payload, "answer_sdp");
    } else {
        panic!("Caller didn't receive answer");
    }

    // Caller 发送 ICE candidate
    let ice = SignalMessage {
        from_id: "caller1".to_string(),
        payload: "ice_candidate_1".to_string(),
        signal_type: SignalType::Candidate,
    };

    caller
        .publish_signal_message("callee1", &ice, SignalRole::Callee)
        .await
        .expect("Failed to send ICE");
    info!("Caller sent ICE candidate");

    // Callee 收到 ICE
    if let Ok(Some(SignalEvent::SignalMessage(msg))) =
        timeout(Duration::from_secs(5), callee_rx.recv()).await
    {
        assert_eq!(msg.from_id, "caller1");
        assert_eq!(msg.signal_type, SignalType::Candidate);
        assert_eq!(msg.payload, "ice_candidate_1");
        info!("Callee received ICE candidate");
    } else {
        panic!("Callee didn't receive ICE");
    }

    // 测试取消订阅
    caller
        .unsubscribe_remote_status("callee1", SignalRole::Callee)
        .await
        .expect("Failed to unsubscribe");
    info!("Caller unsubscribed from callee");

    // Drop callee，caller 应该不会收到离线事件（因为已取消订阅）
    drop(callee);
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 检查 caller 的事件队列应该是空的
    match timeout(Duration::from_millis(500), caller_rx.recv()).await {
        Err(_) => info!("Caller correctly didn't receive offline event"),
        Ok(Some(event)) => panic!("Unexpected event: {:?}", event),
        Ok(None) => panic!("Channel closed unexpectedly"),
    }

    info!("Test completed successfully!");
}

#[tokio::test]
async fn test_offline_detection() {
    init_tracing();

    let (caller, mut caller_rx) =
        Signal::new("caller2".to_string(), SignalRole::Caller, test_config())
            .await
            .expect("Failed to create caller");

    let (callee, mut callee_rx) =
        Signal::new("callee2".to_string(), SignalRole::Callee, test_config())
            .await
            .expect("Failed to create callee");

    // 等待连接
    timeout(Duration::from_secs(5), caller_rx.recv()).await.ok();
    timeout(Duration::from_secs(5), callee_rx.recv()).await.ok();

    // 订阅对方状态
    caller.subscribe_remote_status("callee2", SignalRole::Callee).await.unwrap();

    // 等待上线事件
    timeout(Duration::from_secs(5), caller_rx.recv()).await.ok();

    // Callee 下线
    drop(callee);
    info!("Callee dropped");

    wait_for_remote_offline(&mut caller_rx, "callee2").await;

    info!("Caller detected callee offline");
}
