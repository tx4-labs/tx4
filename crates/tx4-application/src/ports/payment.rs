//! Payment provider port (IA §12) — Phase-2C: interface only; Mock lives in adapters.

use tx4_domain::Money;

use crate::ApplicationError;

/// Opaque provider-side payment reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderRef(String);

impl ProviderRef {
    pub fn new(raw: impl Into<String>) -> Result<Self, ApplicationError> {
        let value = raw.into();
        if value.trim().is_empty() {
            return Err(ApplicationError::validation(
                "provider_ref must be non-empty",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Intent to create a provider payment (generic; no provider-specific fields).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentIntent {
    pub amount: Money,
    pub provider_idempotency_key: String,
    pub description: Option<String>,
}

/// Observation returned by provider status queries (IA §12.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderObservation {
    Pending,
    Succeeded { provider_ref: ProviderRef },
    Failed { reason: String },
    Unknown { reason: String },
}

/// Generic payment provider port (IA §12.1). Implementations live in adapter crates.
pub trait PaymentProvider: Send + Sync {
    fn create_payment(
        &self,
        intent: &PaymentIntent,
    ) -> impl std::future::Future<Output = Result<ProviderRef, ApplicationError>> + Send;

    fn capture(
        &self,
        provider_ref: &ProviderRef,
    ) -> impl std::future::Future<Output = Result<(), ApplicationError>> + Send;

    fn refund(
        &self,
        provider_ref: &ProviderRef,
        money: &Money,
    ) -> impl std::future::Future<Output = Result<(), ApplicationError>> + Send;

    fn get_payment_status(
        &self,
        provider_ref: &ProviderRef,
    ) -> impl std::future::Future<Output = Result<ProviderObservation, ApplicationError>> + Send;
}
