use crate::AppState;
pub mod health;
pub mod identify;
pub mod tracks;
pub mod websocket;
use axum::{Router, extract::DefaultBodyLimit, routing::get};
use std::sync::Arc;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .nest(
            "/tracks",
            tracks::router().layer(DefaultBodyLimit::max(state.settings.max_file_size)),
        )
        .nest("/identify", identify::router())
        .with_state(state)
}
