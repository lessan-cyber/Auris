use crate::AppState;
pub mod health;
pub mod identify;
pub mod tracks;

use axum::{Router, extract::DefaultBodyLimit, routing::get};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health::health_check))
        .nest(
            "/tracks",
            tracks::router().layer(DefaultBodyLimit::max(state.settings.max_file_size)),
        )
        .nest("/identify", identify::router())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
