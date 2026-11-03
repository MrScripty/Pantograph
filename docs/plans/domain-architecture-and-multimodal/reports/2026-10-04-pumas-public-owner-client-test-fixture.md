# Pumas public owner/client test fixture repair

Base: consumer repair `129eb57ec9ca94e350899b53c9bede298148fd05`.
Branch: `fix/pumas-public-owner-client-test-fixture`. Frozen embedding output
`1f7ad59b93a1651064c92e2ab52df3ad9e435620`, its separate scheduler-ID test repair,
and pretrained qualification remain separate. Pumas stays pinned to
`5be6d967dbd5c0ff7449f342e77f05f9cd645a8e`, with its full default feature profile.

Hosted PR49 reported E0432/E0603 in `workflow-nodes/src/setup.rs`: the fixture
imported a removed `IpcDispatch` export and now-private `IpcServer`. The actual
cloud feature-enabled test-target compiler reproduced both errors before editing.

The repair changes only the existing `#[cfg(all(test, feature = "model-library"))]`
module. It replaces the two fabricated dispatchers and forged ready-instance
entries with the public owner builder, registered-instance discovery,
authenticated `PumasLocalClient` and public import API. The small GGUF import is
metadata evidence, not an inference model. The five affected tests remain enabled:
configured-root selection and client-drop survival, refusal to attach another
library, canonical-root aliases, selector-cursor update recovery, and selected
detail hydration through batch methods. They compare real owner-produced results
and model identity; update recovery allows additional legitimate owner events.
Owner/client creation failures now fail assertions instead of silently skipping.
The summary cache is warmed before exact owner/client comparison: the producer
reports a different status for initial regeneration versus a fresh cached lookup.

No production source, dependency features, transport internals, admission gates,
credentials, permissions or network configuration changed. The public builder
disables optional HF client/process-manager initialization for isolated fixtures;
this does not reduce compiled dependency features or assert runtime inference.

## Compiler receipts and execution limits

Executed with the locked dependency graph, Rust 1.92.0, offline Cargo and one job:

```sh
ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo check --locked --offline -p workflow-nodes --features model-library --tests
ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo clippy --locked --offline -p workflow-nodes --all-targets --all-features
```

Before repair: test-target check exit 101, E0432 and E0603. Repaired test-target
check and package all-target/all-feature Clippy exit 0; the existing inference
`SelectedTextLoad` unused-field warning remains. These are actual package target
compiler checks, not a boundary harness, and not test execution. The package
Clippy check is not the broader hosted warning-deny workspace audit.

A same-feature setup-test execution was attempted separately:

```sh
ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo test --locked --offline -p workflow-nodes --features model-library --lib setup::tests -- --test-threads=1
```

The test binary cannot link: `ort-sys` reports the missing ONNX Runtime binary,
and `OrtGetApiBase` is unresolved. Exit 101; **no setup test executed**. The
handoff retains before/after compiler, Clippy and final link-attempt receipts.
`ORT_SKIP_DOWNLOAD=1` avoids the currently inaccessible ORT acquisition; the
attempt confirms this is a linking blocker, not a replacement for a working ORT
binary. No real IPC runtime execution, native host, GUI, GPU or pretrained
inference is asserted by this slice. Runtime assertions need the parent's
authorized native/hosted qualification environment.

Rust formatting, critical lint, whitespace and staged decision traceability were
checked separately. Frozen source identities and full PR44 ancestry are retained.
