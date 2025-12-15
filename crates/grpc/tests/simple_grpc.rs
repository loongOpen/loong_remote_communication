use grpc::generated::hello::{
    greeter_client::GreeterClient,
    greeter_server::{Greeter, GreeterServer},
    HelloReply, HelloRequest,
};

use tokio::time::Duration;
use tonic::{transport::Server, Request, Response, Status};
use tracing::info;

#[derive(Default)]
struct MyGreeter;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let name = request.into_inner().name;
        info!("Server received request from: {}", name);
        Ok(Response::new(HelloReply { message: format!("Hello, {}!", name) }))
    }
}

#[tokio::test]
async fn test_grpc_server_client() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    let addr = "127.0.0.1:50051".parse().unwrap();
    tokio::spawn(async move {
        info!("gRPC server listening on {}", addr);
        Server::builder()
            .add_service(GreeterServer::new(MyGreeter::default()))
            .serve(addr)
            .await
            .unwrap();
    });

    // 等待服务启动
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 创建客户端
    let mut client =
        GreeterClient::connect("http://127.0.0.1:50051").await.expect("Failed to connect");

    // 发送请求
    let request = tonic::Request::new(HelloRequest { name: "Rust".into() });

    let response = client.say_hello(request).await.expect("RPC failed");

    info!("Client got response: {:?}", response.get_ref().message);
    assert_eq!(response.get_ref().message, "Hello, Rust!");
}
