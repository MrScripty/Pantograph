# Workflow embedding output test identity repair

Base: frozen embedding output `1f7ad59b93a1651064c92e2ab52df3ad9e435620`.
Branch: `fix/workflow-embedding-test-identities`. Consumer and pretrained
qualification branches remain separate. No production code changed.

The actual workflow-service test compiler reproduced four E0277 errors in
`task_result_output_projection.rs`: scheduler workflow, run, node and task IDs
do not implement `From<&str>`. The repair uses each existing validated `parse`
constructor and the same fixture identities, matching the adjacent test helper.

Executed in the published cloud checkout with Rust 1.92.0, locked/offline
dependencies, unchanged default features and one build job:

```sh
ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo check --locked --offline -p pantograph-workflow-service --tests
```

Before repair: exit 101, exactly four E0277 errors at lines 225–228. After repair:
exit 0, finished in 42.36 seconds. The only warning is the previously recorded
unused `SelectedTextLoad` fields in inference. Receipts are
`workflow-embedding-identity-before-check.log` and
`workflow-embedding-identity-after-check.log` in the handoff archive.

Rust formatting, critical lint, whitespace and staged decision traceability
were checked separately. This is the actual crate test-target compiler, including
the affected test module; it is compilation only. `ORT_SKIP_DOWNLOAD=1` skips the
unavailable ORT acquisition without reducing features. No linking, test execution,
native host runtime, GUI or pretrained inference is claimed by this repair.
