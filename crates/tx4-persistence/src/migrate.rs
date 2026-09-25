//! Ordered forward SQLx migration runner.

use crate::error::PersistenceError;
use sqlx::migrate::Migrator;
use sqlx::PgPool;
use std::path::Path;

/// Embedded migrations directory (resolved at compile time to this crate).
fn migrations_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"))
}

/// Run all ordered migrations under `crates/tx4-persistence/migrations/`.
///
/// Uses the filesystem path baked from `CARGO_MANIFEST_DIR` so `cargo check` /
/// `cargo test` do not require a live database at compile time.
pub async fn run_migrations(pool: &PgPool) -> Result<(), PersistenceError> {
    let migrator = Migrator::new(migrations_dir())
        .await
        .map_err(|_err| PersistenceError::Migrate)?;
    migrator
        .run(pool)
        .await
        .map_err(|_err| PersistenceError::Migrate)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_dir_points_at_persistence_migrations() {
        let dir = migrations_dir();
        assert!(dir.ends_with("migrations"));
        assert!(dir.join("0001_schema_foundation.sql").is_file());
    }
}
