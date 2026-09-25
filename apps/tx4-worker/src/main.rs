//! TX4 worker binary.
//!
//! Phase-1E bootstrap:
//! configuration → observability → database pool → idle lifecycle → graceful shutdown.
//!
//! No outbox claim/lease/processing. No business job loops.
//! Migrations are NOT run here (`tx4-cli migrate` first).

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
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
        worker = tx4_worker::crate_name(),
        "tx4-worker starting (idle foundation; no job claiming)"
    );

    let pool = match tx4_persistence::connect_pool(&config).await {
        Ok(pool) => pool,
        Err(err) => {
            tracing::error!(error = %err, "database pool connection failed");
            eprintln!("database: {err}");
            return ExitCode::FAILURE;
        }
    };

    // Idle lifecycle: wait for shutdown without claiming jobs.
    tx4_observability::wait_for_shutdown_signal().await;
    tracing::info!("shutdown signal received; stopping worker");

    tx4_persistence::close_pool(&pool).await;
    tracing::info!("tx4-worker stopped");
    ExitCode::SUCCESS
}
