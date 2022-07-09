use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Add a `/__lbheartbeat__` endpoint that always responds with `OK`.
///
/// This endpoint is intended to be used as a health check for load balancers since it will always
/// return HTTP 200 if the app is up.
pub async fn lb_heartbeat_middleware<B>(
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
