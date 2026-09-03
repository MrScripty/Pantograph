# Focused Audit: Verification And Tooling

Implementation plan: [Verification and tooling remediation](../../plans/current-standards-remediation/verification-and-tooling/plan.md)

## Scope

This audit covers acceptance claims, test discovery, local commands, hooks,
lint and format policy, CI scheduling, debt governance, release smoke evidence,
and truthfulness of verification documentation.

Applicable current standards are Verification, Tooling, Development
Proportionality, Rust Tooling, TypeScript, Frontend, Launcher, and
Documentation.

## Assessment

Pantograph has many useful tests and CI lanes, but the portfolio is assembled
from command labels and manually curated lists rather than an explicit map of
supported claims to adequate evidence. That permits silent omission and stale
claims.

## Observed Baseline

| Check | Result and scope |
| --- | --- |
| cargo fmt --all -- --check | Pass |
| cargo check --workspace --no-default-features | Pass |
| npm run typecheck | Pass for configured TypeScript |
| npm run test:frontend | Pass; manually lists 71 of 90 test files |
| all discovered frontend tests | Pass; not a canonical command |
| npm test | Pass; only node-engine and workflow-nodes |
| cargo test -p pantograph-scheduler | Pass; 99 tests, not scheduled in CI |
| npm run lint:full | Fail |
| npm run lint:critical | Fail |
| npm run lint:a11y | Fail |
| npm run traceability | Fail |
| cargo clippy --workspace --all-targets --all-features -- -D warnings | Fail |

## Findings

### VER-01 — High: documented clean baselines are false

The former Rust-workspace and testing/release guides state
that strict Clippy passes. It currently fails in managed-dependencies,
runtime-registry, scheduler, and diagnostics-ledger. Frontend lint and
accessibility gates are also red.

CI keeps Clippy and formatting advisory. Formatting now passes and can be
reclassified after its claim and schedule are selected. Clippy needs a truthful
debt boundary or remediation; documentation cannot call it clean meanwhile.

### VER-02 — High: frontend test discovery silently omits tests

package.json manually lists 71 test files while 90 tracked files exist.
Nineteen passing tests are absent from both the canonical local command and CI,
including graph interaction, workbench presenter, workflow validation
lifecycle, and wire tests.

The problem is discovery authority, not currently failing assertions. Test
registration should derive from an owned scope or fail when tracked tests are
unselected.

### VER-03 — High: Rust test selection has no claim map

Quality CI executes two crate libraries and one workflow-service contract
suite. The headless workflow adds selected workflow-service, HTTP adapter,
UniFFI, and Rustler paths. Contract-heavy scheduler, embedded-runtime,
runtime-registry, dependency, persistence, inference, and path-security suites
are not comprehensively scheduled.

npm test, npm run check, launcher.sh --test, pre-push, and CI all select
different subsets. No document maps those subsets to supported behaviors,
evidence kinds, required environments, and execution modes.

### VER-04 — High: release smoke does not exercise the artifact

scripts/check-runtime-redistributables-smoke.sh resolves a release executable
and checks that it exists, then runs Cargo and Node source-tree checks. It does
not invoke or inspect the selected binary. No CI workflow runs the documented
release-smoke path.

This is source evidence labeled as release-artifact evidence. A stale or broken
binary can pass.

### VER-05 — Medium: frontend static gates overclaim their syntax checks

The critical checker blanket-bans appendChild/removeChild even though current
standards allow imperative DOM with explicit ownership and cleanup. The
accessibility checker parses Svelte with regex and produced at least one false
positive because an arrow expression truncates the parsed tag.

These checks may remain useful lint mechanisms, but only for claims their
oracles can decide. They cannot replace component or browser evidence.

### VER-06 — Medium: Svelte and browser behavior is largely untested

tsc includes TypeScript but not Svelte compilation/type checking, and no
svelte-check gate exists. Frontend unit tests do not mount Svelte components.
The only real desktop E2E path is narrow and unscheduled.

This is not a demand for universal browser tests. Each user-visible interaction
claim should select the smallest representative environment that can actually
decide it.

## Required Verification Portfolio Audit

Create stable claim IDs for at least:

- scheduler-only workflow execution;
- runtime/model trust and dependency readiness;
- durable run/session/diagnostic state;
- Tauri and host-binding contract projection;
- generated-component authorization and isolation;
- workflow graph pointer/keyboard/focus behavior;
- packaged native binding load and version/target identity;
- desktop release startup; and
- required-real image generation from editor submission through retained
  artifact inspection.

For each claim record evidence kind, environment, execution mode, command or
manual procedure, schedule, owner, and unavailable behavior. Only then should
hooks and CI be simplified or expanded.

## Immediate Tooling Direction

1. Replace manual frontend enumeration with owned discovery.
2. Reconcile local, hook, CI, and release command surfaces around the claim
   map.
3. Correct false baseline documentation before claiming compliance.
4. Establish explicit governed debt for checks that remain advisory.
5. Redesign release smoke to load or launch the exact packaged artifact and
   assert a bounded observable outcome.
