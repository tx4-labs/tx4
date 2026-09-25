//! Money atomic units + currency (ADR-006).

use crate::currency::CurrencyId;
use crate::error::DomainError;
use core::fmt;
use serde::de::{self, MapAccess, Visitor};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

/// Exact monetary value: signed atomic integer + explicit currency.
#[derive(Clone, Eq, PartialEq)]
pub struct Money {
    amount: i64,
    currency: CurrencyId,
}

impl Money {
    /// Construct money. Amount may be negative (ADR-006 allows signed atomic units).
    pub fn new(amount: i64, currency: CurrencyId) -> Self {
        Self { amount, currency }
    }

    pub fn amount(&self) -> i64 {
        self.amount
    }

    pub fn currency(&self) -> &CurrencyId {
        &self.currency
    }

    /// Checked addition; currency must match; overflow fails closed.
    pub fn checked_add(&self, other: &Money) -> Result<Money, DomainError> {
        self.require_same_currency(other)?;
        let sum = (self.amount as i128)
            .checked_add(other.amount as i128)
            .ok_or(DomainError::MoneyOverflow)?;
        let amount = i64::try_from(sum).map_err(|_| DomainError::MoneyOverflow)?;
        Ok(Money::new(amount, self.currency.clone()))
    }

    /// Checked subtraction; currency must match; overflow fails closed.
    pub fn checked_sub(&self, other: &Money) -> Result<Money, DomainError> {
        self.require_same_currency(other)?;
        let diff = (self.amount as i128)
            .checked_sub(other.amount as i128)
            .ok_or(DomainError::MoneyOverflow)?;
        let amount = i64::try_from(diff).map_err(|_| DomainError::MoneyOverflow)?;
        Ok(Money::new(amount, self.currency.clone()))
    }

    fn require_same_currency(&self, other: &Money) -> Result<(), DomainError> {
        if self.currency != other.currency {
            return Err(DomainError::CurrencyMismatch);
        }
        Ok(())
    }
}

impl fmt::Debug for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Money")
            .field("amount", &self.amount)
            .field("currency", &self.currency)
            .finish()
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

impl Serialize for Money {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Money", 2)?;
        // ADR-006: JSON amount is decimal-digit string, not JSON number.
        state.serialize_field("amount", &self.amount.to_string())?;
        state.serialize_field("currency", &self.currency)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Amount,
            Currency,
        }

        struct MoneyVisitor;

        impl<'de> Visitor<'de> for MoneyVisitor {
            type Value = Money;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Money { amount: string, currency: string }")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Money, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut amount: Option<i64> = None;
                let mut currency: Option<CurrencyId> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Amount => {
                            if amount.is_some() {
                                return Err(de::Error::duplicate_field("amount"));
                            }
                            let raw: String = map.next_value()?;
                            let parsed = parse_amount_string(&raw).map_err(de::Error::custom)?;
                            amount = Some(parsed);
                        }
                        Field::Currency => {
                            if currency.is_some() {
                                return Err(de::Error::duplicate_field("currency"));
                            }
                            currency = Some(map.next_value()?);
                        }
                    }
                }
                let amount = amount.ok_or_else(|| de::Error::missing_field("amount"))?;
                let currency = currency.ok_or_else(|| de::Error::missing_field("currency"))?;
                Ok(Money::new(amount, currency))
            }
        }

        const FIELDS: &[&str] = &["amount", "currency"];
        deserializer.deserialize_struct("Money", FIELDS, MoneyVisitor)
    }
}

fn parse_amount_string(raw: &str) -> Result<i64, DomainError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(DomainError::InvalidMoneyAmount);
    }
    // Reject floats / exponents / separators.
    if s.contains('.') || s.contains('e') || s.contains('E') || s.contains(',') {
        return Err(DomainError::InvalidMoneyAmount);
    }
    s.parse::<i64>()
        .map_err(|_| DomainError::InvalidMoneyAmount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CurrencyId;

    fn idr(amount: i64) -> Money {
        Money::new(amount, CurrencyId::new("IDR").unwrap())
    }

    fn usd(amount: i64) -> Money {
        Money::new(amount, CurrencyId::new("USD").unwrap())
    }

    #[test]
    fn construction_and_equality() {
        assert_eq!(idr(100), idr(100));
        assert_ne!(idr(100), idr(101));
        assert_ne!(idr(100), usd(100));
    }

    #[test]
    fn add_sub_same_currency() {
        assert_eq!(idr(10).checked_add(&idr(5)).unwrap(), idr(15));
        assert_eq!(idr(10).checked_sub(&idr(3)).unwrap(), idr(7));
        assert_eq!(idr(5).checked_sub(&idr(10)).unwrap(), idr(-5));
    }

    #[test]
    fn currency_mismatch() {
        assert_eq!(
            idr(1).checked_add(&usd(1)).unwrap_err(),
            DomainError::CurrencyMismatch
        );
    }

    #[test]
    fn overflow_fails_closed() {
        let a = idr(i64::MAX);
        assert_eq!(
            a.checked_add(&idr(1)).unwrap_err(),
            DomainError::MoneyOverflow
        );
        let b = idr(i64::MIN);
        assert_eq!(
            b.checked_sub(&idr(1)).unwrap_err(),
            DomainError::MoneyOverflow
        );
    }

    #[test]
    fn serde_amount_is_string() {
        let m = idr(1500);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"amount":"1500","currency":"IDR"}"#);
        let back: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn serde_rejects_json_number_amount() {
        let err = serde_json::from_str::<Money>(r#"{"amount":1500,"currency":"IDR"}"#);
        assert!(err.is_err());
    }

    #[test]
    fn serde_rejects_float_string() {
        let err = serde_json::from_str::<Money>(r#"{"amount":"12.5","currency":"IDR"}"#);
        assert!(err.is_err());
    }
}
