//! Infrastructure routes only (no business `/v1` surface in Phase-1E).

mod health;
mod ready;

use axum::routing::get;
use axum::Router;

use crate::AppState;

pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(ready::ready))
}
