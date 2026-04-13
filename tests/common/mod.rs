use axum::{Json, Router, routing::get, routing::post};
use serde_json::{Value, json};
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub async fn start_mock_server() -> SocketAddr {
    let app = Router::new()
        .route("/", get(|| async { "Hello, world!" }))
        .route("/high", get(|| async { "High traffic" }))
        .route("/low", get(|| async { "Low traffic" }))
        .route(
            "/api",
            post(|Json(body): Json<Value>| async move { Json(json!({ "received": body })) }),
        )
        .route(
            "/user/{id}",
            get(
                |axum::extract::Path(id): axum::extract::Path<String>| async move {
                    Json(json!({ "hello": id }))
                },
            ),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    addr
}
