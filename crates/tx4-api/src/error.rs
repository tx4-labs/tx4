//! Minimal API error surface for infrastructure responses.
//!
//! Must not leak SQL details, stack traces, or secrets.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// Infrastructure-safe API error for health/readiness and future mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApiError {
    NotReady,
    Internal,
}

#[derive(Serialize)]
struct ErrorBody {
    status: &'static str,
    error: &'static str,
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotReady => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::NotReady => "not_ready",
            Self::Internal => "internal",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(ErrorBody {
            status: self.code(),
            error: self.code(),
        });
        (self.status_code(), body).into_response()
    }
}
