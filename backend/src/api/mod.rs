use crate::AppState;
pub mod health;
pub mod identify;
pub mod tracks;

use axum::{Router, extract::DefaultBodyLimit, routing::get};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
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
        // Use configured origins. We've already validated these in Settings::from_env
        let origins = state
            .settings
            .cors_allowed_origins
            .iter()
            .map(|origin| origin.parse().unwrap())
            .collect::<Vec<axum::http::HeaderValue>>();

        cors.allow_origin(tower_http::cors::AllowOrigin::list(origins))
    };

    // Enable credentials if configured.
    // Note: tower-http will panic if allow_credentials(true) is used with Any origin.
    let cors = if state.settings.cors_allow_credentials {
        if state.settings.cors_allowed_origins.is_empty() {
            tracing::warn!(
                "CORS_ALLOW_CREDENTIALS=true is ignored because CORS_ALLOWED_ORIGINS is empty (Any)"
            );
            cors
        } else {
            cors.allow_credentials(true)
        }
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
