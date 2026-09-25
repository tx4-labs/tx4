//! Minimal database connectivity / health primitive.

use crate::error::PersistenceError;
use sqlx::PgPool;

/// Verify the pool can execute a trivial query (`SELECT 1`).
pub async fn check_connectivity(pool: &PgPool) -> Result<(), PersistenceError> {
    let value = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .map_err(|_err| PersistenceError::Health)?;
    if value == 1 {
        Ok(())
    } else {
        Err(PersistenceError::Health)
    }
}

/// Verify Phase-1C foundation schema `tx4_infra` exists (schema readiness).
pub async fn check_schema_foundation(pool: &PgPool) -> Result<(), PersistenceError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'tx4_infra')",
    )
    .fetch_one(pool)
    .await
    .map_err(|_err| PersistenceError::Health)?;
    if exists {
        Ok(())
    } else {
        Err(PersistenceError::Health)
    }
}
