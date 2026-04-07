use crate::AppState;
pub mod tracks;
use axum::{Router, extract::DefaultBodyLimit, routing::get};
use std::sync::Arc;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .nest(
            "/tracks",
            tracks::router().layer(DefaultBodyLimit::max(state.settings.max_file_size)),
        )
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}
