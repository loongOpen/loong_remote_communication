use anyhow::Result;
use clap::Parser;
use grpc::{Config, PortalLauncher, PortalLauncherServer, PortalType as GrpcPortalType, SockAddr};
use remote_rpc_rs::portal_hub::{CreatePortalRequest, PortalHubService, PortalType};
use remote_rpc_rs::{init_runtime, MqttArgs, PeerArgs};
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "portal_hub_grpc")]
#[command(about = "gRPC service for managing WebRTC portals")]
struct Args {
    /// Default user ID for requests without user_id
    #[arg(short, long, default_value = "")]
    user_id: String,

    /// Portal hub's gRPC server listen address
    #[arg(short, long, default_value = "[::1]:50051")]
    listen: String,

    #[command(flatten)]
    mqtt: MqttArgs,

    #[command(flatten)]
    peer: PeerArgs,
}

struct GrpcService {
    inner: Arc<PortalHubService>,
}

impl GrpcService {
    fn convert_request(config: Config) -> CreatePortalRequest {
        CreatePortalRequest {
            user_id: config.user_id,
            robot_id: config.robot_id,
            service_name: config.service_name,
            portal_type: if config.r#type == GrpcPortalType::Unix as i32 {
                PortalType::Unix
            } else {
                PortalType::Inet
            },
            inet_port: Some(config.inet_port),
            unix_file: Some(config.unix_file),
        }
    }
}

#[tonic::async_trait]
impl PortalLauncher for GrpcService {
    async fn create_portal(&self, request: Request<Config>) -> Result<Response<SockAddr>, Status> {
        let req = Self::convert_request(request.into_inner());

        match self.inner.create_portal(req).await {
            Ok(resp) => Ok(Response::new(SockAddr { uri: resp.uri })),
            Err(e) => Err(Status::unavailable(e.to_string())),
        }
    }

    async fn destroy_portal(&self, request: Request<Config>) -> Result<Response<()>, Status> {
        let req = Self::convert_request(request.into_inner());

        match self.inner.destroy_portal(req).await {
            Ok(()) => Ok(Response::new(())),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_runtime();

    let args = Args::parse();
    let mq_cfg = args.mqtt.to_config()?;
    let peer_cfg = args.peer.to_config();

    let addr = args.listen.parse()?;
    let service = Arc::new(PortalHubService::new(args.user_id, mq_cfg, peer_cfg));

    info!("PortalHub gRPC server listening on {}", addr);

    Server::builder()
        .add_service(PortalLauncherServer::new(GrpcService { inner: service }))
        .serve(addr)
        .await?;

    Ok(())
}
