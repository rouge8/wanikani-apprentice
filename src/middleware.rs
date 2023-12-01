use std::task::{Context, Poll};

use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use futures::future::BoxFuture;
use tower::{Layer, Service};

/// Add a `/__lbheartbeat__` endpoint that always responds with `OK`.
///
/// This endpoint is intended to be used as a health check for load balancers since it will always
/// return HTTP 200 if the app is up.
pub async fn lb_heartbeat_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
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

/// Enforce that all incoming requests have a correctly set `Host` header, in order to guard
/// against HTTP Host Header attacks.
#[derive(Clone)]
pub struct TrustedHostLayer {
    trusted_hosts: Vec<String>,
}

impl TrustedHostLayer {
    pub fn new(trusted_hosts: Vec<String>) -> Self {
        Self { trusted_hosts }
    }
}

impl<S> Layer<S> for TrustedHostLayer {
    type Service = TrustedHostMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TrustedHostMiddleware {
            trusted_hosts: self.trusted_hosts.clone(),
            inner,
        }
    }
}

#[derive(Clone)]
pub struct TrustedHostMiddleware<S> {
    trusted_hosts: Vec<String>,
    inner: S,
}

impl<S> Service<Request<Body>> for TrustedHostMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let header = req
            .headers()
            .get(header::HOST)
            .map(|header| header.to_owned())
            .unwrap_or(HeaderValue::from_static(""));
        let host = header.to_str().unwrap().split(':').collect::<Vec<&str>>()[0];

        if !self.trusted_hosts.is_empty() && self.trusted_hosts.contains(&host.to_string()) {
            let future = self.inner.call(req);
            Box::pin(async move {
                let response: Response = future.await?;
                Ok(response)
            })
        } else {
            Box::pin(
                async move { Ok((StatusCode::BAD_REQUEST, "Invalid host header").into_response()) },
            )
        }
    }
}
