//! TX4 server binary.
//!
//! Phase-1D: configuration → observability initialization → bootstrap.
//! No HTTP listen, routes, or business endpoints.

use std::process::ExitCode;

fn main() -> ExitCode {
    let config = match tx4_config::Config::from_env() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("config: {err}");
            return ExitCode::FAILURE;
        }
    };

    let _observability = match tx4_observability::init(&config) {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("observability: {err}");
            return ExitCode::FAILURE;
        }
    };

    tracing::info!(
        service = %config.service_name,
        environment = %config.environment,
        http_bind = %config.http_bind,
        api = tx4_api::crate_name(),
        "tx4-server bootstrap (no business routes)"
    );

    ExitCode::SUCCESS
}
