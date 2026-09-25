//! Tracing subscriber initialization.

use crate::otel::OtelExportPlan;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;
use tx4_config::Config;

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Guard returned by [`init`]. Dropping does not uninstall the global subscriber.
#[derive(Debug)]
pub struct ObservabilityGuard {
    pub otel: OtelExportPlan,
}

/// Initialize structured `tracing` for the process.
///
/// Idempotent: subsequent calls succeed without reinstalling the global subscriber.
/// Logs/traces remain diagnostic only (IA §21).
pub fn init(config: &Config) -> Result<ObservabilityGuard, ObservabilityError> {
    let otel = OtelExportPlan::from_config(config);

    if INITIALIZED.load(Ordering::SeqCst) {
        return Ok(ObservabilityGuard { otel });
    }

    let filter = EnvFilter::try_new(config.log_level.as_str()).map_err(|_| {
        ObservabilityError::InvalidFilter {
            directive: config.log_level.clone(),
        }
    })?;

    let subscriber = tracing_subscriber::registry().with(filter).with(
        fmt::layer()
            .with_target(true)
            .with_level(true)
            .with_ansi(false),
    );

    // If another subscriber is already installed, keep it and mark initialized.
    let _ = tracing::subscriber::set_global_default(subscriber);
    INITIALIZED.store(true, Ordering::SeqCst);

    tracing::info!(
        service_name = %config.service_name,
        environment = %config.environment,
        otlp_configured = otel.is_export_configured(),
        otel_interface = ?otel.interface(),
        "observability initialized (telemetry is non-authoritative)"
    );

    // Endpoint is recorded in the export plan only; no vendor SDK is selected (OPEN).
    if let Some(endpoint) = otel.endpoint.as_deref() {
        tracing::info!(
            otlp_endpoint = %endpoint,
            "OTLP endpoint configured for future OpenTelemetry-compatible export"
        );
    }

    Ok(ObservabilityGuard { otel })
}

/// Observability initialization failures (no secrets in messages).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservabilityError {
    InvalidFilter { directive: String },
}

impl std::fmt::Display for ObservabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFilter { .. } => {
                write!(f, "invalid TX4_LOG_LEVEL / tracing filter directive")
            }
        }
    }
}

impl std::error::Error for ObservabilityError {}
