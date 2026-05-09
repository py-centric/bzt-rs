use crate::engine::BztError;
use crate::models::config::{Configuration, DetailedRequest, HTTPRequestDefinition};
use axum::{
    Router,
    extract::{Path, State},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::any,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

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

    tracing::debug!("Mock request: {} {}", method, path);

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

    StatusCode::NOT_FOUND.into_response()
}

pub async fn start_mock_server(config: Configuration) -> Result<SocketAddr, BztError> {
    let shared_config = Arc::new(config);
    let app = Router::new()
        .route("/{*path}", any(handle_mock_request))
        .with_state(shared_config);

    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    println!("Mock server started at http://{local_addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    Ok(local_addr)
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
