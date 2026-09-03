# Plan: Close Dynamic-Code Trust Bypasses

**Plan status:** `Planned`

**Current phase:** Milestone 0 — model-code authorization

**Next slice:** Milestone 0 — carry the Rust-owned trust decision through image planning and remove unconditional Diffusers trust

**Acceptance status:** `pending`

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

**Source audit:** [Security and dynamic code](../../../audits/2026-09-03-current-standards/01-security-and-dynamic-code.md)

**Baseline:** Pantograph `97c04827`; Coding-Standards `82a0ddf315a08364357f6564018e37bdbeb72a1a`

**Reports:** none.

**Related ADRs:** [ADR index](../../../adr/README.md)

## Objective

Prevent model-package code and generated Svelte source from executing through a
permissive default, stale decision, unavailable validator, or main-renderer
import. Ordinary model loading remains closed by default; custom model code
requires authorization bound to exact package provenance. Generated source and
history remain available, but execution is disabled until a separately planned,
capability-free execution boundary is proven.

## Objective Acceptance

| ID | Observable criterion | Kind | Environment | Mode | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| SDC-A1 | Diffusers and existing Transformers loaders derive `trust_remote_code` only from the Rust-owned policy; missing, denied, incomplete, or mismatched authorization cannot call a custom-code loader. | `focused` | `not-applicable` | `automated` | `pending` | Milestone 0 |
| SDC-A2 | Image worker contracts preserve the applied decision and exact source/revision/artifact identity, reject malformed or unknown fields, and do not reuse a diffusion cache across an identity or decision change. | `contract` | `simulated` | `automated` | `pending` | Milestone 0 |
| SDC-A3 | Generated-source validation returns typed `invalid`, `unsupported`, or `unavailable`; failure or timeout leaves the destination and generated history unchanged. | `integration` | `representative` | `automated` | `pending` | Milestone 1 |
| SDC-A4 | A timed-out validator is terminated and reaped before completion; repeated timeouts leave no validator thread or child. | `system` | `representative` | `automated` | `pending` | Milestone 1 |
| SDC-A5 | Agent update, startup restore, HMR, retry, undo, and redo never import or render a generated module in the main renderer and retain a typed `execution_unsupported` state. | `integration` | `representative` | `automated` | `pending` | Milestone 2 |
| SDC-A6 | A hostile component reaches discovery in the real isolated desktop harness, but its top-level marker and Tauri call do not execute; the app records the unsupported state and remains usable. | `system` | `representative` | `automated` | `pending` | Milestone 3 |

Static checks and source searches support these claims but cannot replace the
contract, process-lifecycle, or desktop evidence.

## Scope

### In Scope

- Diffusers trust policy from Rust image plan through worker load, including
  single/batch requests and diffusion cache identity.
- Preservation of the existing closed Transformers/ASR policy while using the
  same Rust authority rather than adding a Diffusers-specific Boolean.
- Generated-component write, validate, restore, update, HMR, retry, undo/redo,
  import, registry, and render paths.
- Validator ownership, typed outcomes, persisted-source proof invalidation, and
  direct evidence for the affected boundaries.

### Out Of Scope

- Restoring live generated-component execution; this requires a new plan and
  ADR for process/renderer isolation and an explicit message contract.
- A user-facing custom-code opt-in, model acquisition/signing, general IPC
  decoding, scheduler/binding redesign, or repository-wide CSP/network policy.
- GPU/image-quality proof and migration of generated history storage.

## Constraints And Assumptions

- Generated Svelte source is untrusted and cannot share the main renderer's
  JavaScript realm or Tauri authority.
- `ModelLoadSecurityPolicy` remains the model trust owner. Package facts describe
  code requirements; they do not authorize execution.
- Custom-code opt-in requires a decision ID, coverage of discovered code sources,
  matching source/code revisions, canonical model identity, and selected artifact
  fingerprint. Missing required identity is `unavailable`; denial or a valid but
  unsupported package is `unsupported`; contradiction/malformed input is
  `invalid`.
- Rust and its embedded Python worker are replaced atomically, so a current-
  format worker contract bump has no old/new overlap.
- Compiler/import validators are required. Lint/design checks may remain
  advisory but cannot authorize writes or execution.
- Existing generated files/history are retained. A prior Boolean/cache entry is
  not proof; current bytes must be re-admitted.
- If package facts cannot supply immutable provenance or the desktop harness
  cannot run, acceptance is blocked rather than weakened.

## Binding Decisions

| Decision | Owner | Evidence | Supersedes |
| --- | --- | --- | --- |
| Apply `ModelLoadSecurityPolicy` to current package facts before image planning and pass a proof-bearing result to Python. | `crates/inference` | SEC-01 and existing Transformers policy | Diffusers `trust_remote_code=True` |
| Keep model-code and generated-UI trust as separate authorities; share outcome vocabulary only. | Inference and Tauri owners | Different identities, consumers, and lifecycles | Global dynamic-code trust flag |
| Consolidate component admission in `src-tauri::hotload_sandbox`; bind it to content digest and contract version. | Tauri hotload owner | SEC-02 | Independent write/frontend validation decisions |
| Delete the Boa semantic validator; it neither validates the full Svelte contract nor isolates execution. | Tauri hotload owner | SEC-03 | Detached native validation thread |
| Remove generated-module import/render from the main renderer and retain source/history with `execution_unsupported`. | Desktop composition | SEC-02/SEC-04 | `SafeComponent` as an isolation boundary |
| Validation rejection is `invalid`, unsupported policy/execution is `unsupported`, and missing/failed/timed-out capability is `unavailable`; none permits fallback execution. | Boundary owners | Security and Contracts standards | Generic error followed by success |

Milestone 0 records these durable decisions in the planned
`docs/adr/ADR-017-dynamic-code-trust-and-isolation.md`.

## Evidence And Oracle Plan

| Claim | Deciding oracle | Independent authority | Intended negative result |
| --- | --- | --- | --- |
| SDC-A1/A2 | Rust-applied authorization, Rust-produced fixtures decoded by Python, and fake loader call capture | Rust policy plus Pumas facts/load target | Typed rejection before loader/cache use |
| SDC-A3 | Real repository Node validators against isolated temp roots | Svelte/esbuild result decoded by backend | Exact typed result; no file/history mutation |
| SDC-A4 | Hanging test validator records a PID; owner observes kill and reap | OS child terminal state | `unavailable` only after termination |
| SDC-A5 | Table of every registry entry path with a module side-effect counter | Registry transition contract | `execution_unsupported`; no constructor/import |
| SDC-A6 | Isolated Tauri/WebDriver project, discovery diagnostic, and harness-owned marker | Real desktop process boundary | Discovered source, absent marker, responsive app |

Negative fixtures change one fact from a valid case. Generic failure or an absent
marker without proof that discovery occurred is invalid evidence.

## Systemic Finding Audit

- **Owners:** `ModelLoadSecurityPolicy` owns Python model policy;
  `src-tauri::hotload_sandbox` owns generated-source admission; desktop
  composition owns execution capability.
- **Bounded consumers:** all Python `from_pretrained` calls affected by the shared
  policy, image plan/envelopes/cache, agent write and `validate_component`, and
  generated source update/restore/HMR/retry/undo/redo/import/render paths.
- **Dispositions:** Diffusers joins the existing model policy; write/frontend
  validation becomes one fail-closed admission; Boa and main-renderer import are
  deleted; stored source remains non-executable.
- **Alternatives rejected:** another trust Boolean, regex/lint/Boa as security
  authority, unsafe feature flag, and speculative iframe/webview isolation.
- **Stopping condition:** every bounded consumer has a disposition, SDC-A1–A6
  pass, and no unconditional remote-code trust, generated import, or old
  validation cache remains.

## Simplicity And Ownership Review

**Applicability:** `applicable`

- Independent concepts and dimensions: package facts describe code; policy expresses intent;
  applied authorization binds intent to identity; component admission validates
  stored bytes; execution capability decides whether bytes may run.
- State, identity, value, time, policy, and mechanism: model policy, package provenance, component digest,
  worker current-format version, and validation-contract version stay separate.
  Admission state and cached values are bound to those identities; deadlines
  belong to the validator owner. Changing identity, policy, bytes, or applicable
  version invalidates its proof.
- Caller and composition-root knowledge: runtime host supplies policy and package facts; Python
  cannot choose trust. Frontend receives typed metadata, never a generated
  component constructor.
- Representative change paths and forced owners: a new Python loader consumes applied authorization;
  a validator changes one admission owner/version; future live preview changes a
  separately planned execution boundary.
- Stable Interfaces versus hidden knowledge: applied authorization hides provenance checks;
  component admission hides process/digest mechanics, and typed unsupported
  state hides renderer details from callers.
- Independent evolution, testing, failure, and replacement: model authorization,
  component admission, and desktop non-execution have independent evidence and
  can fail or be replaced without creating an alternate trust authority.
- Necessary complexity and containment: provenance and child termination remain in their
  owners. Deleting Boa, `SafeComponent`, importer, and validation cache offsets
  the two small proof-bearing result types; no plugin framework is added.
- Deletion and cumulative machinery result: unconditional worker trust,
  fail-open validation, main-renderer import machinery, and obsolete cache paths
  are removed; only the Rust-owned authorization and fail-closed admission
  contracts remain.

## Order Rationale

1. Close active critical Diffusers trust first.
2. Establish one typed, lifecycle-owned component admission.
3. Remove renderer execution while preserving diagnosable source state.
4. Prove the completed desktop path after backend and frontend semantics settle.

## Milestones

Plan-control writes are limited to the three files in this directory. Milestone
write sets below are exhaustive; a newly required source path triggers re-plan.

### Milestone 0: Apply Model-Code Authorization

**Goal:** Diffusers cannot enable or reuse custom code without exact Rust-owned
authorization.

**Allowed write set:** `docs/adr/ADR-017-dynamic-code-trust-and-isolation.md`;
`crates/inference/src/{model_contracts.rs,image_generation_planner.rs,image_generation_planner_tests.rs,image_generation_batch_tests.rs,gateway_tests.rs}`;
`crates/inference/src/backend/{pytorch.rs,pytorch_tests.rs,pytorch_worker_contract.rs,pytorch_worker_image_contract.rs,pytorch_worker_image_contract_tests.rs,pytorch_image_generation.rs,pytorch_image_generation_tests.rs,pytorch_worker_image_python_tests.rs}`;
`crates/inference/torch/{worker.py,worker_contract.py,worker_image_contract.py}`;
`crates/inference/tests/fixtures/pytorch_worker_contract/{load_transformers_model_request.json,generate_image_request.json,generate_image_batch_request.json}`;
`crates/pantograph-embedded-runtime/src/runtime_host_image_execution.rs`.

**Tasks:**

- [ ] Construct an applied authorization from policy plus current package/load-
  target provenance, with exact typed failures.
- [ ] Carry it through image plans, batch members, versioned worker DTOs, fixtures,
  and complete Python decoding; remove the Diffusers literal `True`.
- [ ] Bind diffusion cache reuse to model/artifact/decision identity and preserve
  existing Transformers/ASR closed behavior.
- [ ] Test deny, no-code, missing identity, source/revision mismatch, exact allow,
  unknown fields, single/batch projection, and changed cache identity.

**Gate:** SDC-A1/A2; affected inference focused/contract tests and `cargo check`
and format supporting gates.

**Migration/rollback:** current-format worker version bump, no overlap. Rollback
keeps all Python loaders at `trust_remote_code=False`; it never restores `True`.

**Re-plan if:** provenance is unavailable, another policy owner/loader appears,
or worker versions deploy independently.

**Status:** `Planned`

### Milestone 1: Make Component Admission Fail Closed

**Goal:** One backend operation validates current bytes with typed results and
owned child-process completion.

**Allowed write set:** `src-tauri/src/hotload_sandbox/{component_validation.rs,mod.rs,runtime_sandbox.rs,README.md}`;
`src-tauri/src/agent/tools/{write.rs,write_validation.rs,error.rs}`;
`src-tauri/src/llm/commands/sandbox.rs`; `src-tauri/src/config.rs`;
`src-tauri/Cargo.toml`; `Cargo.lock`;
`scripts/{validate-svelte.mjs,validate-esbuild.mjs}`.

**Tasks:**

- [ ] Add `GeneratedComponentAdmission` with status, digest, contract version,
  and bounded diagnostics; use it for agent writes and Tauri validation.
- [ ] Make compiler/import checks required and fully decode their output.
- [ ] On timeout/cancellation kill and reap validation work before returning;
  keep destination/history unchanged for every non-valid outcome.
- [ ] Delete Boa and label retained lint/design checks non-authorizing.
- [ ] Test real valid/invalid validators, missing executable, malformed output,
  nonzero exit, timeout/repeat, containment, digest change, and no mutation.

**Gate:** SDC-A3/A4; affected Tauri tests with real Node validators, path-security
tests, `cargo check -p pantograph`, and format.

**Migration/rollback:** discard old validation cache/proof, retain source/history.
Rollback keeps execution disabled and validator failure non-authorizing.

**Re-plan if:** validator descendants cannot be reaped on a supported platform
or validation has an independently deployed compatibility requirement.

**Status:** `Planned`

### Milestone 2: Remove Main-Renderer Execution

**Goal:** Generated-component consumers project source metadata and diagnostics,
not executable modules.

**Allowed write set:** `src/lib/hotload-sandbox/{types.ts,generatedComponentAdmission.ts,generatedComponentAdmission.test.ts,index.ts}`;
`src/lib/hotload-sandbox/services/{ComponentRegistry.ts,ComponentRegistry.test.ts,ImportManager.ts,GlobRegistry.ts,ValidationCache.ts}`;
`src/lib/hotload-sandbox/components/{SafeComponent.svelte,ComponentContainer.svelte}`;
`src/services/{HotLoadRegistry.ts,RuntimeCompiler.ts}`;
`src/components/{HotLoadContainer.svelte,CommitTimeline.svelte,Toolbar.svelte,TopBar.svelte,SidePanel.svelte}`;
`src/shared/components/index.ts`;
`src/App.svelte`; `package.json`.

**Tasks:**

- [ ] Decode the admission response from `unknown` through its complete contract.
- [ ] Remove component constructors, Vite/dynamic import, import/cache wrappers,
  and `SafeComponent`; do not retain an unsafe feature flag.
- [ ] Route update/restore/HMR/retry/undo/redo to typed registry state and preserve
  source/history plus `execution_unsupported` diagnostics.
- [ ] Test all entry paths, malformed response, changed bytes, stale completion,
  and a top-level import marker. Register tests in the current canonical scope;
  if verification work changed that owner, re-plan rather than restore a list.

**Gate:** SDC-A5; focused/integration tests and affected TypeScript/Svelte checks.

**Migration/rollback:** retain generated data and ignore old proof. Rollback may
remove the source UI, but cannot restore main-renderer import.

**Re-plan if:** another executable consumer exists or disabling execution changes
an established public promise.

**Status:** `Planned`

### Milestone 3: Prove Desktop Non-Execution

**Goal:** Prove hostile generated source remains non-executable through real
startup and update discovery, then align claims.

**Allowed write set:**
`tests/e2e/generated-component-safety/{generated-component-safety.e2e.mjs,wdio.conf.mjs,fixtures/HostileGeneratedComponent.svelte}`;
`scripts/check-generated-component-safety-gui.sh`; `package.json`;
`docs/runtime-operations.md`;
`docs/adr/ADR-017-dynamic-code-trust-and-isolation.md`.

**Tasks:**

- [ ] Launch Tauri with an isolated root containing the hostile fixture; prove
  discovery/diagnostic, absent side effects/Tauri call, and app responsiveness.
- [ ] Repeat through runtime update/HMR and align docs with non-execution.
- [ ] Hand the command and SDC claim IDs to the verification plan's scheduling
  owner; do not call source-only evidence a desktop test.

**Gate:** SDC-A6 plus continued SDC-A1–A5. Missing representative desktop
capability blocks acceptance without fallback.

**Migration/rollback:** test state is temporary; user data is untouched. Any
rollback preserves disabled execution.

**Re-plan if:** discovery is not observable, the harness requires weaker
production security, or an isolation design is introduced.

**Status:** `Planned`

## Cross-Plan Dependencies

- [`current-image-generation-graphs`](../../current-image-generation-graphs/plan.md)
  and the
  [`architecture/lifecycle remediation`](../architecture-lifecycle-and-bindings/plan.md)
  own the image path and existing policy/facts; this plan specializes rather
  than duplicates them.
- The audit-02 architecture plan may overlap
  `runtime_host_image_execution.rs`; integrate Milestone 0 first or preserve its
  authorization contract.
- The [audit-03](../../../audits/2026-09-03-current-standards/03-frontend-and-accessibility.md)
  plan owns general IPC decoding; this plan owns only the changed admission DTO.
- The [audit-04](../../../audits/2026-09-03-current-standards/04-verification-and-tooling.md)
  plan owns discovery/CI scheduling and must retain SDC-A1–A6 fidelity.
- Dependency/release remediation must serialize its lockfile work with Boa
  removal. These are coordination dependencies, not current blockers.

## Risks

| Risk | Control |
| --- | --- |
| Existing Diffusers custom code becomes unavailable | Typed rejection and complete opt-in; never restore implicit trust |
| Same-path cache survives changed authority | Include the applied closure in cache identity and test downgrade/change |
| Validator child descendants survive | Representative kill/reap evidence on supported targets |
| Hidden frontend importer remains | Bounded path table plus real desktop discovery test |
| “Validated” later becomes “safe to execute” | Separate admission/execution types, ADR, docs, and deferred owner |

## Blockers

- `none`

## Re-Plan Triggers

- Required provenance, process control, owner, consumer, or representative
  evidence differs from the assumptions above.
- Another dynamic loader/executor or persisted proof enters the bounded family.
- A milestone misses its gate, another active plan changes shared authority, or
  the repaired composition grows materially beyond this review.
- Live generated execution becomes required or a proven isolated executor is
  proposed.

## Concurrent Work

No concurrent implementation is authorized. Shared worker contracts, runtime
host, lockfile, package scripts, and verification ownership require serial
integration. Add bounded delegation only through re-planning.

## Final Acceptance

- Acceptance status: `pending`
- Deferred: live generated-component execution, owned by a future desktop
  architecture/security plan and triggered by an accepted isolated-execution
  requirement.
- Deferred: general CSP/custom-command/localhost posture, owned by a future
  desktop/network security plan and triggered by new untrusted renderer content
  or changed listener exposure.
- Final status: `Planned`
