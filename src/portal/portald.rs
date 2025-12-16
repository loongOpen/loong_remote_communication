use anyhow::Result;
use clap::Parser;
use peer::portal_manager::PortalManager;
use remote_rpc_rs::{init_runtime, MqttArgs, PeerArgs};

#[derive(Parser, Debug)]
#[command(name = "portald")]
#[command(about = "WebRTC Portal - Create local portal to remote service")]
struct Args {
    /// Local client ID for signaling
    #[arg(short, long)]
    local_id: String,

    /// Remote proxy ID to connect
    #[arg(short, long)]
    remote_id: String,

    /// Local address to listen for incoming connections
    #[arg(short, long)]
    portal_addr: String,

    #[command(flatten)]
    mqtt: MqttArgs,

    #[command(flatten)]
    peer: PeerArgs,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_runtime();

    let args = Args::parse();

    let (manager, event_loop) = PortalManager::builder()
        .local_id(&args.local_id)
        .mqtt(args.mqtt.to_config()?)
        .peer(args.peer.to_config())
        .run()
        .await?;

    manager.create_portal(&args.remote_id, args.portal_addr.clone()).await?;

    tracing::info!("Portal established: {} -> {}", args.portal_addr, args.remote_id);

    tokio::select! {
        _ = event_loop => tracing::info!("PortalManager exited"),
        _ = tokio::signal::ctrl_c() => tracing::info!("Shutting down..."),
    }

    Ok(())
}
