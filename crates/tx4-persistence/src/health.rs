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
