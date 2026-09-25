//! Process liveness (`GET /health`).
//!
//! Liveness reflects process responsiveness only. It must not require PostgreSQL
//! (IA §30.3: LIVENESS ≠ READINESS).

use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

/// Liveness probe: the process can answer HTTP.
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_payload_has_no_secrets() {
        let json = serde_json::to_string(&HealthResponse { status: "ok" }).unwrap();
        assert_eq!(json, r#"{"status":"ok"}"#);
        assert!(!json.contains("postgres"));
        assert!(!json.contains("password"));
        assert!(!json.contains("secret"));
    }
}
