use crate::AppState;
pub mod health;
pub mod identify;
pub mod tracks;

use axum::{Router, extract::DefaultBodyLimit, routing::get};
use std::sync::Arc;
use tower_http::cors::{CorsLayer, AllowOrigin};
use tower_http::trace::TraceLayer;

pub fn create_router(state: Arc<AppState>) -> Router {
    // Build CORS layer with configured origins
    let cors = CorsLayer::new()
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::PATCH,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ]);

    // Configure allowed origins
    let cors = if state.settings.cors_allowed_origins.is_empty() {
        // If no origins specified, allow any (for development)
        cors.allow_origin(tower_http::cors::Any)
    } else {
        // Use configured origins
        let origins = state.settings.cors_allowed_origins
            .iter()
            .filter_map(|origin| origin.parse().ok())
            .collect::<Vec<_>>();
        
        if origins.is_empty() {
            cors.allow_origin(tower_http::cors::Any)
        } else {
            cors.allow_origin(tower_http::cors::AllowOrigin::list(origins))
        }
    };

    // Enable credentials if configured
    let cors = if state.settings.cors_allow_credentials {
        cors.allow_credentials(true)
    } else {
        cors
    };

    Router::new()
        .route("/health", get(health::health_check))
        .nest(
            "/tracks",
            tracks::router().layer(DefaultBodyLimit::max(state.settings.max_file_size)),
        )
        .nest(
            "/identify",
            identify::router().layer(DefaultBodyLimit::max(state.settings.identify_max_file_size)),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
