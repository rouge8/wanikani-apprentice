use axum::{
    body::Body,
    extract::Path,
    http::{header, HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, get_service},
    Extension, Router,
};
use config::Config;
use constants::BS_PRIMARY_COLOR;
use db::Database;
use dotenvy::dotenv;
use std::io;
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tower_http::{catch_panic::CatchPanicLayer, services::ServeDir};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use wanikani::WaniKaniAPIClient;

mod config;
mod constants;
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

/// Mirror the WaniKani radical SVGs, replacing the `stroke` color with our primary color.
async fn radical_svg(Path(path): Path<String>, state: Extension<Arc<State>>) -> impl IntoResponse {
    #[cfg(not(test))]
    let base_url = "https://files.wanikani.com";
    #[cfg(test)]
    let base_url = mockito::server_url();

    let url = format!("{base_url}/{path}");
    info!(url, "downloading SVG");
    let resp = state
        .http_client
        .get(url)
        .send()
        .await
        .expect("failed to request SVG");
    resp.error_for_status_ref().expect("failed to download SVG");
    let svg = resp
        .text()
        .await
        .expect("failed to decode SVG")
        .replace("stroke:#000", &format!("stroke:{}", *BS_PRIMARY_COLOR));

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/svg+xml".parse().unwrap());

    (headers, svg)
}

async fn test_500() {
    let _ = 1 / 0;
}

struct State {
    http_client: reqwest::Client,
}

fn create_app(http_client: reqwest::Client) -> Router {
    let state = Arc::new(State { http_client });

    Router::new()
        .route("/test-500", get(test_500))
        .route("/radical-svg/:path", get(radical_svg))
        .nest(
            "/static",
            get_service(ServeDir::new("static")).handle_error(handle_static_files_error),
        )
        .layer(
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
                .layer(middleware::from_fn(lb_heartbeat_middleware))
                .layer(Extension(state)),
        )
}

async fn handle_static_files_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}

#[tokio::main]
async fn main() -> reqwest::Result<()> {
    dotenv().ok();

    let subscriber = FmtSubscriber::builder().finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let http_client = reqwest::Client::new();

    let config = Config::from_env();

    let api = WaniKaniAPIClient::new(&config.wanikani_api_key, &http_client);

    // Load the WaniKani data
    let mut db = Database::new();
    db.populate(&api).await?;

    // Build the application
    let app = create_app(http_client);

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
    use mockito::mock;
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};
    use tower::ServiceExt;

    #[fixture]
    fn app() -> Router {
        create_app(reqwest::Client::new())
    }

    #[rstest]
    #[tokio::test]
    async fn test_radical_svg(app: Router) {
        let _m = mock("GET", "/foo")
            .with_status(200)
            .with_body("foo bar stroke:#000 other:#000")
            .create();

        let resp = app
            .oneshot(
                Request::get("/radical-svg/foo")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
        assert_eq!(
            body,
            format!("foo bar stroke:{} other:#000", *BS_PRIMARY_COLOR)
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_500(app: Router) {
        let resp = app
            .oneshot(Request::get("/test-500").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[rstest]
    #[tokio::test]
    async fn test_lb_heartbeat(app: Router) {
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
}
