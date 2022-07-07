use axum::{routing::get, Router};
use config::Config;
use db::Database;
use dotenvy::dotenv;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use wanikani::WaniKaniAPIClient;

mod config;
mod db;
mod models;
mod wanikani;

async fn test_500() {
    let _ = 1 / 0;
}

fn create_app() -> Router {
    Router::new().route("/test-500", get(test_500)).layer(
        ServiceBuilder::new().layer(CatchPanicLayer::new()).layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(tower_http::LatencyUnit::Seconds),
                ),
        ),
    )
}

#[tokio::main]
async fn main() -> reqwest::Result<()> {
    dotenv().ok();

    let subscriber = FmtSubscriber::builder().finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let config = Config::from_env();

    let api = WaniKaniAPIClient::new(&config.wanikani_api_key, reqwest::Client::new());

    // Load the WaniKani data
    let mut db = Database::new();
    db.populate(&api).await?;

    // Build the application
    let app = create_app();

    // Serve the app
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on http://{addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test_helper::TestClient;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_500() {
        let app = create_app();
        let test_client = TestClient::new(app);

        let resp = test_client.get("/test-500").send().await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
