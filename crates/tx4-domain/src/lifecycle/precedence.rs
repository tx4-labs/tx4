//! Concurrent conflict resolution — precedence and tie-break (ADR-007 §9).

use super::state::TransactionState;
use super::transition::{is_eligible, LifecycleContext};
use crate::identity::OperationId;
use crate::DomainError;

/// One eligible primary-transition candidate in a conflict set (ADR-007 §9.2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictCandidate {
    pub operation_id: OperationId,
    pub target: TransactionState,
    pub context: LifecycleContext,
}

/// Select the single durable winner among distinct eligible candidates (ADR-007 §9.3–§9.4).
///
/// Candidates that are not matrix-eligible under `current` + their context are ignored.
/// If none remain eligible → error. Exactly one winner is returned.
pub fn select_conflict_winner(
    current: TransactionState,
    candidates: &[ConflictCandidate],
) -> Result<&ConflictCandidate, DomainError> {
    let eligible: Vec<&ConflictCandidate> = candidates
        .iter()
        .filter(|c| is_eligible(current, c.target, &c.context))
        .collect();

    if eligible.is_empty() {
        return Err(DomainError::NoEligibleConflictCandidate);
    }

    let mut best = eligible[0];
    for cand in eligible.iter().skip(1) {
        best = prefer(best, cand)?;
    }
    Ok(best)
}

fn prefer<'a>(
    a: &'a ConflictCandidate,
    b: &'a ConflictCandidate,
) -> Result<&'a ConflictCandidate, DomainError> {
    let pa = a.target.precedence_class();
    let pb = b.target.precedence_class();
    match pa.cmp(&pb) {
        core::cmp::Ordering::Greater => Ok(a),
        core::cmp::Ordering::Less => Ok(b),
        core::cmp::Ordering::Equal => {
            // Same precedence class: lexicographically smaller operation_id wins (§9.4).
            match a.operation_id.as_str().cmp(b.operation_id.as_str()) {
                core::cmp::Ordering::Less => Ok(a),
                core::cmp::Ordering::Greater => Ok(b),
                core::cmp::Ordering::Equal => {
                    // Same logical transition identity → not a race; treat as equivalent.
                    if a.target == b.target {
                        Ok(a)
                    } else {
                        Err(DomainError::AmbiguousConflictTie)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::OperationId;

    fn op(s: &str) -> OperationId {
        OperationId::new(s).unwrap()
    }

    fn cand(id: &str, target: TransactionState, ctx: LifecycleContext) -> ConflictCandidate {
        ConflictCandidate {
            operation_id: op(id),
            target,
            context: ctx,
        }
    }

    #[test]
    fn race_completed_beats_cancelled() {
        let ctx = LifecycleContext::empty();
        let cancel_ctx = LifecycleContext {
            cancel_during_processing_decision: true,
            compensating_intent_recorded: true,
            ..LifecycleContext::empty()
        };
        let candidates = [
            cand("op-b", TransactionState::Cancelled, cancel_ctx),
            cand("op-a", TransactionState::Completed, ctx),
        ];
        let w = select_conflict_winner(TransactionState::Processing, &candidates).unwrap();
        assert_eq!(w.target, TransactionState::Completed);
    }

    #[test]
    fn race_paid_beats_expired() {
        let paid_ctx = LifecycleContext {
            known_success_evidence: true,
            ..LifecycleContext::empty()
        };
        let candidates = [
            cand("z", TransactionState::Expired, LifecycleContext::empty()),
            cand("a", TransactionState::Paid, paid_ctx),
        ];
        let w = select_conflict_winner(TransactionState::Pending, &candidates).unwrap();
        assert_eq!(w.target, TransactionState::Paid);
    }

    #[test]
    fn race_paid_beats_cancelled() {
        let paid_ctx = LifecycleContext {
            known_success_evidence: true,
            ..LifecycleContext::empty()
        };
        let candidates = [
            cand("c1", TransactionState::Cancelled, LifecycleContext::empty()),
            cand("p1", TransactionState::Paid, paid_ctx),
        ];
        let w = select_conflict_winner(TransactionState::Pending, &candidates).unwrap();
        assert_eq!(w.target, TransactionState::Paid);
    }

    #[test]
    fn race_completed_beats_failed() {
        let fail_ctx = LifecycleContext {
            compensating_intent_recorded: true,
            ..LifecycleContext::empty()
        };
        let candidates = [
            cand("f", TransactionState::Failed, fail_ctx),
            cand("c", TransactionState::Completed, LifecycleContext::empty()),
        ];
        let w = select_conflict_winner(TransactionState::Processing, &candidates).unwrap();
        assert_eq!(w.target, TransactionState::Completed);
    }

    #[test]
    fn race_failed_beats_cancelled() {
        let candidates = [
            cand(
                "cancel",
                TransactionState::Cancelled,
                LifecycleContext::empty(),
            ),
            cand("fail", TransactionState::Failed, LifecycleContext::empty()),
        ];
        let w = select_conflict_winner(TransactionState::Pending, &candidates).unwrap();
        assert_eq!(w.target, TransactionState::Failed);
    }

    #[test]
    fn race_cancelled_beats_expired() {
        let candidates = [
            cand("exp", TransactionState::Expired, LifecycleContext::empty()),
            cand(
                "can",
                TransactionState::Cancelled,
                LifecycleContext::empty(),
            ),
        ];
        let w = select_conflict_winner(TransactionState::Pending, &candidates).unwrap();
        assert_eq!(w.target, TransactionState::Cancelled);
    }

    #[test]
    fn tie_break_lexicographic_smaller_wins() {
        let ctx = LifecycleContext {
            known_success_evidence: true,
            ..LifecycleContext::empty()
        };
        // Same target PAID — same precedence — smaller op id wins.
        let candidates = [
            cand("op-b", TransactionState::Paid, ctx),
            cand("op-a", TransactionState::Paid, ctx),
        ];
        let w = select_conflict_winner(TransactionState::Pending, &candidates).unwrap();
        assert_eq!(w.operation_id.as_str(), "op-a");
    }
}
