//! Persistence-layer errors (infrastructure only).
//!
//! Messages MUST NOT include database URLs or credentials.

use core::fmt;

/// Failures from PostgreSQL connectivity, pooling, migrations, or health checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PersistenceError {
    /// Failed to establish or configure the connection pool.
    Connect,
    /// Failed to acquire a connection from the pool.
    Pool,
    /// Ordered migration execution failed.
    Migrate,
    /// Connectivity/health probe failed.
    Health,
    /// Unexpected database query failure in the persistence foundation.
    Query,
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => write!(f, "failed to connect to PostgreSQL"),
            Self::Pool => write!(f, "failed to acquire a PostgreSQL pool connection"),
            Self::Migrate => write!(f, "database migration failed"),
            Self::Health => write!(f, "database health check failed"),
            Self::Query => write!(f, "database query failed"),
        }
    }
}

impl std::error::Error for PersistenceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_has_no_secret_placeholders() {
        for err in [
            PersistenceError::Connect,
            PersistenceError::Pool,
            PersistenceError::Migrate,
            PersistenceError::Health,
            PersistenceError::Query,
        ] {
            let s = err.to_string();
            assert!(!s.contains("postgres://"));
            assert!(!s.contains("password"));
            assert!(!s.contains("DATABASE_URL"));
        }
    }
}
