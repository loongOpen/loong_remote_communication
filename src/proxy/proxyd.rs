use anyhow::Result;
use clap::Parser;
use peer::proxy_manager::ProxyManager;
use remote_rpc_rs::{init_runtime, MqttArgs, PeerArgs};

#[derive(Parser, Debug)]
#[command(name = "proxyd")]
#[command(about = "WebRTC Proxy - Proxy remote connections to local service")]
struct Args {
    /// Local service ID for signaling
    #[arg(short, long)]
    local_id: String,

    /// Target service address to proxy (e.g., 127.0.0.1:9000 or unix:///path/to/socket)
    #[arg(short, long)]
    proxy_addr: String,

    #[command(flatten)]
    mqtt: MqttArgs,

    #[command(flatten)]
    peer: PeerArgs,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_runtime();

    let args = Args::parse();

    let (_manager, event_loop) = ProxyManager::builder()
        .local_id(&args.local_id)
        .mqtt(args.mqtt.to_config()?)
        .peer(args.peer.to_config())
        .target_addr(&args.proxy_addr)
        .run()
        .await?;

    tracing::info!("Proxyd started: {} -> {}", args.local_id, args.proxy_addr);

    tokio::select! {
        _ = event_loop => tracing::info!("ProxyManager exited"),
        _ = tokio::signal::ctrl_c() => tracing::info!("Shutting down..."),
    }

    Ok(())
}
