use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
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

async fn lb_heartbeat_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();

    if path == "/__lbheartbeat__" {
        Ok(Response::builder()
            .body(Body::from("OK"))
            .unwrap()
            .into_response())
    } else {
        Ok(next.run(req).await)
    }
}

async fn test_500() {
    let _ = 1 / 0;
}

fn create_app() -> Router {
    Router::new().route("/test-500", get(test_500)).layer(
        ServiceBuilder::new()
            .layer(CatchPanicLayer::new())
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_request(DefaultOnRequest::new().level(Level::INFO))
                    .on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .latency_unit(tower_http::LatencyUnit::Seconds),
                    ),
            )
            .layer(middleware::from_fn(lb_heartbeat_middleware)),
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
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use pretty_assertions::assert_eq;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_lb_heartbeat() {
        let app = create_app();

        let resp = app
            .oneshot(
                Request::get("/__lbheartbeat__")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
        assert_eq!(body, "OK");
    }

    #[tokio::test]
    async fn test_500() {
        let app = create_app();

        let resp = app
            .oneshot(Request::get("/test-500").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
