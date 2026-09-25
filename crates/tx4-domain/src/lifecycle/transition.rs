//! Lifecycle transition commands, matrix, and pure evaluation (ADR-007 §6–§8, §12–§14).

use super::state::TransactionState;
use crate::DomainError;

/// Reason categories for `PENDING` → `FAILED` Terminal Failure Decision (ADR-007 §7.3).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TerminalFailureReason {
    BusinessRejectedTerminal,
    AttemptBudgetExhausted,
    DomainAbandonedTerminal,
}

/// Explicit inputs required to evaluate conditional / evidence-gated transitions.
///
/// Temporal clocks, provider I/O, and durability are **not** obtained here — callers
/// supply only domain-visible facts (ADR-007 Phase-1F pure domain boundary).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LifecycleContext {
    /// Known-success evidence path accepted for `PENDING` → `PAID` (ADR-007 §7.2.4).
    pub known_success_evidence: bool,
    /// Explicit cancel-after-payment decision (C1).
    pub cancel_after_payment_decision: bool,
    /// Explicit cancel-during-processing decision (C3).
    pub cancel_during_processing_decision: bool,
    /// Documented terminal business inability after payment acceptance (C2).
    pub terminal_business_inability_after_payment: bool,
    /// Compensating-obligation intent recorded for post-acceptance exits (ADR-007 §6.1).
    pub compensating_intent_recorded: bool,
}

impl LifecycleContext {
    pub const fn empty() -> Self {
        Self {
            known_success_evidence: false,
            cancel_after_payment_decision: false,
            cancel_during_processing_decision: false,
            terminal_business_inability_after_payment: false,
            compensating_intent_recorded: false,
        }
    }
}

/// Pure-domain lifecycle command (not an HTTP/API DTO).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LifecycleCommand {
    /// `CREATED` → `PENDING`
    OpenPending,
    /// `PENDING` → `PAID`
    AcceptPaymentSuccess,
    /// `PAID` → `PROCESSING`
    StartProcessing,
    /// `PROCESSING` → `COMPLETED`
    Complete,
    /// → `CANCELLED` (predicates depend on current state)
    Cancel,
    /// `CREATED` | `PENDING` → `EXPIRED`
    Expire,
    /// `PENDING` → `FAILED` under Terminal Failure Decision (§7.3)
    FailPending { reason: TerminalFailureReason },
    /// `PAID` → `FAILED` under C2 + §6.1
    FailAfterPayment,
    /// `PROCESSING` → `FAILED` + §6.1
    FailDuringProcessing,
    /// Duplicate delivery of an already-applied transition (ADR-007 §8).
    ReplayDuplicate { resulting_state: TransactionState },
}

impl LifecycleCommand {
    /// Target primary state this command attempts to establish (if applied).
    pub fn target_state(&self) -> TransactionState {
        match self {
            Self::OpenPending => TransactionState::Pending,
            Self::AcceptPaymentSuccess => TransactionState::Paid,
            Self::StartProcessing => TransactionState::Processing,
            Self::Complete => TransactionState::Completed,
            Self::Cancel => TransactionState::Cancelled,
            Self::Expire => TransactionState::Expired,
            Self::FailPending { .. } | Self::FailAfterPayment | Self::FailDuringProcessing => {
                TransactionState::Failed
            }
            Self::ReplayDuplicate { resulting_state } => *resulting_state,
        }
    }
}

/// Result of evaluating a single lifecycle command against the current primary state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LifecycleResult {
    /// Primary state advanced.
    Transitioned {
        from: TransactionState,
        to: TransactionState,
    },
    /// Duplicate of already-applied transition — remain in state (ADR-007 §8).
    IdempotentNoOp { state: TransactionState },
}

impl LifecycleResult {
    pub fn resulting_state(&self) -> TransactionState {
        match self {
            Self::Transitioned { to, .. } => *to,
            Self::IdempotentNoOp { state } => *state,
        }
    }
}

/// Matrix edge classification (ADR-007 §6).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatrixEdge {
    Allowed,
    Forbidden,
    /// Conditional — predicates in `evaluate` / `edge_predicates_satisfied`.
    Conditional,
}

/// Look up the ADR-007 §6 transition matrix cell (from → to).
pub fn matrix_edge(from: TransactionState, to: TransactionState) -> MatrixEdge {
    use MatrixEdge::*;
    use TransactionState::*;

    // Self cells are forbidden as *new* edges; duplicate delivery is handled separately (§8).
    if from == to {
        return Forbidden;
    }

    match (from, to) {
        (Created, Pending) => Allowed,
        (Created, Cancelled) => Allowed,
        (Created, Expired) => Allowed,

        (Pending, Paid) => Allowed,
        (Pending, Cancelled) => Allowed,
        (Pending, Expired) => Allowed,
        (Pending, Failed) => Allowed, // only with Terminal Failure Decision (§7.3)

        (Paid, Processing) => Allowed,
        (Paid, Cancelled) => Conditional, // C1
        (Paid, Failed) => Conditional,    // C2

        (Processing, Completed) => Allowed,
        (Processing, Failed) => Allowed, // + §6.1 compensating intent
        (Processing, Cancelled) => Conditional, // C3

        // All other cells forbidden (including regressions and terminal exits).
        _ => Forbidden,
    }
}

/// Whether compensating-obligation intent is required for this from→to edge (ADR-007 §6.1).
pub fn requires_compensating_intent(from: TransactionState, to: TransactionState) -> bool {
    matches!(
        (from, to),
        (
            TransactionState::Paid | TransactionState::Processing,
            TransactionState::Cancelled | TransactionState::Failed
        )
    )
}

fn conditional_predicates_ok(
    from: TransactionState,
    to: TransactionState,
    ctx: &LifecycleContext,
) -> Result<(), DomainError> {
    let ok = match (from, to) {
        (TransactionState::Paid, TransactionState::Cancelled) => ctx.cancel_after_payment_decision,
        (TransactionState::Paid, TransactionState::Failed) => {
            ctx.terminal_business_inability_after_payment
        }
        (TransactionState::Processing, TransactionState::Cancelled) => {
            ctx.cancel_during_processing_decision
        }
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(DomainError::ConditionalPredicateUnsatisfied)
    }
}

fn ensure_compensating_intent(
    from: TransactionState,
    to: TransactionState,
    ctx: &LifecycleContext,
) -> Result<(), DomainError> {
    if requires_compensating_intent(from, to) && !ctx.compensating_intent_recorded {
        return Err(DomainError::CompensatingIntentRequired);
    }
    Ok(())
}

/// Evaluate a lifecycle command against the current primary state (pure, deterministic).
pub fn evaluate(
    current: TransactionState,
    command: &LifecycleCommand,
    ctx: &LifecycleContext,
) -> Result<LifecycleResult, DomainError> {
    if let LifecycleCommand::ReplayDuplicate { resulting_state } = command {
        if current == *resulting_state {
            return Ok(LifecycleResult::IdempotentNoOp { state: current });
        }
        return Err(DomainError::ConflictingTransition);
    }

    // Terminal primary states forbid further primary progression (ADR-007 §11).
    // Duplicate replay above remains allowed as idempotent no-op.
    if current.is_terminal() {
        return Err(DomainError::TerminalStateImmutable);
    }

    let to = command.target_state();

    // Stale / source mismatch: command expected a different source state.
    match command {
        LifecycleCommand::OpenPending if current != TransactionState::Created => {
            return Err(DomainError::StaleTransition);
        }
        LifecycleCommand::AcceptPaymentSuccess if current != TransactionState::Pending => {
            return Err(DomainError::StaleTransition);
        }
        LifecycleCommand::StartProcessing if current != TransactionState::Paid => {
            return Err(DomainError::StaleTransition);
        }
        LifecycleCommand::Complete if current != TransactionState::Processing => {
            return Err(DomainError::StaleTransition);
        }
        LifecycleCommand::Expire
            if !matches!(
                current,
                TransactionState::Created | TransactionState::Pending
            ) =>
        {
            return Err(DomainError::ForbiddenTransition);
        }
        LifecycleCommand::FailPending { .. } if current != TransactionState::Pending => {
            return Err(DomainError::StaleTransition);
        }
        LifecycleCommand::FailAfterPayment if current != TransactionState::Paid => {
            return Err(DomainError::StaleTransition);
        }
        LifecycleCommand::FailDuringProcessing if current != TransactionState::Processing => {
            return Err(DomainError::StaleTransition);
        }
        LifecycleCommand::Cancel
            if !matches!(
                current,
                TransactionState::Created
                    | TransactionState::Pending
                    | TransactionState::Paid
                    | TransactionState::Processing
            ) =>
        {
            return Err(DomainError::ForbiddenTransition);
        }
        _ => {}
    }

    match matrix_edge(current, to) {
        MatrixEdge::Forbidden => return Err(DomainError::ForbiddenTransition),
        MatrixEdge::Allowed | MatrixEdge::Conditional => {}
    }

    if matches!(command, LifecycleCommand::AcceptPaymentSuccess) && !ctx.known_success_evidence {
        return Err(DomainError::MissingKnownSuccessEvidence);
    }

    // PENDING → FAILED requires an explicit Terminal Failure Decision reason (carried on command).
    if matches!(command, LifecycleCommand::FailPending { .. }) {
        // reason present by construction; evidence of timeout alone is not this command.
    }

    if matches!(matrix_edge(current, to), MatrixEdge::Conditional) {
        conditional_predicates_ok(current, to, ctx)?;
    }

    ensure_compensating_intent(current, to, ctx)?;

    Ok(LifecycleResult::Transitioned { from: current, to })
}

/// Whether a candidate transition is matrix-eligible under the given context
/// (used for conflict-set filtering before precedence — ADR-007 §9.2).
pub fn is_eligible(
    current: TransactionState,
    to: TransactionState,
    ctx: &LifecycleContext,
) -> bool {
    // Reconstruct a minimal command for eligibility — only edge + predicates matter.
    let edge = matrix_edge(current, to);
    match edge {
        MatrixEdge::Forbidden => false,
        MatrixEdge::Allowed => {
            if to == TransactionState::Paid && !ctx.known_success_evidence {
                return false;
            }
            if to == TransactionState::Failed && current == TransactionState::Pending {
                // Terminal failure decision is represented by presence in conflict set;
                // eligibility for PENDING→FAILED requires caller to only nominate when decided.
                // Here we treat Allowed matrix cell as requiring compensating? No — pre-pay.
                true
            } else if requires_compensating_intent(current, to) {
                ctx.compensating_intent_recorded
            } else {
                true
            }
        }
        MatrixEdge::Conditional => {
            conditional_predicates_ok(current, to, ctx).is_ok()
                && ensure_compensating_intent(current, to, ctx).is_ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_paid_success() -> LifecycleContext {
        LifecycleContext {
            known_success_evidence: true,
            ..LifecycleContext::empty()
        }
    }

    #[test]
    fn happy_path_created_to_completed() {
        let mut state = TransactionState::Created;
        state = evaluate(
            state,
            &LifecycleCommand::OpenPending,
            &LifecycleContext::empty(),
        )
        .unwrap()
        .resulting_state();
        assert_eq!(state, TransactionState::Pending);
        state = evaluate(
            state,
            &LifecycleCommand::AcceptPaymentSuccess,
            &ctx_paid_success(),
        )
        .unwrap()
        .resulting_state();
        assert_eq!(state, TransactionState::Paid);
        state = evaluate(
            state,
            &LifecycleCommand::StartProcessing,
            &LifecycleContext::empty(),
        )
        .unwrap()
        .resulting_state();
        assert_eq!(state, TransactionState::Processing);
        state = evaluate(
            state,
            &LifecycleCommand::Complete,
            &LifecycleContext::empty(),
        )
        .unwrap()
        .resulting_state();
        assert_eq!(state, TransactionState::Completed);
    }

    #[test]
    fn paid_requires_known_success_evidence() {
        let err = evaluate(
            TransactionState::Pending,
            &LifecycleCommand::AcceptPaymentSuccess,
            &LifecycleContext::empty(),
        )
        .unwrap_err();
        assert_eq!(err, DomainError::MissingKnownSuccessEvidence);
    }

    #[test]
    fn c1_requires_decision_and_compensating_intent() {
        let mut ctx = LifecycleContext::empty();
        assert_eq!(
            evaluate(TransactionState::Paid, &LifecycleCommand::Cancel, &ctx).unwrap_err(),
            DomainError::ConditionalPredicateUnsatisfied
        );
        ctx.cancel_after_payment_decision = true;
        assert_eq!(
            evaluate(TransactionState::Paid, &LifecycleCommand::Cancel, &ctx).unwrap_err(),
            DomainError::CompensatingIntentRequired
        );
        ctx.compensating_intent_recorded = true;
        assert_eq!(
            evaluate(TransactionState::Paid, &LifecycleCommand::Cancel, &ctx)
                .unwrap()
                .resulting_state(),
            TransactionState::Cancelled
        );
    }

    #[test]
    fn terminal_immutable() {
        for terminal in [
            TransactionState::Completed,
            TransactionState::Cancelled,
            TransactionState::Expired,
            TransactionState::Failed,
        ] {
            assert_eq!(
                evaluate(
                    terminal,
                    &LifecycleCommand::OpenPending,
                    &LifecycleContext::empty()
                )
                .unwrap_err(),
                DomainError::TerminalStateImmutable
            );
        }
    }

    #[test]
    fn duplicate_replay_noop() {
        let r = evaluate(
            TransactionState::Paid,
            &LifecycleCommand::ReplayDuplicate {
                resulting_state: TransactionState::Paid,
            },
            &LifecycleContext::empty(),
        )
        .unwrap();
        assert_eq!(
            r,
            LifecycleResult::IdempotentNoOp {
                state: TransactionState::Paid
            }
        );
    }

    #[test]
    fn forbidden_pending_to_processing() {
        assert_eq!(
            matrix_edge(TransactionState::Pending, TransactionState::Processing),
            MatrixEdge::Forbidden
        );
    }
}
