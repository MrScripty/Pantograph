# Focused Audit: Security And Dynamic Code

Implementation plan: [Close dynamic-code trust bypasses](../../plans/current-standards-remediation/security-and-dynamic-code/plan.md)

## Scope

This audit covers code obtained from model packages, generated Svelte source,
runtime validation, renderer execution, filesystem authorization, and the
trust decisions that must exist before code or resources are used.

Applicable current standards are Core, Security, Contracts, Resilience,
Architecture, Frontend, IPC, Rust Security, and Verification.

## Assessment

Pantograph has a good fail-closed Transformers model policy and a strong
filesystem containment module. Those controls are not consistently applied to
Diffusers or generated UI code. One critical trust bypass is verified; the
generated-component path is a high-risk unresolved trust model.

## Findings

### SEC-01 — Critical: Diffusers always trusts model-supplied code

[worker.py](../../../crates/inference/torch/worker.py#L1041) calls
DiffusionPipeline.from_pretrained with trust_remote_code enabled
unconditionally. Both batch execution near line 1630 and single-image
execution near line 1731 call this loader.

The Rust-owned [image plan](../../../crates/inference/src/image_generation_planner.rs#L316)
and [worker request](../../../crates/inference/src/backend/pytorch_worker_image_contract.rs#L158)
carry no trust decision. The contract test near line 219 of
pytorch_worker_image_contract_tests.rs explicitly requires a trust_remote_code
field to be rejected as unknown.

This means the active image path can execute code from a model package without
an authorizing policy, source/code revision, or auditable decision identity.
The Transformers path in pytorch.rs near lines 1220-1279 already demonstrates
the desired closed-by-default design.

Required direction: stop unconditional trust first. Reuse one Rust-owned trust
contract for all Python loaders and require exact package/code provenance
before any opt-in execution.

### SEC-02 — High: generated-component validation fails open

[ImportManager.ts](../../../src/lib/hotload-sandbox/services/ImportManager.ts#L148)
turns every validation-system exception into valid: true. The accepted source
is then loaded through Vite or a dynamic import and rendered directly in the
application.

[SafeComponent.svelte](../../../src/lib/hotload-sandbox/components/SafeComponent.svelte#L76)
states that its global error listener is not an isolation boundary. It renders
the generated component in the main renderer and leaves iframe isolation as a
future consideration.

The backend write path also contains fail-open outcomes:

- write.rs near lines 307-326 writes a component when the validator cannot be
  launched;
- write_validation.rs near lines 248-254 converts failed import-validator
  execution to success; and
- config.rs near lines 95-142 defaults import validation and lint controls off.

This contradicts the local READMEs' claim that generated components cannot
bypass validation. No focused tests cover ImportManager, SafeComponent, or
validation-system failure.

Required direction: define whether generated source is trusted or untrusted.
Until that decision is explicit, validation unavailability must not authorize
execution. If source is untrusted, validation alone is not isolation.

### SEC-03 — High: validator timeout does not stop the work

[runtime_sandbox.rs](../../../src-tauri/src/hotload_sandbox/runtime_sandbox.rs#L150)
starts a native thread, returns Timeout when its polling deadline expires, and
explicitly cannot kill or join the timed-out thread. Repeated hostile or hung
inputs can therefore create work that outlives the request and is not owned by
shutdown.

The same validator treats several syntax/import failures and unknown runtime
errors as successful validation. That may be acceptable only as a narrowly
documented capability limitation; it cannot authorize code execution.

Required direction: make validation capability and terminal state explicit.
Use bounded, cancellable isolation or return unavailable without executing the
candidate.

### SEC-04 — Unresolved: renderer and local-network threat model

Tauri grants a narrow declared capability set, which is a strength. The CSP
still allows inline scripts/styles and localhost HTTP/WebSocket connections.
Whether generated code can reach sensitive custom commands or trusted local
services needs a concrete desktop threat model and an actual hostile-component
test. This audit does not claim an exploit.

## Preserved Strengths

- pantograph-path-security centralizes root containment and tests traversal and
  symlink escape.
- Transformers custom-code loading defaults closed and carries a Rust-owned
  trust policy.
- Tauri capability declarations are narrow.
- Vite binds to loopback by default.

## Follow-Up Audit Questions

1. Which actors and persisted sources can supply model or generated UI code?
2. What immutable package, content, and code-revision identity authorizes use?
3. Which process and privilege boundary contains executed Python and Svelte?
4. What is the exact invalid, unsupported, or unavailable outcome at every
   validation and isolation failure?
5. Can hostile fixtures prove denial without executing outside the test
   sandbox?
6. What cancellation and shutdown contract owns validation and code execution?

The focused audit is complete when those questions have evidence-backed
answers for Diffusers, Transformers, generated Svelte, and restored persisted
components.
