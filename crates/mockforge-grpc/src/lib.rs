#[cfg(feature = "data-faker")]
use mockforge_data::provider::register_core_faker_provider;
use mockforge_core::{latency::LatencyInjector, LatencyProfile};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};
use tracing::*;

pub async fn start(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    start_with_latency(port, None).await
}

pub async fn start_with_latency(
    port: u16,
    latency_profile: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "data-faker")]
    register_core_faker_provider();

    let latency_injector = latency_profile.map(|profile| {
        LatencyInjector::new(profile, Default::default())
    });

    // Use shared server utilities for consistent address creation
    let addr = mockforge_core::wildcard_socket_addr(port);
    info!("gRPC listening on {}", addr);
    Server::builder()
        .add_service(GreeterServer::new(GreeterSvc::new(latency_injector)))
        .serve(addr)
        .await?;
    Ok(())
}

tonic::include_proto!("mockforge.greeter");

// Re-export the generated types for easier access
pub use greeter_server::GreeterServer;

pub struct GreeterSvc {
    latency_injector: Option<LatencyInjector>,
}

impl GreeterSvc {
    pub fn new(latency_injector: Option<LatencyInjector>) -> Self {
        Self { latency_injector }
    }
}

impl Default for GreeterSvc {
    fn default() -> Self {
        Self::new(None)
    }
}

#[tonic::async_trait]
impl greeter_server::Greeter for GreeterSvc {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let name = request.into_inner().name;

        // Inject latency before responding
        if let Some(ref injector) = self.latency_injector {
            let _ = injector.inject_latency(&[]).await;
        }

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
        let latency_injector = self.latency_injector.clone();
        let (tx, rx) = mpsc::channel(4);

        tokio::spawn(async move {
            for i in 0..3 {
                // Inject latency before sending each message
                if let Some(injector) = &latency_injector {
                    let _ = injector.inject_latency(&[]).await;
                }

                let _ = tx
                    .send(Ok(HelloReply {
                        message: format!("hi {} #{}", name, i),
                    }))
                    .await;
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

        // Inject latency before responding
        if let Some(ref injector) = self.latency_injector {
            let _ = injector.inject_latency(&[]).await;
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
        let latency_injector = self.latency_injector.clone();

        tokio::spawn(async move {
            while let Ok(Some(m)) = s.message().await {
                // Inject latency before responding to each message
                if let Some(injector) = &latency_injector {
                    let _ = injector.inject_latency(&[]).await;
                }

                let msg = HelloReply {
                    message: format!("you said: {}", m.name),
                };
                let _ = tx.send(Ok(msg)).await;
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
