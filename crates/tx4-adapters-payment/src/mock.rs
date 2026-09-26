//! Deterministic Mock payment provider (IA §12.2 CORE NOW). No network I/O.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tx4_application::{
    ApplicationError, PaymentIntent, PaymentProvider, ProviderObservation, ProviderRef,
};
use tx4_domain::Money;

/// Controllable create_payment outcome for tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MockPaymentOutcome {
    Succeed,
    Fail,
    UnknownTimeout,
}

#[derive(Clone, Debug)]
struct MockState {
    create_outcome: MockPaymentOutcome,
    /// Stable refs keyed by provider_idempotency_key (deterministic reuse).
    by_idempotency: HashMap<String, ProviderRef>,
    status: HashMap<String, ProviderObservation>,
}

/// In-memory Mock PaymentProvider — offline and deterministic.
#[derive(Clone, Debug)]
pub struct MockPaymentProvider {
    inner: Arc<Mutex<MockState>>,
}

impl MockPaymentProvider {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockState {
                create_outcome: MockPaymentOutcome::Succeed,
                by_idempotency: HashMap::new(),
                status: HashMap::new(),
            })),
        }
    }

    pub fn set_create_outcome(&self, outcome: MockPaymentOutcome) {
        let mut g = self.inner.lock().expect("mock lock");
        g.create_outcome = outcome;
    }
}

impl Default for MockPaymentProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PaymentProvider for MockPaymentProvider {
    async fn create_payment(
        &self,
        intent: &PaymentIntent,
    ) -> Result<ProviderRef, ApplicationError> {
        let mut g = self.inner.lock().expect("mock lock");
        if let Some(existing) = g.by_idempotency.get(&intent.provider_idempotency_key) {
            return Ok(existing.clone());
        }
        match g.create_outcome {
            MockPaymentOutcome::UnknownTimeout => {
                return Err(ApplicationError::timeout(
                    "mock provider timeout (uncertain outcome)",
                ));
            }
            MockPaymentOutcome::Fail => {
                return Err(ApplicationError::business_rejection(
                    "mock provider declined payment",
                ));
            }
            MockPaymentOutcome::Succeed => {}
        }
        let pref = ProviderRef::new(format!("mock:{}", intent.provider_idempotency_key))?;
        g.by_idempotency
            .insert(intent.provider_idempotency_key.clone(), pref.clone());
        g.status.insert(
            pref.as_str().to_owned(),
            ProviderObservation::Succeeded {
                provider_ref: pref.clone(),
            },
        );
        Ok(pref)
    }

    async fn capture(&self, provider_ref: &ProviderRef) -> Result<(), ApplicationError> {
        let g = self.inner.lock().expect("mock lock");
        if !g.status.contains_key(provider_ref.as_str()) {
            return Err(ApplicationError::not_found("mock provider_ref unknown"));
        }
        Ok(())
    }

    async fn refund(
        &self,
        provider_ref: &ProviderRef,
        _money: &Money,
    ) -> Result<(), ApplicationError> {
        let g = self.inner.lock().expect("mock lock");
        if !g.status.contains_key(provider_ref.as_str()) {
            return Err(ApplicationError::not_found("mock provider_ref unknown"));
        }
        Ok(())
    }

    async fn get_payment_status(
        &self,
        provider_ref: &ProviderRef,
    ) -> Result<ProviderObservation, ApplicationError> {
        let g = self.inner.lock().expect("mock lock");
        g.status
            .get(provider_ref.as_str())
            .cloned()
            .ok_or_else(|| ApplicationError::not_found("mock provider_ref unknown"))
    }
}
