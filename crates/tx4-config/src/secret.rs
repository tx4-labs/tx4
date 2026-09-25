//! Secret value wrapper with redacted Debug/Display.

use core::fmt;

/// Opaque secret string. Never prints the underlying value via Debug or Display.
#[derive(Clone)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Intentionally explicit accessor for trusted call sites.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(REDACTED)")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("REDACTED")
    }
}

impl PartialEq for SecretString {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for SecretString {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_debug_and_display() {
        let s = SecretString::new("super-secret-token");
        assert_eq!(format!("{s:?}"), "SecretString(REDACTED)");
        assert_eq!(format!("{s}"), "REDACTED");
        assert_eq!(s.expose(), "super-secret-token");
    }
}
