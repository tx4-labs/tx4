//! OpenTelemetry-compatible export interface (vendor OPEN).

use tx4_config::Config;

/// How TX4 plans to export telemetry without locking OSS to a hosted vendor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OtelExportInterface {
    /// Spans/events remain on the `tracing` bus; an OTLP bridge may attach later.
    TracingBridge,
}

/// Validated OTel-compatible export plan derived from process configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OtelExportPlan {
    pub service_name: String,
    pub environment: String,
    pub endpoint: Option<String>,
}

impl OtelExportPlan {
    pub fn from_config(config: &Config) -> Self {
        Self {
            service_name: config.service_name.clone(),
            environment: config.environment.clone(),
            endpoint: config.otlp_endpoint.clone(),
        }
    }

    pub fn interface(&self) -> OtelExportInterface {
        OtelExportInterface::TracingBridge
    }

    pub fn is_export_configured(&self) -> bool {
        self.endpoint.is_some()
    }
}
