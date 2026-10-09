//! Pre-call admission and accounting for advisory identity inspection.
//!
//! These declarations are not timing identity, capacity, or native certification.
//! A bounded adapter must establish its worst-case work/allocation bounds before
//! entering this ledger. Post-call lengths and elapsed time cannot do that.

/// The fixed operation vocabulary avoids copying arbitrary diagnostic metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeServiceTimingInspectionOperation {
    NativeOwnerSnapshot,
    TokenizerGetter,
    TokenizerExport,
    TensorRead,
    ConfigurationRead,
}

/// Adapter-declared pre-call maxima, including refused/failed native work and
/// temporary copies. Work units are adapter-specific, never inferred CPU time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeServiceTimingInspectionUpperBound {
    pub work_units: u64,
    pub copied_bytes: u64,
}

/// Unknown must not be represented as zero or derived from a post-call length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeServiceTimingInspectionBound {
    Unknown,
    Declared(RuntimeServiceTimingInspectionUpperBound),
}

/// Caller-selected aggregate ceilings. There is no default or calibrated budget.
/// A call ceiling also bounds a sequence of zero-work/zero-copy operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeServiceTimingInspectionBudget {
    pub max_calls: u64,
    pub max_work_units: u64,
    pub max_copied_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeServiceTimingInspectionRefusal {
    UnknownPreCallBound,
    CallBudgetExceeded,
    WorkBudgetExceeded,
    CopyBudgetExceeded,
}

/// Actual result and separately supplied monotonic elapsed observation. Missing
/// elapsed is retained as unknown. This is not a deadline or a work-bound proof.
pub struct RuntimeServiceTimingInspectionObservation<T, E> {
    pub result: Result<T, E>,
    pub elapsed_ns: Option<u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeServiceTimingInspectionResult<T, E> {
    Refused(RuntimeServiceTimingInspectionRefusal),
    Completed(Result<T, E>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeServiceTimingInspectionAccounting {
    pub last_started_operation: Option<RuntimeServiceTimingInspectionOperation>,
    pub last_completed_operation: Option<RuntimeServiceTimingInspectionOperation>,
    pub reserved_calls: u64,
    pub reserved_work_units: u64,
    pub reserved_copied_bytes: u64,
    pub completed_calls: u64,
    pub failed_calls: u64,
    /// All completed calls, including failures; None stays unknown thereafter.
    pub observed_elapsed_ns: Option<u64>,
}

impl RuntimeServiceTimingInspectionAccounting {
    /// An unwound callback remains charged and has no completed observation.
    pub fn has_incomplete_calls(self) -> bool {
        self.completed_calls != self.reserved_calls
    }
}

/// Constant-size state and constant work for the ledger-owned admission decision;
/// the decision never scans native data.
/// The ledger enforces declarations, not their truth or a wall-time deadline.
/// Counters cover adapter reservations; caller retry loops and gate overhead are
/// not represented as measured native work or bounded by the aggregate totals.
/// Caller-owned closure construction/destruction is outside these declarations:
/// dropping a refused closure can itself run arbitrary Drop code or panic.
pub struct RuntimeServiceTimingInspectionLedger {
    budget: RuntimeServiceTimingInspectionBudget,
    accounting: RuntimeServiceTimingInspectionAccounting,
}

impl RuntimeServiceTimingInspectionLedger {
    pub fn new(budget: RuntimeServiceTimingInspectionBudget) -> Self {
        Self {
            budget,
            accounting: RuntimeServiceTimingInspectionAccounting {
                last_started_operation: None,
                last_completed_operation: None,
                reserved_calls: 0,
                reserved_work_units: 0,
                reserved_copied_bytes: 0,
                completed_calls: 0,
                failed_calls: 0,
                observed_elapsed_ns: Some(0),
            },
        }
    }

    pub fn accounting(&self) -> RuntimeServiceTimingInspectionAccounting {
        self.accounting
    }

    /// Reserve worst-case costs atomically before calling the adapter. A refused
    /// request never invokes the callback body and changes no counters. The
    /// consumed closure is still dropped, including on refusal; captured Drop
    /// code is caller-owned work and is not bounded/accounted by this ledger.
    /// Once admitted, costs are
    /// never refunded, including Err or panic; panics propagate unchanged.
    pub fn try_inspect<T, E>(
        &mut self,
        operation: RuntimeServiceTimingInspectionOperation,
        bound: RuntimeServiceTimingInspectionBound,
        inspect: impl FnOnce() -> RuntimeServiceTimingInspectionObservation<T, E>,
    ) -> RuntimeServiceTimingInspectionResult<T, E> {
        use RuntimeServiceTimingInspectionRefusal as Refusal;
        // This profile has no established native pre-call proof. A declared cap
        // cannot turn its current getter/export API into a bounded implementation.
        if operation == RuntimeServiceTimingInspectionOperation::NativeOwnerSnapshot {
            return RuntimeServiceTimingInspectionResult::Refused(Refusal::UnknownPreCallBound);
        }
        let RuntimeServiceTimingInspectionBound::Declared(bound) = bound else {
            return RuntimeServiceTimingInspectionResult::Refused(Refusal::UnknownPreCallBound);
        };
        let add = |current: u64, extra: u64, ceiling: u64| {
            current.checked_add(extra).filter(|sum| *sum <= ceiling)
        };
        let Some(calls) = add(self.accounting.reserved_calls, 1, self.budget.max_calls) else {
            return RuntimeServiceTimingInspectionResult::Refused(Refusal::CallBudgetExceeded);
        };
        let Some(work) = add(
            self.accounting.reserved_work_units,
            bound.work_units,
            self.budget.max_work_units,
        ) else {
            return RuntimeServiceTimingInspectionResult::Refused(Refusal::WorkBudgetExceeded);
        };
        let Some(bytes) = add(
            self.accounting.reserved_copied_bytes,
            bound.copied_bytes,
            self.budget.max_copied_bytes,
        ) else {
            return RuntimeServiceTimingInspectionResult::Refused(Refusal::CopyBudgetExceeded);
        };
        self.accounting.reserved_calls = calls;
        self.accounting.reserved_work_units = work;
        self.accounting.reserved_copied_bytes = bytes;
        self.accounting.last_started_operation = Some(operation);
        let observation = inspect();
        // completed <= reserved <= u64::MAX; these additions cannot overflow.
        self.accounting.completed_calls += 1;
        self.accounting.last_completed_operation = Some(operation);
        self.accounting.failed_calls += u64::from(observation.result.is_err());
        self.accounting.observed_elapsed_ns = self
            .accounting
            .observed_elapsed_ns
            .zip(observation.elapsed_ns)
            .and_then(|(total, elapsed)| total.checked_add(elapsed));
        RuntimeServiceTimingInspectionResult::Completed(observation.result)
    }

    /// Strict preflight for the current native owner snapshot. Native tokenizer
    /// getters/export, AddedToken/config/build getters can allocate before their
    /// returned lengths are checked. No bounded exporter/input proof is installed,
    /// so refuse before invoking the callback body even if a caller claims a
    /// sufficiently large cap. Caller-owned closure construction/drop is outside
    /// this preflight, as in try_inspect.
    /// This does not change or qualify the existing advisory native observer.
    pub fn inspect_native_owner<T, E>(
        &mut self,
        inspect: impl FnOnce() -> RuntimeServiceTimingInspectionObservation<T, E>,
    ) -> RuntimeServiceTimingInspectionResult<T, E> {
        self.try_inspect(
            RuntimeServiceTimingInspectionOperation::NativeOwnerSnapshot,
            RuntimeServiceTimingInspectionBound::Unknown,
            inspect,
        )
    }
}
