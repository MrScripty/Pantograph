# Plan: CPU text dependency readiness handoff

**Plan status:** `Blocked`
**Current phase:** M1 runtime-effects observation; public terminal-result and trained-model qualification blocked
**Next slice:** coordinate a public failure/cancellation result-projection fix with the workflow finalization owner
**Acceptance status:** `blocked`
**Execution ledger:** [execution-ledger.md](execution-ledger.md)
**Issues:** [issues.md](issues.md)
**Reports:** [reports](reports/)
**Related ADRs:** [ADR index](../../adr/README.md)

## Objective and admission

Repair PR72's missing dependency requirements payload for a host Python CPU
text-generation workflow, then observe actual generated text through public
Pumas resolution, graph dependency Resolve/Check, scheduler and runtime execution.
User delegation authorizes this plan's `start`, implementation, review, commit,
push and PR. Baseline: `07b23d47`. Independent review precedes implementation.
No AGENTS.md exists in the checkout or accessible workspace. Standards read at
MrScripty/Coding-Standards `188beda1fa477d21d576c233dd8c7f4c4c267d23`: Core,
Router, Implementation, Verification, Planning, Development Proportionality,
Commit, Documentation, Rust/API, Library, Contracts, Architecture, Code Design,
Dependencies and Licensing. Apply existing ownership and bounded evidence.

## Acceptance

| ID | Observable criterion | Kind | Environment | Mode | Status |
| --- | --- | --- | --- | --- | --- |
| T1 | Supported text Resolve supplies nonempty, identity-bound Python prerequisites without asserting Ready; unsupported inputs and declared model constraints are refused | contract | not-applicable | automated | satisfied |
| T2 | Real Pumas graph action registers payload without a Ready snapshot; actual inventory Check observes the linked interpreter and preserves its environment identity | integration | representative | automated | satisfied |
| T3 | A trained, licensed, bounded local model generates nonempty text through the public hosted CPU workflow; model identity, resource limits, output and cleanup recorded | system | required-real | automated | blocked |
| T4 | Committed untrained weights exercise public hosted generation, loader failure, active cancellation and owned cleanup while preserving public failure/cancellation meaning | integration | representative | automated | blocked |

Formatting, affected Rust tests and Clippy are supporting gates. Synthetic model
output is only contract evidence and cannot close T3. No GPU, browser GUI,
native desktop, release, remote worker or model-quality benchmark claim.

## Binding decisions and scope

Pumas owns model declarations. The runtime adapter owns prerequisites of its
compiled Python text worker. Inventory owns installed-package observations;
workflow service owns requirements registry insertion/freshness and scheduler
owns admission. Do not modify those owners or bypass Check. Presence-only
inventory cannot enforce arbitrary Pumas exact pins; refuse packages with any
additional declarations rather than dropping constraints. Host prerequisites
are presence requirements only and do not promise version or ABI compatibility.
Actual execution must independently prove this selected environment works.

Support explicit text-generation / pytorch / cpu with current host platform,
no custom code, no overrides/traits, and no foreign bindings or environments.
Preserve proof identity checks and existing Candle behavior. Unsupported requests
return the existing no-resolution outcome, never a fabricated Ready result.
No product dependency/lockfile changes, implicit package installs or user catalog edits. The 2026-10-19 delegation explicitly authorizes normal environment setup from official registries.
Tests create isolated Pumas catalogs through its public APIs.

## Constraints and discovery

The previous adapter hard-codes embedding/candle/cpu. Pumas public dependency
resolution distinguishes zero declarations from unresolved declared bindings.
The existing Python inventory checks distribution presence only. The new text
profile observes fixed distribution names through the same linked PyO3 interpreter
that owns execution. Runtime executable overrides cannot select its observation
source. A blocking metadata call is isolated and joined by the existing inventory
producer; no prerequisite-package imports, installer or generation runs during
this observation. Current environment: approximately 17 GiB RAM, 30 GiB free
disk; CPU-only, two Cargo jobs, command deadlines 600 seconds. Model downloads
are opt-in qualification inputs outside Git, capped at 600 MiB total; verify
upstream revision and license before weights. No executable remote model code.
Model download or native build unavailability leaves T3 unsatisfied.

## Simplicity and ownership review

**Applicability:** `applicable` (extend an existing adapter).

1. Independent concerns: Pumas declaration lookup, worker prerequisites,
   inventory observation, graph freshness/admission and inference execution
   remain with their existing owners.
2. Necessary coupling: requirements identity and platform must bind the same
   model/task/runtime request. No new state, cache or environment authority.
3. Callers retain the existing async Resolve interface. Composition roots and
   scheduler learn nothing new; adapter hides text profile selection.
4. A worker prerequisite change touches its adapter/test; Pumas contract change
   touches its facts adapter; inventory or scheduler policy stays independent.
5. Typed request/result and Pumas API remain boundaries; no private database,
   transport schema or scheduler lifecycle knowledge is added.
6. Text and Candle profiles can fail and be tested independently. Shared request
   proof and no-declarations semantics retain one owner where reused.
7. Removing the text profile restores the observed missing-payload failure;
   the narrow inference-owned package metadata API would be removed with it.
   No new registry, validator framework or permanent service exists.
8. Necessary complexity is bounded to selecting an honest prerequisite payload;
   retain existing inventory and lifecycle machinery without duplicating it.

## M1: repair and qualify

**State:** Blocked
**Write set:**

- `crates/pantograph-embedded-runtime/src/inference_dependency_requirements.rs`
- `crates/pantograph-embedded-runtime/src/inference_interface_facts_provider.rs`
- `crates/pantograph-embedded-runtime/src/pumas_dispatch_package_facts.rs`
- `crates/pantograph-embedded-runtime/src/inference_text_dependency_requirements.rs` and focused sibling tests
- `crates/pantograph-embedded-runtime/src/lib.rs` (private module registration)
- `crates/pantograph-embedded-runtime/src/dependency_inventory_python.rs`
- `crates/pantograph-embedded-runtime/src/dependency_inventory.rs` (preserve embedded environment identity)
- `crates/inference/src/backend/pytorch_text_dependencies.rs`
- `crates/inference/src/backend/pytorch.rs` (owned dependency API export)
- `docs/plans/text-generation-readiness/**`
- `examples/cpu-embedding-demo/README.md` (link qualification scope)

The 2026-10-19 continuation owns the standalone text probe, its Cargo example
registration, `scripts/qualify-cpu-text-workflow.sh`, the separate
`.github/workflows/cpu-text-integration.yml` (avoids shared M3 CI edits),
`docs/cpu-text-generation-workflow-probe.md`, and this plan directory.
The historical M2 worker wrote no probe files; this delegation supersedes its
probe reservation. No scheduler files, fairness policy, shared contracts, root
README (including PR79), or embedding probe files are edited. The new scheduler
M3 owner has a disjoint write set. Expand ownership only through the parent.

Isolation: owner is the CPU text integration delegate; branch
`qualify/cpu-text-generation-2026-10-19` in `/workspace/pantograph-text`, based on
main `4aec0d5c`. PR75 adapter commit `2f1e38fd` is reused as `7a78625b` by selective
cherry-pick, without rewriting PR75. Target is main; parent coordinates integration.
Retain this protected branch/worktree for blocked T3 follow-up and review, then
parent decides retirement. No other branches or worktrees are deleted.

Independent GPT-6.1 Sol / High reviewer `text_review` admitted a cfg-gated local
probe using real Pumas, graph Resolve/Check and scheduler output. It adds no new
runtime authority. Fixed CPU, local-only artifacts, no remote model code, 24 new
tokens, deterministic temperature 0, one loaded model, bounded run and external
600s watchdog preserve scope. New probe code owns only fixture/evidence setup
and public API orchestration. Source file identities are recorded to bind outputs
to exact local inputs; no generic integrity service is introduced. Synthetic
untrained fixtures can prove integration only and cannot satisfy T3. Resolve
must not fabricate Ready; actual Check observations and shutdown are retained.
Cancellation observation requires public Running, acknowledged intent, matching
run/node/attempt runtime cancellation diagnostics, an empty queue and cleanup.
The public API currently flattens cancellation and loader errors into generic
`InvalidRequest` / `TerminalFailed`; observation success does not qualify that
projection or T4. The shared finalization-owner correction is excluded pending
parent coordination. A dropped future cannot supply cancellation evidence.

Gates: independent plan admission, focused adapter regressions, actual hosted
probe with bounded trained model, affected crate tests/Clippy, staged diff and
independent final review. Record commands, exit status and evidence limitations.

## Blockers and re-plan triggers

The no-default baseline test configuration failed because existing tests use
InferenceGateway::new behind backend-llamacpp. Use supported defaults plus
backend-pytorch and host-dependency-inventory; do not edit unrelated tests.
GitHub CLI GraphQL is Forbidden; use the available
authorized connector for PR creation if push succeeds. Replan if model declarations
must be projected, Python runtime/probe identity differs materially, shared
scheduler contracts must change, or qualification requires larger/unlicensed
weights. Stop investigation when the narrow supported profile is established;
implement the reversible repair and retain unavailable evidence honestly.

Revised direction: the text profile's package presence observation belongs to
inference's linked PyO3 interpreter. Existing process probes remain scoped to
their current contracts; the new exact profile cannot fall through to them.
This bounded addition is within dependency/readiness ownership and changes no
scheduler or serialized public schema. T3 remains blocked on sanctioned model
access independently of the implementable T1/T2 repair. Current task configuration
is explicitly GPT-6.1 Sol / High; every new reviewer spawn sets both.

Independent `review_sol` (explicit GPT-6.1 Sol / High) admitted the revised
direction before implementation. The fixed profile, exact route and
unavailable-on-observation-error invariants are implemented and verified.

T1/T2 source and checks are complete. See the [verification report](reports/2026-10-13-dependency-handoff.md)
and [source/result record](reports/source-and-results.json). T3 remains blocked;
this plan is not Accepted. No trained text, GPU or desktop outcome is claimed.
