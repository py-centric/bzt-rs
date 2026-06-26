use crate::engine::BztError;
use crate::models::config::{Configuration, DetailedRequest, HTTPRequestDefinition};
use ax_ws::{Message, WebSocket};
use axum::{
    extract::{Path, State, WebSocketUpgrade, ws as ax_ws},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::any,
    Router,
};
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};

use crate::engine::proto::bzt_mock;
use bzt_mock::mock_service_server::{MockService, MockServiceServer};
use bzt_mock::{MockRequest, MockResponse as GrpcMockResponse};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MockResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

impl Default for MockResponse {
    fn default() -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());
        Self {
            status: 200,
            body: String::new(),
            headers,
        }
    }
}

#[must_use]
pub fn generate_mock_response(req: &DetailedRequest) -> MockResponse {
    let mut response = MockResponse::default();
    let mut bodies = Vec::new();

    for assertion in &req.assert {
        if !assertion.not && assertion.subject == "body" {
            for content in &assertion.contains {
                bodies.push(content.clone());
            }
        }
    }

    if !bodies.is_empty() {
        response.body = bodies.join(" ");
    }

    response
}

async fn handle_socket(mut socket: WebSocket, response_msg: String) {
    if let Some(Ok(Message::Text(text))) = socket.recv().await {
        tracing::info!("[MOCK-WS] Received: {}", text);
        if let Err(e) = socket.send(Message::Text(response_msg.into())).await {
            tracing::error!("[MOCK-WS] Send failed: {}", e);
        }
    }
    let _ = socket.close().await;
}

use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};

#[derive(Default)]
pub struct MyMockService;

#[tonic::async_trait]
impl MockService for MyMockService {
    async fn call(
        &self,
        request: Request<MockRequest>,
    ) -> Result<Response<GrpcMockResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("[MOCK-GRPC] Received unary call for: {}", req.method);

        Ok(Response::new(GrpcMockResponse {
            message: format!("Mock response for {}", req.method),
            status: 0,
        }))
    }

    type ServerStreamStream = Pin<Box<dyn Stream<Item = Result<GrpcMockResponse, Status>> + Send>>;

    async fn server_stream(
        &self,
        request: Request<MockRequest>,
    ) -> Result<Response<Self::ServerStreamStream>, Status> {
        let req = request.into_inner();
        tracing::info!("[MOCK-GRPC] Received server stream for: {}", req.method);
        
        let s = tokio_stream::iter(vec![
            Ok(GrpcMockResponse { message: format!("Part 1 for {}", req.method), status: 0 }),
            Ok(GrpcMockResponse { message: format!("Part 2 for {}", req.method), status: 0 }),
        ]);
        
        Ok(Response::new(Box::pin(s)))
    }

    async fn client_stream(
        &self,
        request: Request<tonic::Streaming<MockRequest>>,
    ) -> Result<Response<GrpcMockResponse>, Status> {
        let mut stream = request.into_inner();
        let mut count = 0;
        while let Some(req) = stream.next().await {
            let _ = req?;
            count += 1;
        }
        tracing::info!("[MOCK-GRPC] Received client stream with {} messages", count);
        
        Ok(Response::new(GrpcMockResponse {
            message: format!("Received {} messages", count),
            status: 0,
        }))
    }

    type BidiStreamStream = Pin<Box<dyn Stream<Item = Result<GrpcMockResponse, Status>> + Send>>;

    async fn bidi_stream(
        &self,
        request: Request<tonic::Streaming<MockRequest>>,
    ) -> Result<Response<Self::BidiStreamStream>, Status> {
        let mut stream = request.into_inner();
        let output = async_stream::try_stream! {
            while let Some(req) = stream.next().await {
                let r = req?;
                tracing::info!("[MOCK-GRPC] Bidi echo: {}", r.method);
                yield GrpcMockResponse {
                    message: format!("Echo: {}", r.method),
                    status: 0,
                };
            }
        };
        Ok(Response::new(Box::pin(output)))
    }
}

async fn handle_ws_upgrade(
    ws: WebSocketUpgrade,
    Path(path): Path<String>,
    State(config): State<Arc<Configuration>>,
) -> impl IntoResponse {
    let path = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };
    let ws_path = format!("/ws{}", path);

    for scenario in config.scenarios.values() {
        for req_def in &scenario.requests {
            if let HTTPRequestDefinition::Detailed(d) = req_def {
                let is_ws = (d.url == path || d.url == ws_path)
                    && d.protocol.as_deref().is_some_and(|p| p == "websocket" || p == "ws");
                if is_ws
                {
                    let mock_res = generate_mock_response(d);
                    tracing::info!("[MOCK-WS] Upgrading: {}", d.url);
                    return ws.on_upgrade(move |socket| handle_socket(socket, mock_res.body));
                }
            }
        }
    }

    StatusCode::NOT_FOUND.into_response()
}

async fn handle_mock_request(
    method: Method,
    Path(path): Path<String>,
    State(config): State<Arc<Configuration>>,
) -> impl IntoResponse {
    let path = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };

    tracing::info!("[MOCK] HTTP Request: {} {}", method, path);

    for scenario in config.scenarios.values() {
        for req_def in &scenario.requests {
            match req_def {
                HTTPRequestDefinition::Simple(url) => {
                    if url == &path && method == Method::GET {
                        return (StatusCode::OK, "OK").into_response();
                    }
                }
                HTTPRequestDefinition::Detailed(d) => {
                    let req_method = d
                        .method
                        .as_deref()
                        .unwrap_or("GET")
                        .parse::<Method>()
                        .unwrap_or(Method::GET);
                    if d.url == path && method == req_method {
                        let mock_res = generate_mock_response(d);
                        return (
                            StatusCode::from_u16(mock_res.status).unwrap_or(StatusCode::OK),
                            mock_res.body,
                        )
                            .into_response();
                    }
                }
            }
        }
    }

    tracing::info!("[MOCK] Not Found: {} {}", method, path);
    StatusCode::NOT_FOUND.into_response()
}

pub struct MockServerAddresses {
    pub http_addr: SocketAddr,
    pub grpc_addr: SocketAddr,
}

pub async fn start_mock_server(config: Configuration) -> Result<MockServerAddresses, BztError> {
    let shared_config = Arc::new(config);
    let app = Router::new()
        .route("/ws/{*path}", axum::routing::get(handle_ws_upgrade))
        .route("/{*path}", any(handle_mock_request))
        .with_state(shared_config);

    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    println!("Mock server started at http://{local_addr}");

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("Mock HTTP server error: {}", e);
        }
    });

    // Start gRPC server on a separate port
    let grpc_addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let grpc_listener = tokio::net::TcpListener::bind(grpc_addr).await?;
    let local_grpc_addr = grpc_listener.local_addr()?;

    println!("gRPC Mock server started at {}", local_grpc_addr);

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(bzt_mock::FILE_DESCRIPTOR_SET)
        .build_v1()
        .map_err(|e| crate::engine::BztError::Internal(e.to_string()))?;

    tokio::spawn(async move {
        if let Err(e) = Server::builder()
            .add_service(MockServiceServer::new(MyMockService))
            .add_service(reflection_service)
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(grpc_listener))
            .await
        {
            tracing::error!("Mock gRPC server error: {}", e);
        }
    });

    Ok(MockServerAddresses {
        http_addr: local_addr,
        grpc_addr: local_grpc_addr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::AssertionDefinition;

    #[test]
    fn test_assertion_to_body_mapping() {
        let req = DetailedRequest {
            url: "/api/test".to_string(),
            assert: vec![AssertionDefinition {
                contains: vec!["Success".to_string(), "token: 123".to_string()],
                subject: "body".to_string(),
                regexp: false,
                not: false,
            }],
            ..Default::default()
        };

        let response = generate_mock_response(&req);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "Success token: 123");
    }
}
