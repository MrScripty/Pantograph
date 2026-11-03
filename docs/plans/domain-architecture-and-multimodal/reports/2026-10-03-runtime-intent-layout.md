# Boxed runtime intent payload

## Accepted source-API decision

Box only the large Runtime.task_intent field in SchedulerTaskExecutionIntent and add runtime(intent) as the preferred constructor. Migrate all ten repository construction sites; the remaining explicit Runtime match only inspects the variant. Borrowed runtime_task_intent access, validation/correlation, and tagged Serde representation remain unchanged. Source consumers directly constructing the public struct variant must now supply Box::new(intent); the changelog records this source-breaking change. Unseen Git/path consumers are not claimed compatible.

The scheduler crate is publish=false and inherits workspace development version 0.1.0. docs/release.md says no accepted release workflow currently exists; it distinguishes development packaging from release acceptance. This milestone therefore does not invent a crate release/version bump or publish a release artifact.

## Verification plan and evidence

New regressions compare exact tagged JSON, deserialize the old wire shape, verify borrowed payload equality, check reduced inline layout, and retain raw validation errors plus exact task/run correlation failure. Existing scheduler suites and workflow-service/embedded-runtime callers remain in hosted qualification.

Rust formatting and whitespace checks pass locally. No local workspace compilation or executed new Rust test is claimed; source review and fresh hosted scheduler/Clippy/workflow tests are required. Existing aggregate gates are unchanged, and this is an internal layout/source-API repair rather than the future research scheduler redesign.

Independent source review accepted tree e3f247a05873d68632044d1078b7490b76529fbf. Fresh hosted compilation/runtime qualification remains required. The layout assertion checks inline size only and is not evidence of measured execution-performance improvement.
