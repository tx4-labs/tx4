//! Infrastructure port namespaces (IA §5.2).

pub mod operational;
pub mod payment;
pub mod persistence;

pub use operational::{
    ClaimedOutboxJob, IdempotencyBeginOutcome, IdempotencyBeginRequest, IdempotencyRepository,
    IdempotencyReservation, IdempotencyStatus, OutboxJob, OutboxRepository, OutboxStatus,
    PaymentAttempt, PaymentAttemptRepository, PaymentAttemptStatus,
};
pub use payment::{PaymentIntent, PaymentProvider, ProviderObservation, ProviderRef};
pub use persistence::TransactionRepository;
