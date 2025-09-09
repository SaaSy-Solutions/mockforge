use tonic::{transport::Server, Request, Response, Status};
use tracing::*;

pub async fn start(port: u16) {
    let addr = format!("0.0.0.0:{}", port).parse().unwrap();
    info!("gRPC listening on {}", addr);
    Server::builder()
        .add_service(GreeterServer::new(GreeterSvc))
        .serve(addr).await.unwrap();
}

tonic::include_proto!("mockforge.greeter");

// Re-export the generated types for easier access
pub use greeter_server::GreeterServer;

#[derive(Default)]
pub struct GreeterSvc;

#[tonic::async_trait]
impl greeter_server::Greeter for GreeterSvc {
    async fn say_hello(&self, request: Request<HelloRequest>) -> Result<Response<HelloReply>, Status> {
        let name = request.into_inner().name;
        let reply = HelloReply { message: format!("Hello, {}", name) };
        Ok(Response::new(reply))
    }

    type SayHelloStreamStream = tokio_stream::wrappers::ReceiverStream<Result<HelloReply, Status>>;
    async fn say_hello_stream(&self, request: Request<HelloRequest>) -> Result<Response<Self::SayHelloStreamStream>, Status> {
        let name = request.into_inner().name;
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            for i in 0..3 {
                let _ = tx.send(Ok(HelloReply { message: format!("hi {} #{}", name, i) })).await;
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
}
