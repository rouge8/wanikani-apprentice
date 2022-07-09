use axum::{
    async_trait,
    extract::{FromRequest, Path, RequestParts},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::{get, get_service, post},
    Extension, Form, Router,
};
use axum_extra::extract::cookie::{Cookie, Key, PrivateCookieJar};
use config::Config;
use constants::{BS_PRIMARY_COLOR, COOKIE_NAME};
use db::Database;
use dotenvy::dotenv;
use middleware::lb_heartbeat_middleware;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::io;
use std::{net::SocketAddr, sync::Arc};
use tera::{Context, Tera};
use tower::ServiceBuilder;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tower_http::{catch_panic::CatchPanicLayer, compression::CompressionLayer, services::ServeDir};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use wanikani::WaniKaniAPIClient;

mod config;
mod constants;
mod db;
mod middleware;
mod models;
mod wanikani;

static TEMPLATES: Lazy<Tera> = Lazy::new(|| match Tera::new("templates/*.html") {
    Ok(t) => t,
    Err(err) => panic!("Parsing error: {}", err),
});

async fn index(wanikani_api_key: Option<WaniKaniAPIKey>) -> impl IntoResponse {
    match wanikani_api_key {
        Some(_) => Redirect::to("/assignments"),
        None => Redirect::to("/login"),
    }
}

#[derive(Serialize, Debug)]
struct LoginContext {
    is_logged_in: bool,
    invalid_api_key: bool,
}

impl LoginContext {
    pub fn logged_out(invalid_api_key: bool) -> Self {
        Self {
            is_logged_in: false,
            invalid_api_key,
        }
    }
}

async fn login_get(wanikani_api_key: Option<WaniKaniAPIKey>) -> impl IntoResponse {
    if wanikani_api_key.is_some() {
        Redirect::to("/assignments").into_response()
    } else {
        Html::from(
            TEMPLATES
                .render(
                    "login.html",
                    &Context::from_serialize(&LoginContext::logged_out(false)).unwrap(),
                )
                .unwrap(),
        )
        .into_response()
    }
}

#[derive(Deserialize, Debug)]
struct LoginForm {
    api_key: String,
}

async fn login_post(
    Form(input): Form<LoginForm>,
    jar: PrivateCookieJar,
    state: Extension<Arc<State>>,
) -> impl IntoResponse {
    let api_key = input.api_key.trim().to_string();
    let api = WaniKaniAPIClient::new(&api_key, &state.http_client);

    match api.username().await {
        Ok(_) => {
            let cookie = Cookie::build(COOKIE_NAME, api_key)
                .secure(true)
                .http_only(true)
                .finish();
            let updated_jar = jar.add(cookie);
            (updated_jar, Redirect::to("/assignments")).into_response()
        }
        Err(err) => {
            if err.status().expect("error during request") == StatusCode::UNAUTHORIZED {
                (
                    StatusCode::UNAUTHORIZED,
                    Html::from(
                        TEMPLATES
                            .render(
                                "login.html",
                                &Context::from_serialize(&LoginContext::logged_out(true)).unwrap(),
                            )
                            .unwrap(),
                    ),
                )
                    .into_response()
            } else {
                unimplemented!("WaniKani API error");
            }
        }
    }
}

async fn logout(jar: PrivateCookieJar) -> (PrivateCookieJar, Redirect) {
    let updated_jar = jar.remove(Cookie::named(COOKIE_NAME));

    (updated_jar, Redirect::to("/login"))
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

struct WaniKaniAPIKey(String);

#[async_trait]
impl<B> FromRequest<B> for WaniKaniAPIKey
where
    B: Send,
{
    type Rejection = (StatusCode, Redirect);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let jar = PrivateCookieJar::<Key>::from_request(req)
            .await
            .map_err(|err| err.into_response());

        if let Ok(jar) = jar {
            if let Some(cookie) = jar.get(COOKIE_NAME) {
                return Ok(WaniKaniAPIKey(cookie.value().to_string()));
            }
        }
        Err((StatusCode::UNAUTHORIZED, Redirect::to("/login")))
    }
}

fn create_app(config: Config, http_client: reqwest::Client) -> Router {
    let state = Arc::new(State { http_client });
    let key = Key::from(&config.session_key.into_bytes());

    Router::new()
        .route("/", get(index))
        .route("/login", get(login_get))
        .route("/login", post(login_post))
        .route("/logout", get(logout))
        .route("/radical-svg/:path", get(radical_svg))
        .route("/test-500", get(test_500))
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
                .layer(CompressionLayer::new())
                .layer(axum::middleware::from_fn(lb_heartbeat_middleware))
                .layer(Extension(state))
                .layer(Extension(key)),
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

    let config = match envy::from_env::<Config>() {
        Ok(config) => config,
        Err(err) => panic!("{:#?}", err),
    };
    let addr = config
        .bind_address
        .parse::<SocketAddr>()
        .expect("invalid BIND_ADDRESS");

    let api = WaniKaniAPIClient::new(&config.wanikani_api_key, &http_client);

    // Load the WaniKani data
    let mut db = Database::new();
    db.populate(&api).await?;

    // Build the application
    let app = create_app(config, http_client);

    // Serve the app
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
        http::{header, Request, StatusCode},
    };
    use mockito::mock;
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};
    use serde_json::json;
    use tower::ServiceExt;

    #[fixture]
    fn app() -> Router {
        create_app(
            Config {
                wanikani_api_key: "fake-key".to_string(),
                session_key: "58dea9de79168641df396a89d4b80a83db10c44e0d9e51248d1cf8a17c9e8224"
                    .to_string(),
                bind_address: "127.0.0.1:0".to_string(),
            },
            reqwest::Client::new(),
        )
    }

    mod index {
        use super::*;
        use pretty_assertions::assert_eq;

        #[rstest]
        #[tokio::test]
        async fn logged_in(app: Router) {
            let _m = mock("GET", "/user")
                .with_status(200)
                .with_body(json!({"data": {"username": "test-user"}}).to_string())
                .create();

            let resp = app
                .clone()
                .oneshot(
                    Request::post("/login")
                        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                        .body(Body::from("api_key=fake-api-key"))
                        .unwrap(),
                )
                .await
                .unwrap();
            let cookie = resp.headers().get(header::SET_COOKIE).unwrap();

            let resp = app
                .oneshot(
                    Request::get("/")
                        .header(header::COOKIE, cookie)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                resp.headers().get(header::LOCATION).unwrap(),
                "/assignments"
            );
        }

        #[rstest]
        #[tokio::test]
        async fn logged_out(app: Router) {
            let resp = app
                .oneshot(Request::get("/").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::SEE_OTHER);
            assert_eq!(resp.headers().get(header::LOCATION).unwrap(), "/login");
        }
    }

    mod login {
        use super::*;
        use pretty_assertions::assert_eq;

        #[rstest]
        #[tokio::test]
        async fn already_logged_in(app: Router) {
            let _m = mock("GET", "/user")
                .with_status(200)
                .with_body(json!({"data": {"username": "test-user"}}).to_string())
                .create();

            let resp = app
                .clone()
                .oneshot(
                    Request::post("/login")
                        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                        .body(Body::from("api_key=fake-api-key"))
                        .unwrap(),
                )
                .await
                .unwrap();
            let cookie = resp.headers().get(header::SET_COOKIE).unwrap();

            let resp = app
                .oneshot(
                    Request::get("/login")
                        .header(header::COOKIE, cookie)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                resp.headers().get(header::LOCATION).unwrap(),
                "/assignments"
            );
        }

        #[rstest]
        #[tokio::test]
        async fn valid_api_key(app: Router) {
            let _m = mock("GET", "/user")
                .with_status(200)
                .with_body(json!({"data": {"username": "test-user"}}).to_string())
                .create();

            let resp = app
                .oneshot(
                    Request::post("/login")
                        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                        .body(Body::from("api_key=fake-api-key"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::SEE_OTHER);
            assert_eq!(
                resp.headers().get(header::LOCATION).unwrap(),
                "/assignments"
            );
        }

        #[rstest]
        #[tokio::test]
        async fn invalid_api_key(app: Router) {
            let _m = mock("GET", "/user").with_status(401).create();

            let resp = app
                .oneshot(
                    Request::post("/login")
                        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                        .body(Body::from("api_key=fake-api-key"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

            let body = String::from_utf8(
                hyper::body::to_bytes(resp.into_body())
                    .await
                    .unwrap()
                    .to_vec(),
            )
            .unwrap();
            assert!(body.contains("is-invalid"));
        }
    }

    #[rstest]
    #[tokio::test]
    async fn logout(app: Router) {
        let _m = mock("GET", "/user")
            .with_status(200)
            .with_body(json!({"data": {"username": "test-user"}}).to_string())
            .create();

        let resp = app
            .clone()
            .oneshot(
                Request::post("/login")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("api_key=fake-api-key"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let cookie = resp.headers().get(header::SET_COOKIE).unwrap();

        let resp = app
            .oneshot(
                Request::get("/logout")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        assert_eq!(resp.headers().get(header::LOCATION).unwrap(), "/login");
        assert!(resp
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("wanikani-api-key=;"));
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
