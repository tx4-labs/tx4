//! TX4 domain primitives and transaction lifecycle foundation.
//!
//! Phase-1B: identity, money/currency (ADR-006), version, and domain errors.
//! Phase-1F: ADR-007 primary lifecycle pure logic (no persistence, HTTP, or providers).

#![forbid(unsafe_code)]

mod allocation;
mod currency;
mod error;
mod identity;
mod lifecycle;
mod money;
mod rounding;
mod transaction;
mod version;

pub use allocation::allocate;
pub use currency::CurrencyId;
pub use error::DomainError;
pub use identity::{OperationId, TenantId, TransactionId};
pub use lifecycle::{
    evaluate, is_eligible, matrix_edge, reject_timeout_as_failed, requires_compensating_intent,
    select_conflict_winner, ConflictCandidate, LifecycleCommand, LifecycleContext, LifecycleResult,
    MatrixEdge, OutcomeDisposition, TerminalFailureReason, TransactionState,
};
pub use money::Money;
pub use rounding::round_half_even_rational;
pub use transaction::Transaction;
pub use version::Version;

/// Marker that the domain crate is linked and compilable.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_is_tx4_domain() {
        assert_eq!(crate_name(), "tx4-domain");
    }
}

#[cfg(test)]
mod lifecycle_matrix_tests {
    use super::*;

    /// Exhaustive property: every matrix cell matches ADR-007 §6 legend intent.
    #[test]
    fn matrix_matches_adr_007_section_6() {
        use TransactionState::*;
        let cases: &[(TransactionState, TransactionState, MatrixEdge)] = &[
            // CREATED
            (Created, Pending, MatrixEdge::Allowed),
            (Created, Cancelled, MatrixEdge::Allowed),
            (Created, Expired, MatrixEdge::Allowed),
            (Created, Paid, MatrixEdge::Forbidden),
            (Created, Processing, MatrixEdge::Forbidden),
            (Created, Completed, MatrixEdge::Forbidden),
            (Created, Failed, MatrixEdge::Forbidden),
            // PENDING
            (Pending, Paid, MatrixEdge::Allowed),
            (Pending, Cancelled, MatrixEdge::Allowed),
            (Pending, Expired, MatrixEdge::Allowed),
            (Pending, Failed, MatrixEdge::Allowed),
            (Pending, Processing, MatrixEdge::Forbidden),
            (Pending, Completed, MatrixEdge::Forbidden),
            (Pending, Created, MatrixEdge::Forbidden),
            // PAID
            (Paid, Processing, MatrixEdge::Allowed),
            (Paid, Cancelled, MatrixEdge::Conditional),
            (Paid, Failed, MatrixEdge::Conditional),
            (Paid, Expired, MatrixEdge::Forbidden),
            (Paid, Pending, MatrixEdge::Forbidden),
            (Paid, Completed, MatrixEdge::Forbidden),
            // PROCESSING
            (Processing, Completed, MatrixEdge::Allowed),
            (Processing, Failed, MatrixEdge::Allowed),
            (Processing, Cancelled, MatrixEdge::Conditional),
            (Processing, Pending, MatrixEdge::Forbidden),
            (Processing, Paid, MatrixEdge::Forbidden),
            (Processing, Expired, MatrixEdge::Forbidden),
            // Terminals: no outbound primary edges
            (Completed, Pending, MatrixEdge::Forbidden),
            (Cancelled, Pending, MatrixEdge::Forbidden),
            (Expired, Pending, MatrixEdge::Forbidden),
            (Failed, Pending, MatrixEdge::Forbidden),
        ];
        for (from, to, expected) in cases {
            assert_eq!(matrix_edge(*from, *to), *expected, "{from} → {to}");
        }
    }

    #[test]
    fn self_edges_forbidden_as_new_transitions() {
        for s in TransactionState::ALL {
            assert_eq!(matrix_edge(s, s), MatrixEdge::Forbidden);
        }
    }

    #[test]
    fn invalid_transitions_never_silently_succeed() {
        let ctx = LifecycleContext::empty();
        for from in TransactionState::ALL {
            for to in TransactionState::ALL {
                if from == to {
                    continue;
                }
                if matrix_edge(from, to) != MatrixEdge::Forbidden {
                    continue;
                }
                // Pick a command targeting `to` that would be stale/forbidden from `from`.
                let cmd = command_toward(to);
                let result = evaluate(from, &cmd, &ctx);
                assert!(
                    result.is_err(),
                    "expected error for forbidden {from} → {to}, got {result:?}"
                );
            }
        }
    }

    fn command_toward(to: TransactionState) -> LifecycleCommand {
        match to {
            TransactionState::Pending => LifecycleCommand::OpenPending,
            TransactionState::Paid => LifecycleCommand::AcceptPaymentSuccess,
            TransactionState::Processing => LifecycleCommand::StartProcessing,
            TransactionState::Completed => LifecycleCommand::Complete,
            TransactionState::Cancelled => LifecycleCommand::Cancel,
            TransactionState::Expired => LifecycleCommand::Expire,
            TransactionState::Failed => LifecycleCommand::FailPending {
                reason: TerminalFailureReason::BusinessRejectedTerminal,
            },
            TransactionState::Created => LifecycleCommand::OpenPending,
        }
    }

    #[test]
    fn evaluate_is_deterministic() {
        let ctx = LifecycleContext {
            known_success_evidence: true,
            ..LifecycleContext::empty()
        };
        let a = evaluate(
            TransactionState::Pending,
            &LifecycleCommand::AcceptPaymentSuccess,
            &ctx,
        );
        let b = evaluate(
            TransactionState::Pending,
            &LifecycleCommand::AcceptPaymentSuccess,
            &ctx,
        );
        assert_eq!(a, b);
    }

    #[test]
    fn processing_failed_requires_compensating_intent() {
        assert_eq!(
            evaluate(
                TransactionState::Processing,
                &LifecycleCommand::FailDuringProcessing,
                &LifecycleContext::empty(),
            )
            .unwrap_err(),
            DomainError::CompensatingIntentRequired
        );
        let ctx = LifecycleContext {
            compensating_intent_recorded: true,
            ..LifecycleContext::empty()
        };
        assert_eq!(
            evaluate(
                TransactionState::Processing,
                &LifecycleCommand::FailDuringProcessing,
                &ctx,
            )
            .unwrap()
            .resulting_state(),
            TransactionState::Failed
        );
    }

    #[test]
    fn pending_fail_with_terminal_decision() {
        let r = evaluate(
            TransactionState::Pending,
            &LifecycleCommand::FailPending {
                reason: TerminalFailureReason::AttemptBudgetExhausted,
            },
            &LifecycleContext::empty(),
        )
        .unwrap();
        assert_eq!(r.resulting_state(), TransactionState::Failed);
    }

    #[test]
    fn expire_forbidden_after_payment() {
        for from in [TransactionState::Paid, TransactionState::Processing] {
            assert_eq!(
                evaluate(from, &LifecycleCommand::Expire, &LifecycleContext::empty()).unwrap_err(),
                DomainError::ForbiddenTransition
            );
        }
    }

    #[test]
    fn covered_races_match_adr_007_9_5() {
        let paid_ctx = LifecycleContext {
            known_success_evidence: true,
            ..LifecycleContext::empty()
        };
        let cancel_proc = LifecycleContext {
            cancel_during_processing_decision: true,
            compensating_intent_recorded: true,
            ..LifecycleContext::empty()
        };
        let fail_proc = LifecycleContext {
            compensating_intent_recorded: true,
            ..LifecycleContext::empty()
        };

        // PROCESSING→COMPLETED vs CANCELLED
        let c = [
            ConflictCandidate {
                operation_id: OperationId::new("1").unwrap(),
                target: TransactionState::Cancelled,
                context: cancel_proc,
            },
            ConflictCandidate {
                operation_id: OperationId::new("2").unwrap(),
                target: TransactionState::Completed,
                context: LifecycleContext::empty(),
            },
        ];
        assert_eq!(
            select_conflict_winner(TransactionState::Processing, &c)
                .unwrap()
                .target,
            TransactionState::Completed
        );

        // PENDING→PAID vs EXPIRED
        let c = [
            ConflictCandidate {
                operation_id: OperationId::new("e").unwrap(),
                target: TransactionState::Expired,
                context: LifecycleContext::empty(),
            },
            ConflictCandidate {
                operation_id: OperationId::new("p").unwrap(),
                target: TransactionState::Paid,
                context: paid_ctx,
            },
        ];
        assert_eq!(
            select_conflict_winner(TransactionState::Pending, &c)
                .unwrap()
                .target,
            TransactionState::Paid
        );

        // PENDING→FAILED vs CANCELLED
        let c = [
            ConflictCandidate {
                operation_id: OperationId::new("c").unwrap(),
                target: TransactionState::Cancelled,
                context: LifecycleContext::empty(),
            },
            ConflictCandidate {
                operation_id: OperationId::new("f").unwrap(),
                target: TransactionState::Failed,
                context: LifecycleContext::empty(),
            },
        ];
        assert_eq!(
            select_conflict_winner(TransactionState::Pending, &c)
                .unwrap()
                .target,
            TransactionState::Failed
        );

        // PROCESSING→COMPLETED vs FAILED
        let c = [
            ConflictCandidate {
                operation_id: OperationId::new("f").unwrap(),
                target: TransactionState::Failed,
                context: fail_proc,
            },
            ConflictCandidate {
                operation_id: OperationId::new("c").unwrap(),
                target: TransactionState::Completed,
                context: LifecycleContext::empty(),
            },
        ];
        assert_eq!(
            select_conflict_winner(TransactionState::Processing, &c)
                .unwrap()
                .target,
            TransactionState::Completed
        );
    }
}
