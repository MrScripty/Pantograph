//! Pure comparison of the two complete serial service orders for two workloads.
//!
//! The caller qualifies the predecessor-conditioned observations. This module
//! owns only checked numeric scoring; it cannot establish readiness, owner
//! identity, evidence freshness, or permission to dispatch.

const PLANS: usize = 2;
const COMPLETION_EVENTS: usize = 4;

/// Work allowance for comparing both complete orders. Larger allowances do not
/// expand the fixed two-plan, four-completion-event comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerSerialServiceBudget {
    pub max_plans: usize,
    pub max_completion_events: usize,
}

impl Default for SchedulerSerialServiceBudget {
    fn default() -> Self {
        Self {
            max_plans: PLANS,
            max_completion_events: COMPLETION_EVENTS,
        }
    }
}

/// Complete scalar service scores, expressed in nanoseconds.
///
/// These sums exclude inter-dispatch scheduling gaps, cold load, and transfer.
/// They approximate predecessor-conditioned service, rather than wall-clock
/// completion, and convey no authority to start either workload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerSerialServiceCompletion {
    /// Index of the workload to place first: 0 or 1 in the caller's FIFO order.
    pub selected_position: usize,
    /// For each first-workload position, its first and terminal cumulative
    /// service completions: `[[A|P, A|P + B|A], [B|P, B|P + A|B]]`.
    pub completions_ns: [[u64; 2]; 2],
    /// Sum of the first and terminal completions for each complete order.
    pub completion_sums_ns: [u64; 2],
    pub plans_evaluated: usize,
    pub completion_events_evaluated: usize,
}

/// A refusal covers the entire comparison; no partially scored order wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerSerialServiceRefusal {
    /// At least one of the four predecessor-conditioned service rows is absent.
    MissingEvidence,
    /// The allowance cannot cover both plans and all four completion events.
    BudgetExhausted,
    /// A terminal completion or completion sum cannot be represented in `u64`.
    DurationOverflow,
}

/// Compare both complete serial orders using qualified service envelopes.
///
/// For current predecessor P and workloads A/B in FIFO order, `first_ns` is
/// `[A|P, B|P]` and `successor_ns` is `[B|A, A|B]`. Each value is a maximum
/// observed dispatch-through-matched-cleanup duration in nanoseconds. The caller
/// must establish sufficient fresh samples, exact owner/predecessor association,
/// and the admissible two-workload population before supplying these scalars.
/// Missing, stale, foreign, or unknown-predecessor evidence must be absent;
/// unconditioned service observations are never substitutes.
///
/// The scores exclude scheduling gaps, cold load, and transfer rather than
/// estimating their costs. Minimize terminal cumulative service, then the sum
/// of both completions, then FIFO position 0. Both orders and all scores must
/// fit before a result is returned. Work is fixed at two plans/four events even
/// when the budget is larger. Zero durations are representable numeric inputs;
/// observation qualification remains the caller's responsibility.
///
/// Returns [`SchedulerSerialServiceRefusal::BudgetExhausted`] for an insufficient
/// budget, [`SchedulerSerialServiceRefusal::MissingEvidence`] for any absent
/// row, or [`SchedulerSerialServiceRefusal::DurationOverflow`] for checked
/// arithmetic overflow. The result is advisory and does not authorize dispatch.
pub fn evaluate_serial_service_completion(
    first_ns: [Option<u64>; 2],
    successor_ns: [Option<u64>; 2],
    budget: SchedulerSerialServiceBudget,
) -> Result<SchedulerSerialServiceCompletion, SchedulerSerialServiceRefusal> {
    use SchedulerSerialServiceRefusal as Refusal;

    if budget.max_plans < PLANS || budget.max_completion_events < COMPLETION_EVENTS {
        return Err(Refusal::BudgetExhausted);
    }
    let ([Some(first_a), Some(first_b)], [Some(after_a), Some(after_b)]) = (first_ns, successor_ns)
    else {
        return Err(Refusal::MissingEvidence);
    };

    let mut completions_ns = [[0; 2]; 2];
    let mut completion_sums_ns = [0; 2];
    for (position, (first, successor)) in [(first_a, after_a), (first_b, after_b)]
        .into_iter()
        .enumerate()
    {
        let terminal = first
            .checked_add(successor)
            .ok_or(Refusal::DurationOverflow)?;
        let sum = first
            .checked_add(terminal)
            .ok_or(Refusal::DurationOverflow)?;
        completions_ns[position] = [first, terminal];
        completion_sums_ns[position] = sum;
    }

    let selected_position = usize::from(
        (completions_ns[1][1], completion_sums_ns[1])
            < (completions_ns[0][1], completion_sums_ns[0]),
    );
    Ok(SchedulerSerialServiceCompletion {
        selected_position,
        completions_ns,
        completion_sums_ns,
        plans_evaluated: PLANS,
        completion_events_evaluated: COMPLETION_EVENTS,
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        evaluate_serial_service_completion, SchedulerSerialServiceBudget,
        SchedulerSerialServiceCompletion, SchedulerSerialServiceRefusal,
    };

    #[test]
    fn complete_service_order_reverses_immediate_service_choice() {
        // P=A: A|A=7, B|A=3, A|B=20. Immediate service favors B;
        // the independently calculated terminal services 10 versus 23 favor A.
        let result = evaluate_serial_service_completion(
            [Some(7), Some(3)],
            [Some(3), Some(20)],
            SchedulerSerialServiceBudget::default(),
        );
        assert_eq!(
            result,
            Ok(SchedulerSerialServiceCompletion {
                selected_position: 0,
                completions_ns: [[7, 10], [3, 23]],
                completion_sums_ns: [17, 26],
                plans_evaluated: 2,
                completion_events_evaluated: 4,
            })
        );
    }

    #[test]
    fn terminal_service_precedes_completion_sum() {
        let result = evaluate_serial_service_completion(
            [Some(1), Some(4)],
            [Some(8), Some(4)],
            SchedulerSerialServiceBudget::default(),
        )
        .unwrap();
        assert_eq!(result.completions_ns, [[1, 9], [4, 8]]);
        assert_eq!(result.completion_sums_ns, [10, 12]);
        assert_eq!(result.selected_position, 1);
    }

    #[test]
    fn equal_terminal_service_uses_completion_sum() {
        let result = evaluate_serial_service_completion(
            [Some(7), Some(3)],
            [Some(3), Some(7)],
            SchedulerSerialServiceBudget::default(),
        )
        .unwrap();
        assert_eq!(result.completions_ns, [[7, 10], [3, 10]]);
        assert_eq!(result.completion_sums_ns, [17, 13]);
        assert_eq!(result.selected_position, 1);
    }

    #[test]
    fn equal_complete_scores_preserve_fifo() {
        let result = evaluate_serial_service_completion(
            [Some(3), Some(3)],
            [Some(7), Some(7)],
            SchedulerSerialServiceBudget::default(),
        )
        .unwrap();
        assert_eq!(result.completions_ns, [[3, 10], [3, 10]]);
        assert_eq!(result.completion_sums_ns, [13, 13]);
        assert_eq!(result.selected_position, 0);
    }

    #[test]
    fn every_missing_row_refuses_the_entire_comparison() {
        for (first, successor) in [
            ([None, Some(3)], [Some(3), Some(20)]),
            ([Some(7), None], [Some(3), Some(20)]),
            ([Some(7), Some(3)], [None, Some(20)]),
            ([Some(7), Some(3)], [Some(3), None]),
        ] {
            assert_eq!(
                evaluate_serial_service_completion(
                    first,
                    successor,
                    SchedulerSerialServiceBudget::default(),
                ),
                Err(SchedulerSerialServiceRefusal::MissingEvidence)
            );
        }
    }

    #[test]
    fn insufficient_plan_or_completion_event_budget_refuses() {
        for (max_plans, max_completion_events) in [(0, 4), (1, 4), (2, 0), (2, 1), (2, 2), (2, 3)] {
            assert_eq!(
                evaluate_serial_service_completion(
                    [Some(7), Some(3)],
                    [Some(3), Some(20)],
                    SchedulerSerialServiceBudget {
                        max_plans,
                        max_completion_events,
                    },
                ),
                Err(SchedulerSerialServiceRefusal::BudgetExhausted)
            );
        }
    }

    #[test]
    fn overflow_in_either_order_refuses_even_after_a_complete_first_score() {
        for (first, successor) in [
            // Terminal overflow in the first and second order respectively.
            ([Some(u64::MAX), Some(1)], [Some(1), Some(1)]),
            ([Some(1), Some(u64::MAX)], [Some(1), Some(1)]),
            // Terminals fit, but the first or second completion sum overflows.
            ([Some(u64::MAX / 2 + 1), Some(1)], [Some(0), Some(1)]),
            ([Some(1), Some(u64::MAX / 2 + 1)], [Some(1), Some(0)]),
        ] {
            assert_eq!(
                evaluate_serial_service_completion(
                    first,
                    successor,
                    SchedulerSerialServiceBudget::default(),
                ),
                Err(SchedulerSerialServiceRefusal::DurationOverflow)
            );
        }
    }

    #[test]
    fn oversized_budget_evaluates_only_two_plans_and_four_events() {
        let result = evaluate_serial_service_completion(
            [Some(7), Some(3)],
            [Some(3), Some(20)],
            SchedulerSerialServiceBudget {
                max_plans: usize::MAX,
                max_completion_events: usize::MAX,
            },
        )
        .unwrap();
        assert_eq!(result.plans_evaluated, 2);
        assert_eq!(result.completion_events_evaluated, 4);
        assert_eq!(result.completions_ns, [[7, 10], [3, 23]]);
        assert_eq!(result.completion_sums_ns, [17, 26]);
        assert_eq!(result.selected_position, 0);
    }

    #[test]
    fn representable_zero_and_maximum_scores_are_preserved() {
        let result = evaluate_serial_service_completion(
            [Some(0), Some(0)],
            [Some(0), Some(u64::MAX)],
            SchedulerSerialServiceBudget::default(),
        )
        .unwrap();
        assert_eq!(result.completions_ns, [[0, 0], [0, u64::MAX]]);
        assert_eq!(result.completion_sums_ns, [0, u64::MAX]);
        assert_eq!(result.selected_position, 0);
    }
}
