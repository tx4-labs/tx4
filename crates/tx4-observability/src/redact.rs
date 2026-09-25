//! Sensitive-field redaction helpers for application logging.

/// Field names that must never be emitted as structured log values.
pub const FORBIDDEN_LOG_KEYS: &[&str] = &[
    "password",
    "passwd",
    "secret",
    "token",
    "api_key",
    "apikey",
    "authorization",
    "database_url",
    "database_password",
    "webhook_secret",
    "provider_secret",
    "private_key",
    "access_key",
    "client_secret",
];

/// Returns true when `key` must not be logged as a sensitive value.
pub fn is_forbidden_log_key(key: &str) -> bool {
    let normalized = key.trim().to_ascii_lowercase().replace('-', "_");
    FORBIDDEN_LOG_KEYS
        .iter()
        .any(|forbidden| normalized == *forbidden || normalized.contains(forbidden))
}

/// Replace a sensitive value with a constant redaction marker.
pub fn redact_value(_value: &str) -> &'static str {
    "REDACTED"
}
