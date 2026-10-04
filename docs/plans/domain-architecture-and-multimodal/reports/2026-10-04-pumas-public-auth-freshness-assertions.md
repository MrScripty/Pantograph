# Public owner/client authentication and freshness assertions

Branch: `fix/pumas-public-auth-freshness-assertions`.
Exact base: `645889a969cc025f604001935da7d60770992b5f`.
The full stacked PR44 ancestry, including
`ca9edde6850cd58ade0b7e534bb4f58e704c2c64`, is preserved.

## Failure evidence and bounded repair

The parent reports that hosted Quality run `37199095601` executed the consumer
suite: **207 passed, 2 failed, 0 ignored**. The failures were the legacy private
invalid-token message assertion and equality of summary results with legitimate
`regenerated` versus `fresh` statuses. Real owner/client full package facts and
guarded-target IPC passed. Other Quality jobs, strict Clippy, Headless and Runtime
Separation passed according to the parent. These are parent-reported receipts,
not local executions. A read-only GitHub run-log request returned Forbidden;
no alternate route or permission/network change was attempted.

Only the two affected assertions and their lookup ordering change inside the
existing `model-library` test module in `crates/workflow-nodes/src/setup.rs`:

- Wrong-token rejection requires `PumasError::InvalidParams` with the public
  sanitized message `Invalid local IPC parameters`. The authenticated positive
  control, token-only mutation and owner-survival checks remain.
- Selected detail now hydrates a cold summary through real public IPC and must
  report `Regenerated`. The following owner batch lookup must report `Fresh`.
  Both exact model identities and the complete nonempty summary payload must
  match. A further public-client lookup must match the entire warm owner result,
  including freshness. Complete selector-row and descriptor comparisons remain.

Producer source is pinned to `5be6d967dbd5c0ff7449f342e77f05f9cd645a8e`:
`rust/crates/pumas-core/src/ipc/protocol.rs` sanitizes the private token failure
and maps RPC code -32602 back to `InvalidParams`. The summary resolver in
`src/model_library/library.rs` returns `Regenerated` when it populates package
facts and `Fresh` when the summary fingerprint matches. The result DTO has three
fields: `model_id`, `status`, `summary`; all three remain checked. The selector
snapshot is a bounded cache observation and does not regenerate package facts.

The public owner builder, ready-instance discovery, authenticated local client,
real GGUF import and full-facts/guarded-target fixture remain unchanged. All 14
existing setup tests remain enabled. No production implementation, model bytes,
dependency pin/profile, admission or audit/traceability gate changes.

## Actual local verification

The following passed with locked offline dependencies and one build job:

```sh
ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo check --locked --offline -p workflow-nodes --features model-library --tests
ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo clippy --locked --offline -p workflow-nodes --all-targets --all-features
cargo fmt --all -- --check
git diff --check
node scripts/check-critical-antipatterns.mjs
```

Clippy reports the existing inference unused-field warning; this is not a strict
warning-deny workspace run. Decision traceability also passed for the staged
test/report delta. No gate was bypassed or altered.
The first traceability invocation lacked the worktree's Node dependencies;
linking the existing checkout dependency directory resolved that cache gap.
Both the failed invocation and passing receipt are retained.

Actual same-feature runtime attempt:

```sh
ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo test --locked --offline -p workflow-nodes --features model-library --lib setup::tests -- --test-threads=1
```

It exits **101 at link time** because ONNX Runtime is unavailable, including
undefined `OrtGetApiBase`. **Zero setup/IPC tests executed locally on this new
head.** Compiler checks are not runtime evidence. Hosted CI must execute the
revised cold-to-warm assertions before acceptance. No mock inference, pretrained
model inference, GUI or GPU checks were substituted or executed.

The earlier composition `7c5370c81354af2953ccd699d039d78fd4d6ba81` is frozen and
on hold. The new consumer repair remains separate for independent qualification
before coordinated composition with accepted embedding head
`34bd2655c3c7f78b0d5b0d2a0ab318b7a5d4a914`. Pooling repair
`495efea606620153fe2f0363b104e56518436a06` remains separate. No main merge,
PR advancement or external review request was made.
