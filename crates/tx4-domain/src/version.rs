//! Minimal monotonic version primitive.

use crate::DomainError;
use core::fmt;
use serde::{Deserialize, Serialize};

/// Non-negative monotonic version counter (u64).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Version(u64);

impl Version {
    /// Construct an explicit version value.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Version zero.
    pub const fn zero() -> Self {
        Self(0)
    }

    pub fn get(self) -> u64 {
        self.0
    }

    /// Checked successor.
    pub fn checked_next(self) -> Result<Self, DomainError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(DomainError::InvalidVersion)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_and_ordering() {
        assert_eq!(Version::new(1), Version::new(1));
        assert!(Version::new(1) < Version::new(2));
        assert!(Version::zero() < Version::new(1));
    }

    #[test]
    fn next_and_overflow() {
        assert_eq!(Version::new(7).checked_next().unwrap(), Version::new(8));
        assert_eq!(
            Version::new(u64::MAX).checked_next().unwrap_err(),
            DomainError::InvalidVersion
        );
    }
}
