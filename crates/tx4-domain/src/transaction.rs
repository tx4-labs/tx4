//! Durable transaction aggregate (current-state representation for persistence).

use crate::currency::CurrencyId;
use crate::error::DomainError;
use crate::identity::{TenantId, TransactionId};
use crate::lifecycle::{
    evaluate, LifecycleCommand, LifecycleContext, LifecycleResult, TransactionState,
};
use crate::money::Money;
use crate::version::Version;

/// Authoritative TX4 transaction aggregate (mutable current primary state).
///
/// Timestamps use Unix microseconds (UTC) as explicit domain inputs — no wall-clock reads here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transaction {
    id: TransactionId,
    tenant_id: TenantId,
    primary_state: TransactionState,
    version: Version,
    money: Option<Money>,
    expires_at_unix_micros: Option<i64>,
    attempt_budget: Option<i32>,
    created_at_unix_micros: i64,
    updated_at_unix_micros: i64,
}

impl Transaction {
    /// Construct a new aggregate in `CREATED` at version 0.
    pub fn new_created(
        id: TransactionId,
        tenant_id: TenantId,
        money: Option<Money>,
        expires_at_unix_micros: Option<i64>,
        attempt_budget: Option<i32>,
        created_at_unix_micros: i64,
    ) -> Result<Self, DomainError> {
        if let Some(budget) = attempt_budget {
            if budget < 1 {
                return Err(DomainError::InvalidAttemptBudget);
            }
        }
        Ok(Self {
            id,
            tenant_id,
            primary_state: TransactionState::Created,
            version: Version::zero(),
            money,
            expires_at_unix_micros,
            attempt_budget,
            created_at_unix_micros,
            updated_at_unix_micros: created_at_unix_micros,
        })
    }

    /// Reconstruct from durable storage (fail-closed on invalid combinations).
    #[allow(clippy::too_many_arguments)]
    pub fn reconstitute(
        id: TransactionId,
        tenant_id: TenantId,
        primary_state: TransactionState,
        version: Version,
        money: Option<Money>,
        expires_at_unix_micros: Option<i64>,
        attempt_budget: Option<i32>,
        created_at_unix_micros: i64,
        updated_at_unix_micros: i64,
    ) -> Result<Self, DomainError> {
        if let Some(budget) = attempt_budget {
            if budget < 1 {
                return Err(DomainError::InvalidAttemptBudget);
            }
        }
        Ok(Self {
            id,
            tenant_id,
            primary_state,
            version,
            money,
            expires_at_unix_micros,
            attempt_budget,
            created_at_unix_micros,
            updated_at_unix_micros,
        })
    }

    pub fn id(&self) -> &TransactionId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn primary_state(&self) -> TransactionState {
        self.primary_state
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn money(&self) -> Option<&Money> {
        self.money.as_ref()
    }

    pub fn amount_atomic(&self) -> Option<i64> {
        self.money.as_ref().map(Money::amount)
    }

    pub fn currency_id(&self) -> Option<&CurrencyId> {
        self.money.as_ref().map(Money::currency)
    }

    pub fn expires_at_unix_micros(&self) -> Option<i64> {
        self.expires_at_unix_micros
    }

    pub fn attempt_budget(&self) -> Option<i32> {
        self.attempt_budget
    }

    pub fn created_at_unix_micros(&self) -> i64 {
        self.created_at_unix_micros
    }

    pub fn updated_at_unix_micros(&self) -> i64 {
        self.updated_at_unix_micros
    }

    /// Apply a lifecycle command and return the next aggregate with checked version bump.
    ///
    /// `updated_at_unix_micros` must be supplied explicitly (no internal clock).
    pub fn apply(
        &self,
        command: &LifecycleCommand,
        ctx: &LifecycleContext,
        updated_at_unix_micros: i64,
    ) -> Result<Self, DomainError> {
        let outcome = evaluate(self.primary_state, command, ctx)?;
        let next_state = match outcome {
            LifecycleResult::Transitioned { to, .. } => to,
            LifecycleResult::IdempotentNoOp { state } => state,
        };
        let next_version = match outcome {
            LifecycleResult::Transitioned { .. } => self.version.checked_next()?,
            LifecycleResult::IdempotentNoOp { .. } => self.version,
        };
        Ok(Self {
            id: self.id.clone(),
            tenant_id: self.tenant_id.clone(),
            primary_state: next_state,
            version: next_version,
            money: self.money.clone(),
            expires_at_unix_micros: self.expires_at_unix_micros,
            attempt_budget: self.attempt_budget,
            created_at_unix_micros: self.created_at_unix_micros,
            updated_at_unix_micros,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_open_pending_bumps_version() {
        let tx = Transaction::new_created(
            TransactionId::new("tx-1").unwrap(),
            TenantId::new("t-1").unwrap(),
            None,
            None,
            None,
            1_000_000,
        )
        .unwrap();
        let next = tx
            .apply(
                &LifecycleCommand::OpenPending,
                &LifecycleContext::empty(),
                2_000_000,
            )
            .unwrap();
        assert_eq!(next.primary_state(), TransactionState::Pending);
        assert_eq!(next.version(), Version::new(1));
        assert_eq!(next.updated_at_unix_micros(), 2_000_000);
    }
}
