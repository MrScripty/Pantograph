//! Prospective independent u128 accounting oracle; no native inspection runs.
//! Authored before reading the implementation, from the frozen admission spec.
use pantograph_timing_contracts::{
    RuntimeServiceTimingInspectionAccounting as Accounting,
    RuntimeServiceTimingInspectionBound as Bound, RuntimeServiceTimingInspectionBudget as Budget,
    RuntimeServiceTimingInspectionLedger as Ledger,
    RuntimeServiceTimingInspectionObservation as Observation,
    RuntimeServiceTimingInspectionOperation as Operation,
    RuntimeServiceTimingInspectionRefusal as Refusal,
    RuntimeServiceTimingInspectionResult as InspectionResult,
    RuntimeServiceTimingInspectionUpperBound as UpperBound,
};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn bound(work_units: u64, copied_bytes: u64) -> Bound {
    Bound::Declared(UpperBound {
        work_units,
        copied_bytes,
    })
}
fn observation(elapsed_ns: Option<u64>) -> Observation<(), &'static str> {
    Observation {
        result: Ok(()),
        elapsed_ns,
    }
}
fn expect_refused<T, E>(value: InspectionResult<T, E>, expected: Refusal) {
    assert!(matches!(value, InspectionResult::Refused(reason) if reason == expected));
}
fn fits(before: Accounting, budget: Budget, work: u64, copied: u64) -> bool {
    // Wider independent arithmetic; production checked-add helpers are not used.
    let calls = u128::from(before.reserved_calls) + 1;
    let units = u128::from(before.reserved_work_units) + u128::from(work);
    let bytes = u128::from(before.reserved_copied_bytes) + u128::from(copied);
    calls <= u128::from(budget.max_calls)
        && units <= u128::from(budget.max_work_units)
        && bytes <= u128::from(budget.max_copied_bytes)
}

#[test]
fn inspection_unknown_and_native_declarations_never_invoke_callback() {
    let mut ledger = Ledger::new(Budget {
        max_calls: 10,
        max_work_units: 100,
        max_copied_bytes: 100,
    });
    let called = Cell::new(0);
    let before = ledger.accounting();
    let result = ledger.try_inspect(Operation::TokenizerGetter, Bound::Unknown, || {
        called.set(called.get() + 1);
        observation(Some(3))
    });
    expect_refused(result, Refusal::UnknownPreCallBound);
    assert_eq!(ledger.accounting(), before);
    for declaration in [
        Bound::Unknown,
        bound(0, 0),
        bound(1, 1),
        bound(u64::MAX, u64::MAX),
    ] {
        expect_refused(
            ledger.try_inspect(Operation::NativeOwnerSnapshot, declaration, || {
                called.set(called.get() + 1);
                observation(Some(3))
            }),
            Refusal::UnknownPreCallBound,
        );
        assert_eq!(ledger.accounting(), before);
    }
    expect_refused(
        ledger.inspect_native_owner(|| {
            called.set(called.get() + 1);
            observation(Some(3))
        }),
        Refusal::UnknownPreCallBound,
    );
    assert_eq!(called.get(), 0);
    assert_eq!(ledger.accounting(), before);
}

#[test]
fn inspection_known_zero_is_real_but_finite_call_budget_cannot_be_bypassed() {
    let mut ledger = Ledger::new(Budget {
        max_calls: 2,
        max_work_units: 0,
        max_copied_bytes: 0,
    });
    for operation in [Operation::TokenizerGetter, Operation::ConfigurationRead] {
        assert!(matches!(
            ledger.try_inspect(operation, bound(0, 0), || observation(Some(0))),
            InspectionResult::Completed(Ok(()))
        ));
    }
    let before = ledger.accounting();
    expect_refused(
        ledger.try_inspect(
            Operation::TensorRead,
            bound(0, 0),
            || -> Observation<(), ()> { panic!("exhausted zero-cost call must not run") },
        ),
        Refusal::CallBudgetExceeded,
    );
    assert_eq!(ledger.accounting(), before);
    assert_eq!(
        (
            before.reserved_calls,
            before.completed_calls,
            before.failed_calls
        ),
        (2, 2, 0)
    );
    assert_eq!(
        (before.reserved_work_units, before.reserved_copied_bytes),
        (0, 0)
    );
    assert_eq!(before.observed_elapsed_ns, Some(0));
    assert!(!ledger.accounting().has_incomplete_calls());
}

#[test]
fn inspection_exact_thresholds_and_single_dimension_refusals_are_atomic() {
    for (budget, request, expected) in [
        (
            Budget {
                max_calls: 0,
                max_work_units: 10,
                max_copied_bytes: 10,
            },
            (1, 1),
            Refusal::CallBudgetExceeded,
        ),
        (
            Budget {
                max_calls: 1,
                max_work_units: 9,
                max_copied_bytes: 10,
            },
            (10, 10),
            Refusal::WorkBudgetExceeded,
        ),
        (
            Budget {
                max_calls: 1,
                max_work_units: 10,
                max_copied_bytes: 9,
            },
            (10, 10),
            Refusal::CopyBudgetExceeded,
        ),
    ] {
        let mut ledger = Ledger::new(budget);
        let before = ledger.accounting();
        expect_refused(
            ledger.try_inspect(
                Operation::TokenizerExport,
                bound(request.0, request.1),
                || -> Observation<(), ()> {
                    panic!("one-unit cap refusal must not invoke callback")
                },
            ),
            expected,
        );
        assert_eq!(ledger.accounting(), before);
    }
    let mut ledger = Ledger::new(Budget {
        max_calls: 1,
        max_work_units: 10,
        max_copied_bytes: 10,
    });
    assert!(matches!(
        ledger.try_inspect(Operation::TokenizerExport, bound(10, 10), || observation(
            Some(1)
        )),
        InspectionResult::Completed(Ok(()))
    ));
    let a = ledger.accounting();
    assert_eq!(
        (
            a.reserved_calls,
            a.reserved_work_units,
            a.reserved_copied_bytes
        ),
        (1, 10, 10)
    );
    assert_eq!(a.last_started_operation, Some(Operation::TokenizerExport));
    assert_eq!(a.last_completed_operation, Some(Operation::TokenizerExport));
}

#[test]
fn inspection_independent_generated_sequence_conserves_pre_call_reservations() {
    for calls in 0..=4 {
        for units in 0..=4 {
            for bytes in 0..=4 {
                let budget = Budget {
                    max_calls: calls,
                    max_work_units: units,
                    max_copied_bytes: bytes,
                };
                for seed in 0..12_u64 {
                    let mut ledger = Ledger::new(budget);
                    let invoked = Cell::new(0_u64);
                    let mut expected_units = 0;
                    let mut expected_bytes = 0;
                    for step in 0..8_u64 {
                        let work = (seed + step * 3) % 5;
                        let copy = (seed * 2 + step) % 5;
                        let before = ledger.accounting();
                        let admitted = fits(before, budget, work, copy);
                        let failure = (seed + step).is_multiple_of(3);
                        let result = ledger.try_inspect(
                            Operation::ConfigurationRead,
                            bound(work, copy),
                            || {
                                invoked.set(invoked.get() + 1);
                                Observation {
                                    result: if failure {
                                        Err("controlled error")
                                    } else {
                                        Ok(())
                                    },
                                    elapsed_ns: Some(2),
                                }
                            },
                        );
                        if admitted {
                            expected_units += work;
                            expected_bytes += copy;
                            match result {
                                InspectionResult::Completed(v) => assert_eq!(v.is_err(), failure),
                                _ => panic!("u128 reference admitted a refused invocation"),
                            }
                            let after = ledger.accounting();
                            assert_eq!(after.reserved_calls, before.reserved_calls + 1);
                            assert_eq!(after.completed_calls, before.completed_calls + 1);
                            assert_eq!(
                                after.failed_calls,
                                before.failed_calls + u64::from(failure)
                            );
                            assert_eq!(after.observed_elapsed_ns, Some(2 * invoked.get()));
                        } else {
                            assert!(matches!(result, InspectionResult::Refused(_)));
                            assert_eq!(ledger.accounting(), before);
                        }
                        let actual = ledger.accounting();
                        assert_eq!(actual.reserved_calls, invoked.get());
                        assert_eq!(actual.reserved_work_units, expected_units);
                        assert_eq!(actual.reserved_copied_bytes, expected_bytes);
                        assert!(!ledger.accounting().has_incomplete_calls());
                    }
                }
            }
        }
    }
}

#[test]
fn inspection_integer_maxima_accept_exactly_and_overflow_never_wraps() {
    for dimension in ["work", "copy"] {
        let mut ledger = Ledger::new(Budget {
            max_calls: u64::MAX,
            max_work_units: u64::MAX,
            max_copied_bytes: u64::MAX,
        });
        let first = if dimension == "work" {
            bound(u64::MAX, 0)
        } else {
            bound(0, u64::MAX)
        };
        assert!(matches!(
            ledger.try_inspect(Operation::TensorRead, first, || observation(Some(0))),
            InspectionResult::Completed(Ok(()))
        ));
        let before = ledger.accounting();
        let next = if dimension == "work" {
            bound(1, 0)
        } else {
            bound(0, 1)
        };
        let expected = if dimension == "work" {
            Refusal::WorkBudgetExceeded
        } else {
            Refusal::CopyBudgetExceeded
        };
        expect_refused(
            ledger.try_inspect(Operation::TensorRead, next, || -> Observation<(), ()> {
                panic!("overflow must not execute")
            }),
            expected,
        );
        assert_eq!(ledger.accounting(), before);
    }
}

#[test]
fn inspection_callback_error_remains_charged_and_exact_error_is_preserved() {
    let mut ledger = Ledger::new(Budget {
        max_calls: 3,
        max_work_units: 13,
        max_copied_bytes: 21,
    });
    let result = ledger.try_inspect(Operation::TokenizerGetter, bound(5, 8), || Observation {
        result: Err::<(), _>("native getter failure"),
        elapsed_ns: Some(9),
    });
    assert!(matches!(
        result,
        InspectionResult::Completed(Err("native getter failure"))
    ));
    assert!(matches!(
        ledger.try_inspect(Operation::TokenizerExport, bound(8, 13), || observation(
            Some(4)
        )),
        InspectionResult::Completed(Ok(()))
    ));
    let before = ledger.accounting();
    assert_eq!(
        (
            before.reserved_calls,
            before.reserved_work_units,
            before.reserved_copied_bytes
        ),
        (2, 13, 21)
    );
    assert_eq!(
        (
            before.completed_calls,
            before.failed_calls,
            before.observed_elapsed_ns
        ),
        (2, 1, Some(13))
    );
    expect_refused(
        ledger.try_inspect(
            Operation::TokenizerGetter,
            bound(1, 0),
            || -> Observation<(), ()> { panic!("failed calls may not be refunded") },
        ),
        Refusal::WorkBudgetExceeded,
    );
    assert_eq!(ledger.accounting(), before);
    assert!(!ledger.accounting().has_incomplete_calls());
}

#[test]
fn inspection_panic_preserves_reserved_uncertainty_without_completion_or_refund() {
    let mut ledger = Ledger::new(Budget {
        max_calls: 3,
        max_work_units: 10,
        max_copied_bytes: 10,
    });
    assert!(matches!(
        ledger.try_inspect(Operation::TokenizerGetter, bound(1, 1), || observation(
            Some(2)
        )),
        InspectionResult::Completed(Ok(()))
    ));
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let _: InspectionResult<(), ()> =
            ledger.try_inspect(Operation::TensorRead, bound(4, 6), || {
                panic!("controlled unwinding")
            });
    }));
    assert!(panicked.is_err());
    let after = ledger.accounting();
    assert_eq!(
        (
            after.reserved_calls,
            after.reserved_work_units,
            after.reserved_copied_bytes
        ),
        (2, 5, 7)
    );
    assert_eq!(
        (
            after.completed_calls,
            after.failed_calls,
            after.observed_elapsed_ns
        ),
        (1, 0, Some(2))
    );
    assert_eq!(after.last_started_operation, Some(Operation::TensorRead));
    assert_eq!(
        after.last_completed_operation,
        Some(Operation::TokenizerGetter)
    );
    assert!(ledger.accounting().has_incomplete_calls());
    assert!(matches!(
        ledger.try_inspect(Operation::ConfigurationRead, bound(5, 3), || observation(
            Some(3)
        )),
        InspectionResult::Completed(Ok(()))
    ));
    assert!(ledger.accounting().has_incomplete_calls());
    assert_eq!(ledger.accounting().observed_elapsed_ns, Some(5));
}

#[test]
fn inspection_unknown_or_overflow_elapsed_is_poisoned_without_refunding_costs() {
    for durations in [[None, Some(0), Some(7)], [Some(u64::MAX), Some(1), Some(0)]] {
        let mut ledger = Ledger::new(Budget {
            max_calls: 3,
            max_work_units: 9,
            max_copied_bytes: 12,
        });
        for (index, elapsed) in durations.into_iter().enumerate() {
            assert!(matches!(
                ledger.try_inspect(Operation::ConfigurationRead, bound(3, 4), || observation(
                    elapsed
                )),
                InspectionResult::Completed(Ok(()))
            ));
            let a = ledger.accounting();
            assert_eq!(
                (a.reserved_calls, a.completed_calls),
                ((index + 1) as u64, (index + 1) as u64)
            );
            assert_eq!(a.reserved_work_units, 3 * (index + 1) as u64);
            assert_eq!(a.reserved_copied_bytes, 4 * (index + 1) as u64);
        }
        assert_eq!(ledger.accounting().observed_elapsed_ns, None);
        assert!(!ledger.accounting().has_incomplete_calls());
    }
}

#[test]
fn inspection_elapsed_is_separate_evidence_and_not_a_hard_deadline() {
    let mut ledger = Ledger::new(Budget {
        max_calls: 1,
        max_work_units: 0,
        max_copied_bytes: 0,
    });
    // A caller-supplied large elapsed value grants no timeout mechanism or units conversion.
    assert!(matches!(
        ledger.try_inspect(Operation::ConfigurationRead, bound(0, 0), || observation(
            Some(u64::MAX)
        )),
        InspectionResult::Completed(Ok(()))
    ));
    let a = ledger.accounting();
    assert_eq!(a.observed_elapsed_ns, Some(u64::MAX));
    assert_eq!((a.reserved_work_units, a.reserved_copied_bytes), (0, 0));
    assert_eq!((a.reserved_calls, a.completed_calls), (1, 1));
}

#[test]
fn inspection_refusal_does_not_invoke_body_but_caller_capture_drop_is_outside_ledger() {
    struct Capture<'a>(&'a Cell<u64>);
    impl Drop for Capture<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let dropped = Cell::new(0);
    let invoked = Cell::new(0);
    let captured = Capture(&dropped);
    let mut ledger = Ledger::new(Budget {
        max_calls: 0,
        max_work_units: 0,
        max_copied_bytes: 0,
    });
    let before = ledger.accounting();
    let result = ledger.inspect_native_owner(|| {
        invoked.set(invoked.get() + 1);
        // Moving the entire captured value binds its destructor to the closure.
        drop(captured);
        observation(Some(0))
    });
    expect_refused(result, Refusal::UnknownPreCallBound);
    assert_eq!(invoked.get(), 0);
    assert_eq!(dropped.get(), 1);
    assert_eq!(ledger.accounting(), before);
}
