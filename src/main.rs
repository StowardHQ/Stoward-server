use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};
use std::{
    env,
    sync::{Arc, Mutex},
};
use tower_http::cors::CorsLayer;

mod auth;
mod background;
mod db;
mod handlers;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let master_api_key =
        env::var("DISCOVERY_API_KEY").expect("DISCOVERY_API_KEY environment variable not set");
    let conn = db::init_db();
    let shared_db = Arc::new(Mutex::new(conn));

    tokio::spawn(background::start_refresh_job(Arc::clone(&shared_db)));

    let authenticated_routes = Router::new()
        .route("/api/servers", post(handlers::sync_server))
        .route("/api/servers/:sid/bump", post(handlers::bump_server))
        .route(
            "/api/servers/:sid",
            axum::routing::patch(handlers::patch_server),
        )
        .route_layer(axum_middleware::from_fn_with_state(
            master_api_key,
            auth::api_key_auth,
        ));

    let app = Router::new()
        .route("/api/servers", get(handlers::get_servers))
        .route("/api/servers/:sid", get(handlers::get_server_by_id))
        .merge(authenticated_routes)
        .layer(CorsLayer::permissive())
        .with_state(shared_db);

    let port = 4000;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    println!("Server running on: {port}");

    axum::serve(listener, app).await.unwrap();
}
