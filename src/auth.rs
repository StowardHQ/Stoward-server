use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

pub async fn api_key_auth(
    State(master_key): State<String>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user_key_str = req
        .headers()
        .get("x-api-key")
        .and_then(|key| key.to_str().ok());

    if user_key_str == Some(master_key.as_str()) {
        return Ok(next.run(req).await);
    }

    Err(StatusCode::UNAUTHORIZED)
}
