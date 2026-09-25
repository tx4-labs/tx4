//! TX4 persistence adapters (PostgreSQL / SQLx).
//!
//! Phase-1C: connection pool, migration runner, and connectivity health.
//! No business repositories or business schema.

#![forbid(unsafe_code)]

mod error;
mod health;
mod migrate;
mod pool;

pub use error::PersistenceError;
pub use health::{check_connectivity, check_schema_foundation};
pub use migrate::run_migrations;
pub use pool::{close_pool, connect_pool};

pub use sqlx::PgPool;

pub use tx4_application;
pub use tx4_domain;

/// Marker that the persistence crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_tx4_persistence() {
        assert_eq!(crate_name(), "tx4-persistence");
    }
}
