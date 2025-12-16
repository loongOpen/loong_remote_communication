use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{delete, post},
    Json, Router,
};
use clap::Parser;
use remote_rpc_rs::portal_hub::{CreatePortalRequest, PortalHubService, PortalType};
use remote_rpc_rs::{init_runtime, MqttArgs, PeerArgs};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "portal_hub_rest")]
#[command(about = "REST API service for managing WebRTC portals")]
struct Args {
    /// Default user ID for requests without user_id
    #[arg(short, long, default_value = "")]
    user_id: String,

    /// Portal hub's HTTP server listen address
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    listen: String,

    #[command(flatten)]
    mqtt: MqttArgs,

    #[command(flatten)]
    peer: PeerArgs,
}

#[derive(Debug, Deserialize)]
struct PortalRequest {
    user_id: Option<String>,
    robot_id: String,
    service_name: String,
    #[serde(default)]
    portal_type: String, // "inet" or "unix"
    inet_port: Option<String>,
    unix_file: Option<String>,
}

#[derive(Debug, Serialize)]
struct PortalResponse {
    uri: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

impl From<PortalRequest> for CreatePortalRequest {
    fn from(req: PortalRequest) -> Self {
        CreatePortalRequest {
            user_id: req.user_id.unwrap_or_default(),
            robot_id: req.robot_id,
            service_name: req.service_name,
            portal_type: if req.portal_type == "unix" {
                PortalType::Unix
            } else {
                PortalType::Inet
            },
            inet_port: req.inet_port,
            unix_file: req.unix_file,
        }
    }
}

async fn create_portal(
    State(service): State<Arc<PortalHubService>>,
    Json(req): Json<PortalRequest>,
) -> Result<Json<PortalResponse>, (StatusCode, Json<ErrorResponse>)> {
    match service.create_portal(req.into()).await {
        Ok(resp) => Ok(Json(PortalResponse { uri: resp.uri })),
        Err(e) => {
            Err((StatusCode::SERVICE_UNAVAILABLE, Json(ErrorResponse { error: e.to_string() })))
        }
    }
}

async fn destroy_portal(
    State(service): State<Arc<PortalHubService>>,
    Json(req): Json<PortalRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match service.destroy_portal(req.into()).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_runtime();

    let args = Args::parse();
    let mq_cfg = args.mqtt.to_config()?;
    let peer_cfg = args.peer.to_config();

    let service = Arc::new(PortalHubService::new(args.user_id, mq_cfg, peer_cfg));

    let app = Router::new()
        .route("/portal", post(create_portal))
        .route("/portal", delete(destroy_portal))
        .with_state(service);

    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    info!("PortalHub REST server listening on {}", args.listen);

    axum::serve(listener, app).await?;

    Ok(())
}
