//! Service readiness (`GET /ready`).
//!
//! Readiness requires database connectivity and foundation schema readiness
//! (IA §30.3). Temporary DB unavailability returns not-ready without affecting
//! liveness.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
pub struct ReadyResponse {
    pub status: &'static str,
}

#[derive(Serialize)]
struct NotReadyResponse {
    status: &'static str,
    reason: &'static str,
}

/// Readiness probe: required dependencies are available.
pub async fn ready(State(state): State<AppState>) -> Response {
    match check_ready(&state).await {
        Ok(()) => (StatusCode::OK, Json(ReadyResponse { status: "ready" })).into_response(),
        Err(reason) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(NotReadyResponse {
                status: "not_ready",
                reason,
            }),
        )
            .into_response(),
    }
}

async fn check_ready(state: &AppState) -> Result<(), &'static str> {
    tx4_persistence::check_connectivity(&state.pool)
        .await
        .map_err(|_| "database_unavailable")?;
    tx4_persistence::check_schema_foundation(&state.pool)
        .await
        .map_err(|_| "schema_not_ready")?;
    Ok(())
}
