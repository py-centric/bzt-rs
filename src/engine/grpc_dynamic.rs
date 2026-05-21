use crate::engine::BztError;
use prost_reflect::{DescriptorPool, DynamicMessage, MethodDescriptor};
use tonic::client::Grpc;
use tonic::transport::Channel;
use tonic_reflection::pb::v1::{ServerReflectionRequest, server_reflection_client::ServerReflectionClient};
use tonic_reflection::pb::v1::server_reflection_request::MessageRequest;
use tokio_stream::StreamExt;
use std::collections::HashSet;

pub struct DynamicGrpcClient {
    pool: DescriptorPool,
}

impl DynamicGrpcClient {
    pub async fn discover(host: &str) -> Result<Self, BztError> {
        let channel = Channel::from_shared(host.to_string())
            .map_err(|e| BztError::Network {
                host: host.to_string(),
                operation: "Connect for reflection".to_string(),
                details: e.to_string(),
            })?
            .connect()
            .await
            .map_err(|e| BztError::Network {
                host: host.to_string(),
                operation: "Establish reflection connection".to_string(),
                details: e.to_string(),
            })?;

        let mut client = ServerReflectionClient::new(channel);
        let mut pool = DescriptorPool::new();
        let mut discovered_files = HashSet::new();

        let request = ServerReflectionRequest {
            host: host.to_string(),
            message_request: Some(MessageRequest::ListServices("*".to_string())),
        };

        let mut stream = client.server_reflection_info(tokio_stream::iter(vec![request]))
            .await
            .map_err(|e| BztError::Internal(format!("gRPC Reflection failed: {}", e)))?
            .into_inner();

        let mut services = Vec::new();
        if let Some(Ok(response)) = stream.next().await {
            if let Some(tonic_reflection::pb::v1::server_reflection_response::MessageResponse::ListServicesResponse(list)) = response.message_response {
                for svc in list.service {
                    services.push(svc.name);
                }
            }
        }

        for svc_name in services {
            let req = ServerReflectionRequest {
                host: host.to_string(),
                message_request: Some(MessageRequest::FileContainingSymbol(svc_name)),
            };
            
            let mut s = client.server_reflection_info(tokio_stream::iter(vec![req]))
                .await
                .map_err(|e| BztError::Internal(format!("gRPC FileDescriptor fetch failed: {}", e)))?
                .into_inner();

            if let Some(Ok(resp)) = s.next().await {
                if let Some(tonic_reflection::pb::v1::server_reflection_response::MessageResponse::FileDescriptorResponse(fd_resp)) = resp.message_response {
                    for raw_fd in fd_resp.file_descriptor_proto {
                        if discovered_files.insert(raw_fd.clone()) {
                            let _ = pool.add_file_descriptor_proto(
                                prost_types::FileDescriptorProto::decode(&raw_fd[..]).unwrap()
                            );
                        }
                    }
                }
            }
        }
        
        Ok(Self { pool })
    }

    pub fn find_method(&self, full_name: &str) -> Result<MethodDescriptor, BztError> {
        for service in self.pool.services() {
            if let Some(method) = service.methods().find(|m| m.full_name() == full_name) {
                return Ok(method);
            }
        }
        Err(BztError::Validation {
            field: "method_name".to_string(),
            reason: format!("Method not found via reflection: {}", full_name),
        })
    }

    pub async fn call_unary(
        &self,
        channel: Channel,
        method: MethodDescriptor,
        payload_json: &str,
    ) -> Result<String, BztError> {
        let input_msg = self.json_to_dynamic(method.input(), payload_json)?;
        let mut grpc = Grpc::new(channel);
        let path = format!("/{}/{}", method.parent_service().full_name(), method.name());
        
        let client = grpc.unary(
            tonic::Request::new(input_msg), 
            path.parse().unwrap(), 
            DynamicCodec::new(method.output())
        );
        
        let response = client.await.map_err(|e| BztError::Network {
            host: "gRPC Call".to_string(),
            operation: "unary".to_string(),
            details: e.to_string(),
        })?;
        
        self.dynamic_to_json(&response.into_inner())
    }

    pub async fn call_server_streaming(
        &self,
        channel: Channel,
        method: MethodDescriptor,
        payload_json: &str,
    ) -> Result<Box<dyn tokio_stream::Stream<Item = Result<String, BztError>> + Unpin + Send>, BztError> {
        let input_msg = self.json_to_dynamic(method.input(), payload_json)?;
        let mut grpc = Grpc::new(channel);
        let path = format!("/{}/{}", method.parent_service().full_name(), method.name());
        
        let stream = grpc.server_streaming(
            tonic::Request::new(input_msg),
            path.parse().unwrap(),
            DynamicCodec::new(method.output())
        ).await.map_err(|e| BztError::Network {
            host: "gRPC Call".to_string(),
            operation: "server_streaming".to_string(),
            details: e.to_string(),
        })?.into_inner();

        Ok(Box::new(stream.map(move |res| {
            match res {
                Ok(msg) => {
                    serde_json::to_string(&msg).map_err(|e| BztError::Internal(e.to_string()))
                }
                Err(e) => Err(BztError::Network {
                    host: "gRPC Stream".to_string(),
                    operation: "recv".to_string(),
                    details: e.to_string(),
                })
            }
        })))
    }

    pub async fn call_client_streaming(
        &self,
        channel: Channel,
        method: MethodDescriptor,
        payloads: Vec<String>,
    ) -> Result<String, BztError> {
        let mut grpc = Grpc::new(channel);
        let path = format!("/{}/{}", method.parent_service().full_name(), method.name());
        
        let mut dynamic_payloads = Vec::new();
        for p in payloads {
            dynamic_payloads.push(self.json_to_dynamic(method.input(), &p)?);
        }
        let stream = tokio_stream::iter(dynamic_payloads);

        let response = grpc.client_streaming(
            tonic::Request::new(stream),
            path.parse().unwrap(),
            DynamicCodec::new(method.output())
        ).await.map_err(|e| BztError::Network {
            host: "gRPC Call".to_string(),
            operation: "client_streaming".to_string(),
            details: e.to_string(),
        })?;

        self.dynamic_to_json(&response.into_inner())
    }

    pub async fn call_bidi_streaming(
        &self,
        channel: Channel,
        method: MethodDescriptor,
        payloads: Vec<String>,
    ) -> Result<Box<dyn tokio_stream::Stream<Item = Result<String, BztError>> + Unpin + Send>, BztError> {
        let mut grpc = Grpc::new(channel);
        let path = format!("/{}/{}", method.parent_service().full_name(), method.name());
        
        let mut dynamic_payloads = Vec::new();
        for p in payloads {
            dynamic_payloads.push(self.json_to_dynamic(method.input(), &p)?);
        }
        let stream = tokio_stream::iter(dynamic_payloads);

        let response_stream = grpc.streaming(
            tonic::Request::new(stream),
            path.parse().unwrap(),
            DynamicCodec::new(method.output())
        ).await.map_err(|e| BztError::Network {
            host: "gRPC Call".to_string(),
            operation: "bidi_streaming".to_string(),
            details: e.to_string(),
        })?.into_inner();

        Ok(Box::new(response_stream.map(|res| {
            match res {
                Ok(msg) => {
                    serde_json::to_string(&msg).map_err(|e| BztError::Internal(e.to_string()))
                }
                Err(e) => Err(BztError::Network {
                    host: "gRPC Stream".to_string(),
                    operation: "recv".to_string(),
                    details: e.to_string(),
                })
            }
        })))
    }

    fn json_to_dynamic(&self, desc: prost_reflect::MessageDescriptor, json: &str) -> Result<DynamicMessage, BztError> {
        let mut deserializer = serde_json::Deserializer::from_str(json);
        DynamicMessage::deserialize(desc, &mut deserializer)
            .map_err(|e| BztError::Validation {
                field: "body".to_string(),
                reason: format!("Failed to parse gRPC JSON payload: {}", e),
            })
    }

    fn dynamic_to_json(&self, msg: &DynamicMessage) -> Result<String, BztError> {
        serde_json::to_string(msg).map_err(|e| BztError::Internal(e.to_string()))
    }
}

struct DynamicCodec {
    output_desc: prost_reflect::MessageDescriptor,
}

impl DynamicCodec {
    fn new(output_desc: prost_reflect::MessageDescriptor) -> Self {
        Self { output_desc }
    }
}

impl tonic::codec::Codec for DynamicCodec {
    type Encode = DynamicMessage;
    type Decode = DynamicMessage;
    type Encoder = DynamicEncoder;
    type Decoder = DynamicDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        DynamicEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        DynamicDecoder { desc: self.output_desc.clone() }
    }
}

struct DynamicEncoder;
impl tonic::codec::Encoder for DynamicEncoder {
    type Item = DynamicMessage;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut tonic::codec::EncodeBuf<'_>) -> Result<(), Self::Error> {
        use prost::Message;
        item.encode(dst).map_err(|e| Status::internal(e.to_string()))
    }
}

struct DynamicDecoder {
    desc: prost_reflect::MessageDescriptor,
}
impl tonic::codec::Decoder for DynamicDecoder {
    type Item = DynamicMessage;
    type Error = Status;

    fn decode(&mut self, src: &mut tonic::codec::DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        use prost::Message;
        let mut msg = DynamicMessage::new(self.desc.clone());
        msg.merge(src).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Some(msg))
    }
}

use tonic::Status;
use prost::Message;
