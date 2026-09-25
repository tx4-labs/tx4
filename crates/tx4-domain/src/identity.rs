//! Strongly typed opaque identity values.
//!
//! Representation is an opaque non-empty string. UUID selection remains OPEN;
//! this crate does not freeze an ID generation technology.

use crate::DomainError;
use core::fmt;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

macro_rules! opaque_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Construct from a non-empty trimmed string.
            pub fn new(raw: impl AsRef<str>) -> Result<Self, DomainError> {
                let value = raw.as_ref().trim();
                if value.is_empty() {
                    return Err(DomainError::InvalidIdentity($label));
                }
                Ok(Self(value.to_owned()))
            }

            /// Borrow the opaque identity string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct IdVisitor;

                impl Visitor<'_> for IdVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str(concat!("a non-empty ", $label))
                    }

                    fn visit_str<E>(self, value: &str) -> Result<$name, E>
                    where
                        E: de::Error,
                    {
                        $name::new(value).map_err(de::Error::custom)
                    }
                }

                deserializer.deserialize_str(IdVisitor)
            }
        }
    };
}

opaque_id!(TenantId, "tenant_id");
opaque_id!(OperationId, "operation_id");
opaque_id!(TransactionId, "transaction_id");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_and_inequality() {
        let a = TenantId::new("t-1").unwrap();
        let b = TenantId::new("t-1").unwrap();
        let c = TenantId::new("t-2").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn rejects_empty() {
        assert!(TenantId::new("").is_err());
        assert!(TenantId::new("   ").is_err());
        assert!(OperationId::new("").is_err());
        assert!(TransactionId::new("").is_err());
    }

    #[test]
    fn types_are_not_interchangeable() {
        let tenant = TenantId::new("same").unwrap();
        let operation = OperationId::new("same").unwrap();
        assert_eq!(tenant.as_str(), operation.as_str());
        assert_ne!(
            std::any::type_name::<TenantId>(),
            std::any::type_name::<OperationId>()
        );
    }

    #[test]
    fn trims_whitespace() {
        let id = TransactionId::new("  tx-9  ").unwrap();
        assert_eq!(id.as_str(), "tx-9");
    }
}
