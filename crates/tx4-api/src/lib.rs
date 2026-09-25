//! TX4 HTTP API boundary.
//!
//! Phase-1E: Axum router bootstrap with infrastructure health/readiness only.
//! No business `/v1` routes.

#![forbid(unsafe_code)]

mod error;
mod routes;
mod state;

use axum::Router;

pub use error::ApiError;
pub use state::AppState;
pub use tx4_application;

/// Build the HTTP router for the TX4 server foundation.
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(routes::health_routes())
        .with_state(state)
}

/// Marker that the API crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use std::collections::BTreeMap;
    use tower::ServiceExt;
    use tx4_config::Config;
    use tx4_persistence::{close_pool, connect_pool};

    fn config_for_url(url: &str) -> Config {
        let mut m = BTreeMap::new();
        m.insert("DATABASE_URL".to_owned(), url.to_owned());
        m.insert("TX4_DATABASE_MAX_CONNECTIONS".to_owned(), "2".to_owned());
        m.insert(
            "TX4_DATABASE_ACQUIRE_TIMEOUT_SECS".to_owned(),
            "2".to_owned(),
        );
        Config::from_map(&m).expect("valid config")
    }

    #[test]
    fn crate_name_is_tx4_api() {
        assert_eq!(crate_name(), "tx4-api");
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL against a real PostgreSQL database"]
    async fn health_and_ready_with_postgres() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let cfg = config_for_url(&url);
        let pool = connect_pool(&cfg).await.expect("connect");
        tx4_persistence::run_migrations(&pool)
            .await
            .expect("migrate");
        let app = router(AppState::new(cfg, pool.clone()));

        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        let health_body = String::from_utf8(
            health
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        assert!(health_body.contains("\"status\":\"ok\""));
        assert!(!health_body.contains("postgres://"));
        assert!(!health_body.contains("password"));

        let ready = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ready.status(), StatusCode::OK);
        let ready_body = String::from_utf8(
            ready
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        assert!(ready_body.contains("\"status\":\"ready\""));
        assert!(!ready_body.contains("postgres://"));

        close_pool(&pool).await;
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL against a real PostgreSQL database"]
    async fn ready_fails_after_pool_close_while_health_stays_ok() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let cfg = config_for_url(&url);
        let pool = connect_pool(&cfg).await.expect("connect");
        tx4_persistence::run_migrations(&pool)
            .await
            .expect("migrate");
        let app = router(AppState::new(cfg, pool.clone()));
        close_pool(&pool).await;

        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);

        let ready = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ready.status(), StatusCode::SERVICE_UNAVAILABLE);
        let ready_body = String::from_utf8(
            ready
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        assert!(ready_body.contains("\"status\":\"not_ready\""));
        assert!(!ready_body.contains("postgres://"));
        assert!(!ready_body.contains("secret"));
    }
}
