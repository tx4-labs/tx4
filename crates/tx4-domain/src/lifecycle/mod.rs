//! Pure transaction lifecycle domain logic (ADR-007).
//!
//! Encodes primary states, transition matrix, uncertainty disposition,
//! compensating-intent fail-closed rules, and conflict precedence/tie-break.
//! No persistence, HTTP, or provider I/O.

mod disposition;
mod precedence;
mod state;
mod transition;

pub use disposition::{reject_timeout_as_failed, OutcomeDisposition};
pub use precedence::{select_conflict_winner, ConflictCandidate};
pub use state::TransactionState;
pub use transition::{
    evaluate, is_eligible, matrix_edge, requires_compensating_intent, LifecycleCommand,
    LifecycleContext, LifecycleResult, MatrixEdge, TerminalFailureReason,
};
