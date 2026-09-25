//! TX4 HTTP server binary.
//!
//! Phase-1E bootstrap:
//! configuration → observability → database pool → Axum serve → graceful shutdown.
//!
//! Migrations are NOT run here (`tx4-cli migrate` first). No business routes.

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
        http_bind = %config.http_bind,
        "tx4-server starting"
    );

    let pool = match tx4_persistence::connect_pool(&config).await {
        Ok(pool) => pool,
        Err(err) => {
            tracing::error!(error = %err, "database pool connection failed");
            eprintln!("database: {err}");
            return ExitCode::FAILURE;
        }
    };

    let bind = config.http_bind;
    let state = tx4_api::AppState::new(config, pool.clone());
    let app = tx4_api::router(state);

    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!(error = %err, %bind, "failed to bind HTTP listener");
            eprintln!("bind: {err}");
            tx4_persistence::close_pool(&pool).await;
            return ExitCode::FAILURE;
        }
    };

    tracing::info!(%bind, "tx4-server listening");

    let result = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tx4_observability::wait_for_shutdown_signal().await;
            tracing::info!("shutdown signal received; draining HTTP server");
        })
        .await;

    tx4_persistence::close_pool(&pool).await;

    match result {
        Ok(()) => {
            tracing::info!("tx4-server stopped");
            ExitCode::SUCCESS
        }
        Err(err) => {
            tracing::error!(error = %err, "tx4-server exited with error");
            eprintln!("server: {err}");
            ExitCode::FAILURE
        }
    }
}
