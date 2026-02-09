use axum::Router;
use axum::routing::{get, post};

mod fingerprint;
mod routes;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/fingerprint", post(routes::fingerprint))
        .route("/health", get(|| async { "healthy" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}