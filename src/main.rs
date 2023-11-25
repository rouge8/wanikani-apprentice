use std::fmt;
use std::net::SocketAddr;

use axum::body::{self, Empty, Full};
use axum::extract::{FromRef, FromRequestParts, Path, State};
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{async_trait, Form, Router};
use axum_extra::extract::cookie::{Cookie, Key, PrivateCookieJar};
use chrono::{DateTime, Utc};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use dotenvy::dotenv;
use git_version::git_version;
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::{info, Level};
use tracing_subscriber::prelude::*;
use tracing_subscriber::FmtSubscriber;

use crate::config::Config;
use crate::constants::{BS_PRIMARY_COLOR, COOKIE_NAME};
use crate::db::Database;
use crate::middleware::{lb_heartbeat_middleware, TrustedHostLayer};
use crate::models::{Assignment, Subject};
use crate::resources::{STATIC_DIR, TEMPLATES};
use crate::wanikani::WaniKaniAPIClient;

mod config;
mod constants;
mod db;
mod middleware;
mod models;
mod resources;
mod wanikani;

fn display_time_remaining(_state: &minijinja::State, value: String, now: String) -> String {
    let value = DateTime::parse_from_rfc3339(&value).expect("unable to parse DateTime");
    let now = DateTime::parse_from_rfc3339(&now).expect("unable to parse DateTime");
    let delta = value.signed_duration_since(now);

    if delta.num_seconds() > 0 {
        HumanTime::from(delta).to_text_en(Accuracy::Rough, Tense::Future)
    } else {
        "now".to_string()
    }
}

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
                .get_template("login.html")
                .unwrap()
                .render(LoginContext::logged_out(false))
                .unwrap(),
        )
        .into_response()
    }
}

#[derive(Clone, Deserialize, Debug)]
struct LoginForm {
    api_key: String,
}

async fn login_post(
    jar: PrivateCookieJar,
    State(state): State<AppState>,
    State(wanikani_api_url): State<WaniKaniAPIURL>,
    Form(input): Form<LoginForm>,
) -> impl IntoResponse {
    let api_key = input.api_key.trim().to_string();
    let api = WaniKaniAPIClient::new(&api_key, &wanikani_api_url.to_string(), &state.http_client);

    match api.username().await {
        Ok(_) => {
            let mut cookie =
                Cookie::build(COOKIE_NAME, api_key)
                    .secure(true)
                    .http_only(true)
                    .finish();
            cookie.make_permanent();
            let updated_jar = jar.add(cookie);
            (updated_jar, Redirect::to("/assignments")).into_response()
        }
        Err(err) => {
            if err.status().expect("error during request") == StatusCode::UNAUTHORIZED {
                (
                    StatusCode::UNAUTHORIZED,
                    Html::from(
                        TEMPLATES
                            .get_template("login.html")
                            .unwrap()
                            .render(LoginContext::logged_out(true))
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

#[derive(Serialize, Debug)]
struct AssignmentContext {
    is_logged_in: bool,
    radicals: Vec<Assignment>,
    kanji: Vec<Assignment>,
    vocabulary: Vec<Assignment>,
    kana_vocabulary: Vec<Assignment>,
    now: DateTime<Utc>,
}

impl AssignmentContext {
    pub fn new(
        radicals: Vec<Assignment>,
        kanji: Vec<Assignment>,
        vocabulary: Vec<Assignment>,
        kana_vocabulary: Vec<Assignment>,
    ) -> Self {
        Self {
            is_logged_in: true,
            radicals,
            kanji,
            vocabulary,
            kana_vocabulary,
            now: Utc::now(),
        }
    }
}

async fn assignments(
    wanikani_api_key: WaniKaniAPIKey,
    State(http_client): State<reqwest::Client>,
    State(db): State<Database>,
    State(wanikani_api_url): State<WaniKaniAPIURL>,
) -> impl IntoResponse {
    let api = WaniKaniAPIClient::new(
        &wanikani_api_key.to_string(),
        &wanikani_api_url.to_string(),
        &http_client,
    );

    let mut radicals = Vec::new();
    let mut kanji = Vec::new();
    let mut vocabulary = Vec::new();
    let mut kana_vocabulary = Vec::new();

    let mut assignments = api
        .assignments(&db)
        .await
        .expect("failed fetching assignments");

    assignments.sort_by_key(|assignment| assignment.available_at);

    for assignment in assignments {
        match assignment.subject {
            Subject::Radical(_) => radicals.push(assignment),
            Subject::Kanji(_) => kanji.push(assignment),
            Subject::Vocabulary(_) => vocabulary.push(assignment),
            Subject::KanaVocabulary(_) => kana_vocabulary.push(assignment),
        }
    }

    Html::from(
        TEMPLATES
            .get_template("assignments.html")
            .unwrap()
            .render(AssignmentContext::new(radicals, kanji, vocabulary, kana_vocabulary))
            .unwrap(),
    )
    .into_response()
}

/// Mirror the WaniKani radical SVGs, replacing the `stroke` color with our primary color.
async fn radical_svg(
    Path(path): Path<String>,
    State(wanikani_files_server_url): State<WaniKaniFilesServerURL>,
    State(http_client): State<reqwest::Client>,
) -> impl IntoResponse {
    let url = format!("{wanikani_files_server_url}/{path}");
    info!(url, "downloading SVG");
    let resp = http_client
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

/// Servce static files from the binary
async fn static_file(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');
    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    match STATIC_DIR.get_file(path) {
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            )
            .body(body::boxed(Full::from(file.contents())))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))
            .unwrap(),
    }
}

async fn test_500() {
    let _ = 1 / 0;
}

#[derive(Clone, FromRef)]
struct AppState {
    db: Database,
    http_client: reqwest::Client,
    key: Key,
    wanikani_api_url: WaniKaniAPIURL,
    wanikani_files_server_url: WaniKaniFilesServerURL,
}

struct WaniKaniAPIKey(String);

impl fmt::Display for WaniKaniAPIKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for WaniKaniAPIKey
where
    S: Send + Sync,
    Key: FromRef<S>,
{
    type Rejection = (StatusCode, Redirect);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = PrivateCookieJar::<Key>::from_request_parts(parts, state)
            .await
            .map_err(|err| err.into_response());

        if let Ok(jar) = jar {
            if let Some(cookie) = jar.get(COOKIE_NAME) {
                return Ok(WaniKaniAPIKey(cookie.value().to_string()));
            }
        }
        Err((StatusCode::SEE_OTHER, Redirect::to("/login")))
    }
}

#[derive(Debug, Clone)]
struct WaniKaniAPIURL(String);

impl fmt::Display for WaniKaniAPIURL {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
struct WaniKaniFilesServerURL(String);

impl fmt::Display for WaniKaniFilesServerURL {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn create_app(config: Config, db: Database, http_client: reqwest::Client) -> Router {
    let key = Key::from(&config.session_key.into_bytes());
    let state = AppState {
        db,
        http_client,
        key,
        wanikani_api_url: WaniKaniAPIURL(config.wanikani_api_url),
        wanikani_files_server_url: WaniKaniFilesServerURL(config.wanikani_files_server_url),
    };

    Router::new()
        .route("/", get(index))
        .route("/login", get(login_get))
        .route("/login", post(login_post))
        .route("/logout", get(logout))
        .route("/assignments", get(assignments))
        .route("/radical-svg/:path", get(radical_svg))
        .route("/static/:path", get(static_file))
        .route("/test-500", get(test_500))
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
                .layer(sentry_tower::NewSentryLayer::new_from_top())
                .layer(sentry_tower::SentryHttpLayer::with_transaction())
                .layer(axum::middleware::from_fn(lb_heartbeat_middleware))
                .layer(CompressionLayer::new())
                .layer(TrustedHostLayer::new(config.trusted_hosts)),
        )
        .with_state(state)
}

#[tokio::main]
async fn main() -> reqwest::Result<()> {
    dotenv().ok();
    let config = match envy::from_env::<Config>() {
        Ok(config) => config,
        Err(err) => panic!("{err:#?}"),
    };

    // Configure Sentry
    let mut opts = sentry::apply_defaults(sentry::ClientOptions {
        release: Some(git_version!(args = ["--always", "--abbrev=40"]).into()),
        ..Default::default()
    });
    // Disable debug-images: it conflicts with the 'debug = 1' rustc build option:
    // https://github.com/getsentry/sentry-rust/issues/574
    opts.integrations.retain(|i| i.name() != "debug-images");
    opts.default_integrations = false;
    let _guard = sentry::init((config.sentry_dsn.clone(), opts));

    // Configure logging
    let subscriber = FmtSubscriber::builder()
        .finish()
        .with(sentry_tracing::layer());
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let http_client = reqwest::Client::new();

    let addr = config
        .bind_address
        .parse::<SocketAddr>()
        .expect("invalid BIND_ADDRESS");

    // Load the WaniKani data
    let api =
        WaniKaniAPIClient::new(
            &config.wanikani_api_key,
            &config.wanikani_api_url,
            &http_client,
        );
    let mut db = Database::new();
    db.populate(&api).await?;

    // Build the application
    let app = create_app(config, db, http_client);

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
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use minijinja::{context, Environment};
    use rstest::{fixture, rstest};
    use serde_json::json;
    use similar_asserts::assert_eq;
    use tower::ServiceExt;

    use super::*;

    #[fixture]
    async fn mockito_server() -> mockito::ServerGuard {
        mockito::Server::new_async().await
    }

    fn create_test_app(server: &mockito::ServerGuard) -> Router {
        create_app(
            Config {
                wanikani_api_key: "fake-key".to_string(),
                wanikani_api_url: server.url(),
                wanikani_files_server_url: server.url(),
                session_key: "58dea9de79168641df396a89d4b80a83db10c44e0d9e51248d1cf8a17c9e8224"
                    .to_string(),
                bind_address: "127.0.0.1:0".to_string(),
                sentry_dsn: None,
                trusted_hosts: vec!["".to_string()],
            },
            Database::new(),
            reqwest::Client::new(),
        )
    }

    mod index {
        use similar_asserts::assert_eq;

        use super::*;

        #[rstest]
        #[tokio::test]
        async fn logged_in(#[future] mockito_server: mockito::ServerGuard) {
            let mut mockito_server = mockito_server.await;
            let app = create_test_app(&mockito_server);
            let _m = mockito_server
                .mock("GET", "/user")
                .with_status(200)
                .with_body(json!({"data": {"username": "test-user"}}).to_string())
                .create_async()
                .await;

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
        async fn logged_out(#[future] mockito_server: mockito::ServerGuard) {
            let mockito_server = mockito_server.await;
            let app = create_test_app(&mockito_server);
            let resp = app
                .oneshot(Request::get("/").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::SEE_OTHER);
            assert_eq!(resp.headers().get(header::LOCATION).unwrap(), "/login");
        }
    }

    mod login {
        use similar_asserts::assert_eq;

        use super::*;

        #[rstest]
        #[tokio::test]
        async fn already_logged_in(#[future] mockito_server: mockito::ServerGuard) {
            let mut mockito_server = mockito_server.await;
            let app = create_test_app(&mockito_server);
            let _m = mockito_server
                .mock("GET", "/user")
                .with_status(200)
                .with_body(json!({"data": {"username": "test-user"}}).to_string())
                .create_async()
                .await;

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
        async fn valid_api_key(#[future] mockito_server: mockito::ServerGuard) {
            let mut mockito_server = mockito_server.await;
            let app = create_test_app(&mockito_server);
            let _m = mockito_server
                .mock("GET", "/user")
                .with_status(200)
                .with_body(json!({"data": {"username": "test-user"}}).to_string())
                .create_async()
                .await;

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
        async fn invalid_api_key(#[future] mockito_server: mockito::ServerGuard) {
            let mut mockito_server = mockito_server.await;
            let app = create_test_app(&mockito_server);
            let _m = mockito_server
                .mock("GET", "/user")
                .with_status(401)
                .create_async()
                .await;

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
            assert!(body.contains("Invalid API key."));
        }
    }

    #[rstest]
    #[tokio::test]
    async fn logout(#[future] mockito_server: mockito::ServerGuard) {
        let mut mockito_server = mockito_server.await;
        let app = create_test_app(&mockito_server);
        let _m = mockito_server
            .mock("GET", "/user")
            .with_status(200)
            .with_body(json!({"data": {"username": "test-user"}}).to_string())
            .create_async()
            .await;

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

    mod assignments {
        use similar_asserts::assert_eq;

        use super::*;

        #[rstest]
        #[tokio::test]
        async fn logged_out_redirect(#[future] mockito_server: mockito::ServerGuard) {
            let mockito_server = mockito_server.await;
            let app = create_test_app(&mockito_server);
            let resp = app
                .oneshot(Request::get("/assignments").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::SEE_OTHER);
            assert_eq!(resp.headers().get(header::LOCATION).unwrap(), "/login");
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_radical_svg(#[future] mockito_server: mockito::ServerGuard) {
        let mut mockito_server = mockito_server.await;
        let app = create_test_app(&mockito_server);
        let _m =
            mockito_server
                .mock("GET", "/foo")
                .with_status(200)
                .with_body("foo bar stroke:#000 other:#000")
                .create_async()
                .await;

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
    async fn test_500(#[future] mockito_server: mockito::ServerGuard) {
        let mockito_server = mockito_server.await;
        let app = create_test_app(&mockito_server);
        let resp = app
            .oneshot(Request::get("/test-500").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    mod lb_heartbeat {
        use similar_asserts::assert_eq;

        use super::*;

        #[rstest]
        #[tokio::test]
        async fn ok(#[future] mockito_server: mockito::ServerGuard) {
            let mockito_server = mockito_server.await;
            let app = create_test_app(&mockito_server);
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

        #[rstest]
        #[tokio::test]
        async fn ignores_host_header(#[future] mockito_server: mockito::ServerGuard) {
            let mockito_server = mockito_server.await;
            let app = create_test_app(&mockito_server);
            let resp = app
                .oneshot(
                    Request::get("/__lbheartbeat__")
                        .header(header::HOST, "foo.com")
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

    #[rstest]
    #[tokio::test]
    async fn trusted_host_header(#[future] mockito_server: mockito::ServerGuard) {
        let mockito_server = mockito_server.await;
        let app = create_test_app(&mockito_server);
        let resp = app
            .oneshot(
                Request::get("/")
                    .header(header::HOST, "foo.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
        assert_eq!(body, "Invalid host header");
    }

    #[rstest]
    #[case("2022-01-01T00:00:00Z", "2022-01-01T00:00:00Z", "now")]
    #[case("2022-01-01T00:00:00Z", "2022-01-01T00:00:01Z", "now")]
    #[case("2022-01-01T00:55:00Z", "2022-01-01T00:00:00Z", "in an hour")]
    #[case("2022-01-01T23:00:00Z", "2022-01-01T00:00:00Z", "in a day")]
    #[case("2022-01-01T01:45:00Z", "2022-01-01T00:00:00Z", "in 2 hours")]
    #[case("2022-01-01T00:20:00Z", "2022-01-01T00:00:00Z", "in 20 minutes")]
    fn test_display_time_remaining(#[case] value: &str, #[case] now: &str, #[case] expected: &str) {
        let mut env = Environment::new();
        env.add_filter("display_time_remaining", display_time_remaining);
        env.add_template("test", "{{ value | display_time_remaining(now) }}")
            .unwrap();

        let tmpl = env.get_template("test").unwrap();
        assert_eq!(tmpl.render(context! { value, now }).unwrap(), expected);
    }
}
