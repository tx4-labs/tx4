//! PostgreSQL connection pool foundation.

use crate::error::PersistenceError;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tx4_config::Config;

/// Create a bounded asynchronous PostgreSQL connection pool from process config.
pub async fn connect_pool(config: &Config) -> Result<PgPool, PersistenceError> {
    PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .min_connections(config.database_min_connections)
        .acquire_timeout(config.database_acquire_timeout)
        // Do not log or surface the URL; map any driver failure to a generic error.
        .connect(config.database_url.expose())
        .await
        .map_err(|_err| PersistenceError::Connect)
}

/// Gracefully close all pool connections.
pub async fn close_pool(pool: &PgPool) {
    pool.close().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::time::Duration;

    fn test_config(url: &str) -> Config {
        let mut m = BTreeMap::new();
        m.insert("DATABASE_URL".to_owned(), url.to_owned());
        m.insert("TX4_DATABASE_MAX_CONNECTIONS".to_owned(), "2".to_owned());
        m.insert("TX4_DATABASE_MIN_CONNECTIONS".to_owned(), "0".to_owned());
        m.insert(
            "TX4_DATABASE_ACQUIRE_TIMEOUT_SECS".to_owned(),
            "1".to_owned(),
        );
        Config::from_map(&m).expect("valid test config")
    }

    #[tokio::test]
    async fn connect_failure_is_generic() {
        let cfg = test_config("postgres://tx4:secret@127.0.0.1:1/does_not_exist");
        let err = connect_pool(&cfg).await.unwrap_err();
        assert_eq!(err, PersistenceError::Connect);
        assert!(!err.to_string().contains("secret"));
        assert!(!err.to_string().contains("postgres://"));
        let _ = Duration::from_secs(1);
    }
}
