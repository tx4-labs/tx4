//! Explicit currency identity (ADR-006 catalog concept — catalog contents OPEN).

use crate::DomainError;
use core::fmt;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Currency catalog identity: uppercase code string (for example `IDR`, `USD`).
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CurrencyId(String);

impl CurrencyId {
    /// Parse a currency code: trim, require non-empty A–Z / 0–9 / `_`, store uppercase.
    pub fn new(raw: impl AsRef<str>) -> Result<Self, DomainError> {
        let trimmed = raw.as_ref().trim();
        if trimmed.is_empty() || trimmed.len() > 16 {
            return Err(DomainError::InvalidCurrency);
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(DomainError::InvalidCurrency);
        }
        Ok(Self(trimmed.to_ascii_uppercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CurrencyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CurrencyId").field(&self.0).finish()
    }
}

impl fmt::Display for CurrencyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for CurrencyId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CurrencyId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CurrencyVisitor;

        impl Visitor<'_> for CurrencyVisitor {
            type Value = CurrencyId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a currency identity string")
            }

            fn visit_str<E>(self, value: &str) -> Result<CurrencyId, E>
            where
                E: de::Error,
            {
                CurrencyId::new(value).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(CurrencyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercases_and_accepts_common_codes() {
        assert_eq!(CurrencyId::new("idr").unwrap().as_str(), "IDR");
        assert_eq!(CurrencyId::new("USD").unwrap().as_str(), "USD");
        assert_eq!(CurrencyId::new("eur").unwrap().as_str(), "EUR");
    }

    #[test]
    fn rejects_invalid() {
        assert!(CurrencyId::new("").is_err());
        assert!(CurrencyId::new("us d").is_err());
        assert!(CurrencyId::new("$$$").is_err());
    }

    #[test]
    fn mismatch_via_money_other_module() {
        // Currency identity inequality is the primitive mismatch signal.
        assert_ne!(
            CurrencyId::new("IDR").unwrap(),
            CurrencyId::new("USD").unwrap()
        );
    }
}
