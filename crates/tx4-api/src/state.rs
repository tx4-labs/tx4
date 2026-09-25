//! Shared HTTP application state for the server foundation.

use tx4_config::Config;
use tx4_persistence::PgPool;

/// Runtime state shared by Axum handlers.
///
/// Contains configuration and the PostgreSQL pool only — no business aggregates.
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub pool: PgPool,
}

impl AppState {
    pub fn new(config: Config, pool: PgPool) -> Self {
        Self { config, pool }
    }
}
