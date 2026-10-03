# Inference diagnostic and planning payload layout

Fresh aggregate diagnostics identify two large payload owners: ManagedRuntimeCommandResolutionError::MissingRuntimeVariant embeds DeviceResolutionDiagnostic, and ImageGenerationPlanningOutcome::Planned embeds ImageGenerationExecutionPlan. Box only those payload fields. Leave all other error variants, request validation, planner decisions, facade context, registry resolution return types, and high-arity functions unchanged.

Caller inventory: the managed-runtime diagnostic has one constructor and a Display arm reading its existing message; platform and serialization tests retain diagnostic access. The planning outcome has one constructor, two consuming gateway branches and one consuming embedded batch branch that unbox at their existing boundary, plus a worker projection test that explicitly borrows the raw plan. Inspection-only matches retain their behavior.

These are explicit public Rust field-type changes, documented in the changelog despite inference being publish=false at workspace development version 0.1.0. Direct constructors must box payloads; owned plan consumers must unbox. Tagged JSON, diagnostic fields, Display messages, and underlying plan/result types remain unchanged. No release/version bump or unseen-consumer compatibility claim is made.

The managed-runtime test checks exact full JSON, old-shape decoding, equality and exact Display text. The successful planner fixture retains its detailed plan assertions and adds exact outer tagged shape/round-trip/layout checks. Hosted steps run managed-runtime contracts, all existing planner acceptance/rejection tests, and worker image projection tests. The inline size assertions are not execution-performance measurements.

Formatting and whitespace pass locally. No local workspace compilation/new Rust execution is claimed. Independent source review and fresh hosted inference/embedded/workflow qualification are pending.

Independent source review accepted tree d8784bd001db2ffef90b0c97465b167bf1c51472, including the bounded two-field layout change, consuming caller adaptations, exact JSON/Display assertions and retained planner suites. Fresh hosted execution is still required.


## Hosted qualification correction

At head 58df81689306debaced0e224e16576742376381d, the exact managed-runtime error test and all 25 planner tests pass. The worker projection command ran zero tests because inference's default feature is backend-llamacpp. The corrected command explicitly enables backend-pytorch, checks that the filtered suite lists at least one test, and then runs that same feature/filter selection. Worker projection execution remains unqualified until the corrected hosted step succeeds. Aggregate inference Clippy is down to 11 remaining findings; those are separate scope.

Independent source review accepted correction tree 6e3942251a33f311f829aba75b10859bb46482f6: explicit feature selection and nonzero exact-module discovery close the false-green gap. The final receipt must report the actual executed count; the prior zero-test step remains unqualified.
