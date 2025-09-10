use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};
use tracing::*;

pub async fn start(port: u16) {
    let addr = format!("0.0.0.0:{}", port).parse().unwrap();
    info!("gRPC listening on {}", addr);
    Server::builder()
        .add_service(GreeterServer::new(GreeterSvc))
        .serve(addr)
        .await
        .unwrap();
}

tonic::include_proto!("mockforge.greeter");

// Re-export the generated types for easier access
pub use greeter_server::GreeterServer;

#[derive(Default)]
pub struct GreeterSvc;

#[tonic::async_trait]
impl greeter_server::Greeter for GreeterSvc {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let name = request.into_inner().name;
        let reply = HelloReply {
            message: format!("Hello, {}", name),
        };
        Ok(Response::new(reply))
    }

    type SayHelloStreamStream = ReceiverStream<Result<HelloReply, Status>>;
    async fn say_hello_stream(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::SayHelloStreamStream>, Status> {
        let name = request.into_inner().name;
        let (tx, rx) = mpsc::channel(4);
        tokio::spawn(async move {
            for i in 0..3 {
                let _ = tx
                    .send(Ok(HelloReply {
                        message: format!("hi {} #{}", name, i),
                    }))
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn say_hello_client_stream(
        &self,
        request: Request<tonic::Streaming<HelloRequest>>,
    ) -> Result<Response<HelloReply>, Status> {
        let mut s = request.into_inner();
        let mut names = vec![];
        while let Ok(Some(m)) = s.message().await {
            names.push(m.name);
        }
        Ok(Response::new(HelloReply {
            message: format!("Hello, {}", names.join(", ")),
        }))
    }

    type ChatStream = ReceiverStream<Result<HelloReply, Status>>;
    async fn chat(
        &self,
        request: Request<tonic::Streaming<HelloRequest>>,
    ) -> Result<Response<Self::ChatStream>, Status> {
        let mut s = request.into_inner();
        let (tx, rx) = mpsc::channel(8);
        tokio::spawn(async move {
            while let Ok(Some(m)) = s.message().await {
                let msg = HelloReply {
                    message: format!("you said: {}", m.name),
                };
                let _ = tx.send(Ok(msg)).await;
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
