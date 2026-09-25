//! Infrastructure port namespaces (IA §5.2).

pub mod payment;
pub mod persistence;

pub use persistence::TransactionRepository;
