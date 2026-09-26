//! TX4 payment provider adapters.
//!
//! Phase-2C: deterministic Mock adapter only. Real providers remain Phase 4.

#![forbid(unsafe_code)]

mod mock;

pub use mock::{MockPaymentOutcome, MockPaymentProvider};

pub use tx4_application;
pub use tx4_domain;

/// Marker that the payment adapters crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tx4_application::{PaymentIntent, PaymentProvider, ProviderObservation};
    use tx4_domain::{CurrencyId, Money};

    #[test]
    fn crate_name_is_tx4_adapters_payment() {
        assert_eq!(crate_name(), "tx4-adapters-payment");
    }

    #[tokio::test]
    async fn mock_is_deterministic_and_offline() {
        let mock = MockPaymentProvider::new();
        mock.set_create_outcome(MockPaymentOutcome::Succeed)
            .unwrap();
        let intent = PaymentIntent {
            amount: Money::new(100, CurrencyId::new("IDR").unwrap()),
            provider_idempotency_key: "pik-1".into(),
            description: None,
        };
        let a = mock.create_payment(&intent).await.unwrap();
        let b = mock.create_payment(&intent).await.unwrap();
        assert_eq!(a, b);
        assert_eq!(
            mock.get_payment_status(&a).await.unwrap(),
            ProviderObservation::Succeeded {
                provider_ref: a.clone()
            }
        );
        mock.capture(&a).await.unwrap();
        mock.refund(&a, &intent.amount).await.unwrap();
    }

    #[tokio::test]
    async fn mock_can_simulate_uncertainty() {
        let mock = MockPaymentProvider::new();
        mock.set_create_outcome(MockPaymentOutcome::UnknownTimeout)
            .unwrap();
        let intent = PaymentIntent {
            amount: Money::new(50, CurrencyId::new("USD").unwrap()),
            provider_idempotency_key: "pik-u".into(),
            description: Some("t".into()),
        };
        let err = mock.create_payment(&intent).await.unwrap_err();
        assert!(err.is_uncertainty_or_timeout());
    }
}
