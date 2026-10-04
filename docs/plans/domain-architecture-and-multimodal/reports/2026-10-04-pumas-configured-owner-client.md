# Configured Pumas owner attachment

## Scope and source

The September 9 M3 Owner/LocalClient design in the canonical plan admits this
bounded consumer slice. Base: merged main
`11b046fa23da5715d69e33e98c41cb9aad792f9e`, tree
`5080b74e82e5ef7bb62741d7fc03cc4754ea1f4a`. Branch:
`feature/pumas-configured-owner-client`. Implementation and tests change only
`crates/workflow-nodes/src/setup.rs`; accepted PR48 source is untouched.
This historical attachment slice consumed Pumas revision
`f87c3da8276a914a54c6f4f36d617bef9d9f424e` and made no manifest or lockfile
changes. The integrated consumer historically pinned
`5be6d967dbd5c0ff7449f342e77f05f9cd645a8e` and currently pins
`2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`; its dependency upgrade and
subsequent qualification receipts are recorded in the later consumer reports.

When the configured launcher root already has an owner, setup now tries its
typed LocalClient before opening ReadOnly access. It compares canonical
launcher-root paths and preserves configured-root order. Explicit configuration
cannot fall through to another globally discovered library. Global client/owner
discovery remains available when no root is configured. Direct model-index
ReadOnly access still does not construct an owner. Attached clients are not
registered as Pantograph-owned Pumas APIs.

Four new tests cover the real Pumas owner's selector IPC and invalid-token
rejection, client-drop ownership, root filtering, absent credentials, missing
roots, unconfigured discovery, and Unix path aliases. The transport fixtures
use actual loopback framed IPC with a controlled selector dispatcher; they
are not model inference or producer package-facts qualification. The real-owner
test exercises public setup and the production Pumas dispatcher against an
empty temporary library, without Hugging Face or process management.

## Verification and limits

- `npm test`: node-engine 258 passed, one existing ignored; workflow-nodes
  168 passed. These default features exclude `model-library` and therefore do
  **not** execute or type-check the changed feature code.
- `cargo fmt --all -- --check`, the critical antipattern gate, and
  `git diff --check`: passed.
- Traceability scanner tests: 28 passed. The declared staged gate passed for
  these two paths, with zero mapped impacts. Semantic contract review remains
  required; the scanner is not architectural acceptance.
- Required feature command:
  `cargo test --locked -p workflow-nodes --features model-library --lib`.
  The native independent reviewer, reported by the coordinator, executed the
  actual feature-enabled suite at exact head
  `03fb03943060abbb0d505f79b769a74fb4ec9b5f`: **208 passed**, including all four
  new real-owner/IPC cases. The declared default gate also passed there:
  258 node-engine and 168 workflow-nodes tests, with one inherited benchmark
  ignore. The reviewer accepted bounded configured-owner attachment without
  source changes. Native qualification used the existing verified ORT runtime.
- This cloud environment did not execute the feature-enabled suite. The approved
  saved-domain settings were already applied, but this cloud path still returns
  403 for the required ORT download. There is no pending network update. No
  network settings, credentials or dependency features were changed here.
- A small successor adds the exact feature command immediately after the
  existing workflow-nodes default test step in Quality Gates, preserving that
  step. Its hosted CI result remains pending coordinated publication/review.
- No GUI, real model load, model inference, CUDA execution, or multimodal
  workflow was executed. Current Coding-Standards MCP is unavailable in this
  environment; current MCP compliance is not claimed.

Configured-owner attachment is independently accepted at `03fb0394`. Full-facts
transport, M3 execution and complete product functionality remain unaccepted.

## Executable next sequence for the coordinator

1. Keep the feature command above in CI and qualify its successor at the actual
   published head. Native attachment qualification is complete. The real-owner
   test remains mandatory; socket failure fails these new tests rather than
   silently skipping them.
2. Admit the assessed Pumas dependency update separately. Merged upstream main
   `58b74e83fdf34131290933f576c7e338bde4a49d` already contains the registry
   startup repair. Its changed download-progress Result needs explicit error
   propagation in UniFFI and Rustler before downstream acceptance. That
   assessment does not constitute a tested dependency upgrade.
3. Coordinate the Pumas lane's authenticated typed full-package-facts operation
   and freeze its actual merged revision. Neither the current pin nor assessed
   upstream main exposes it in LocalClient. Then migrate the existing dispatch
   and host facts/target consumers under the reviewed M3 design; preserve
   ReadOnly execution refusal, selected-artifact identity, freshness, and
   owner-only shutdown. This attachment slice does not remove their current
   Owner-only execution restriction.
   The coordinator's current producer candidate
   `40c035705641e344e8b4c91698cffea53b47e2e0` adds a closed
   `LocalIpcOperation` wire-name enum. At that revision the producer must also
   register the missing operation in existing `ipc/protocol.rs`; the existing
   dispatch branch is otherwise unreachable through framed ingress. Its client
   preserves RPC `-32602` as `InvalidParams`, maps other server errors to
   `Other`, and preserves transport loss as `SharedInstanceLost`. These are
   current-candidate observations, distinct from the consumed `f87c3da` and
   assessed merged `58b74e83`. The Pumas lane owns this producer change and
   actual framed-IPC qualification.
4. Obtain legitimate producer identities and complete package evidence for real
   text/image models, then execute the existing canonical scheduler path and
   retain both outputs under one run. GUI/display and model prerequisites remain
   external. Historical Library continuation goals refer to an intent-first
   development branch whose intent client is absent from this main; reconcile
   that authority before adding a new transport or restoring an old path.
5. Review a minimal native embedding slice before implementation: existing
   Candle adapter, CPU F32, one explicitly supported BERT/task recipe, correct
   masking/pooling/normalization, and honest readiness only after executable
   construction. The canonical runtime host currently branches for text and
   otherwise projects image generation; it needs a selected-model embedding
   branch through the existing gateway and retained-output contracts. Qualify
   real single/batch results against an independent reference, then prove
   generated text fans out to image and embedding tasks. Remove Candle's
   unreachable HTTP embedding path only after native parity is established.
6. After real timing/resource baselines exist, review a bounded scheduler slice
   using existing admission and ownership: evaluate compatible candidates
   without reservations, commit the selected reservation once, then compare a
   simple completion estimate against the existing policy. Preserve eligibility,
   priority/aging, cancellation, and single-gateway serialization. No replicas,
   split/offload execution, continuous batching, or deep search is admitted by
   this proposal.

The owner's Lean 4 commitment applies alongside that functional scheduler
slice. First target: for a finite lease ledger, prove per-resource budget
preservation under atomic grant, owner-bound update (excluding the previous
claim), and exact-lease release. Map this to the existing runtime registry's
`can_acquire_reservation`, locked `acquire_reservation`, update and release
operations; reuse its admission owner. Second, when ranking is admitted, prove
that a finite minimum selects only an eligible feasible candidate and obeys
the declared deterministic tie rule. Keep the initial formalization bounded
to these transition/selection lemmas, with a pinned Lean 4 toolchain and explicit
assumptions. No Lean/Lake executable is available here and no proof ran.
Natural-number or abstract-state proofs do not certify Rust refinement, u64
overflow handling, floating-point estimates, hardware envelopes, cancellation
acknowledgement or performance. Those still need actual dispatch-path tests
and real workload measurements. Formalization must not delay the earlier
working inference and producer/consumer slices.

The October 2 compatible-inference book proposes the native embedding rollout;
it executed no models. The newer October scheduler v2 evaluation is a standalone
CPU discrete-event simulator, separate from April Scheduler V2 in Pantograph.
It modifies no Pantograph code, runs no neural networks, and reports substantial
search cost and budget-exceeded cases. These research artifacts inform the
review proposals above; they are not approved production specifications or
hardware evidence. The canonical plan remains the sequencing owner.

Research source identities: compatible-inference Markdown
`libfile_fe49f57b568481918a285086f54f138d`; scheduler thesis Markdown
`libfile_6c1d9aa6e1f08191b4a14e0de0846220`; scheduler v2 evaluation Markdown
`libfile_8b6d205bd260819187ef9fbac029a243`. The inspected Library continuation goal
is `libfile_ded6c24da2948191b923ec25833e9414`.
