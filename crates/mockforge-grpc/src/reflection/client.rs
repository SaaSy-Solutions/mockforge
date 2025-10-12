//! gRPC reflection client for dynamically discovering services and methods.

use prost_reflect::{prost::Message, prost_types, DescriptorPool, ServiceDescriptor};
use tonic::{
    transport::{Channel, Endpoint},
    Status,
};
use tonic_reflection::pb::v1::{
    server_reflection_client::ServerReflectionClient, server_reflection_response::MessageResponse,
    ServerReflectionRequest,
};
use tracing::{debug, error, trace};

/// A client that uses gRPC reflection to discover services and methods
pub struct ReflectionClient {
    /// The gRPC channel to the target server
    channel: Channel,
    /// The descriptor pool containing all discovered services
    pool: DescriptorPool,
}

impl ReflectionClient {
    /// Create a new reflection client
    pub async fn new(endpoint: Endpoint) -> Result<Self, Status> {
        let channel = endpoint.connect().await.map_err(|e| {
            error!("Failed to connect to endpoint: {}", e);
            Status::unavailable(format!("Failed to connect to endpoint: {}", e))
        })?;

        let mut pool = DescriptorPool::new();

        // Create a reflection client
        let mut client = ServerReflectionClient::new(channel.clone());

        // Get the list of services
        let request = tonic::Request::new(futures_util::stream::iter(vec![
            ServerReflectionRequest {
                host: "".to_string(),
                message_request: Some(
                    tonic_reflection::pb::v1::server_reflection_request::MessageRequest::ListServices(
                        "*".to_string(),
                    ),
                ),
            }
        ]));

        let mut service_names = Vec::new();

        match client.server_reflection_info(request).await {
            Ok(response) => {
                let mut stream = response.into_inner();
                while let Some(reply) = stream.message().await.map_err(|e| {
                    error!("Failed to read reflection response: {}", e);
                    Status::internal(format!("Failed to read reflection response: {}", e))
                })? {
                    if let Some(MessageResponse::ListServicesResponse(services)) =
                        reply.message_response
                    {
                        trace!("Found {} services", services.service.len());
                        for service in services.service {
                            debug!("Found service: {}", service.name);
                            service_names.push(service.name.clone());
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to get service list: {}", e);
                return Err(Status::internal(format!("Failed to get service list: {}", e)));
            }
        }

        // For each service, get its file descriptor
        for service_name in &service_names {
            Self::get_file_descriptor_for_service(&mut client, &mut pool, service_name).await?;
        }

        debug!(
            "Created reflection client for endpoint with {} services",
            pool.services().count()
        );

        Ok(Self { channel, pool })
    }

    /// Get file descriptor for a service
    async fn get_file_descriptor_for_service(
        client: &mut ServerReflectionClient<Channel>,
        pool: &mut DescriptorPool,
        service_name: &str,
    ) -> Result<(), Status> {
        trace!("Getting file descriptor for service: {}", service_name);

        let request = tonic::Request::new(futures_util::stream::iter(vec![
            ServerReflectionRequest {
                host: "".to_string(),
                message_request: Some(
                    tonic_reflection::pb::v1::server_reflection_request::MessageRequest::FileContainingSymbol(
                        service_name.to_string(),
                    ),
                ),
            }
        ]));

        match client.server_reflection_info(request).await {
            Ok(response) => {
                let mut stream = response.into_inner();
                while let Some(reply) = stream.message().await.map_err(|e| {
                    error!("Failed to read reflection response: {}", e);
                    Status::internal(format!("Failed to read reflection response: {}", e))
                })? {
                    if let Some(MessageResponse::FileDescriptorResponse(descriptor_response)) =
                        reply.message_response
                    {
                        trace!(
                            "Found {} file descriptors for service {}",
                            descriptor_response.file_descriptor_proto.len(),
                            service_name
                        );
                        for file_descriptor_proto in descriptor_response.file_descriptor_proto {
                            match prost_types::FileDescriptorProto::decode(&*file_descriptor_proto)
                            {
                                Ok(file_descriptor) => {
                                    if let Err(e) = pool.add_file_descriptor_proto(file_descriptor)
                                    {
                                        error!(
                                            "Failed to register file descriptor for service {}: {}",
                                            service_name, e
                                        );
                                        return Err(Status::internal(format!(
                                            "Failed to register file descriptor for service {}: {}",
                                            service_name, e
                                        )));
                                    } else {
                                        debug!(
                                            "Registered file descriptor for service: {}",
                                            service_name
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to decode file descriptor for service {}: {}",
                                        service_name, e
                                    );
                                    return Err(Status::data_loss(format!(
                                        "Failed to decode file descriptor for service {}: {}",
                                        service_name, e
                                    )));
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to get file descriptor for service {}: {}", service_name, e);
                return Err(Status::internal(format!(
                    "Failed to get file descriptor for service {}: {}",
                    service_name, e
                )));
            }
        }

        Ok(())
    }

    /// Get a service descriptor by name
    pub fn get_service(&self, service_name: &str) -> Option<ServiceDescriptor> {
        self.pool.get_service_by_name(service_name)
    }

    /// Get the underlying channel
    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }

    /// Get a reference to the descriptor pool
    pub fn pool(&self) -> &DescriptorPool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {}
}
