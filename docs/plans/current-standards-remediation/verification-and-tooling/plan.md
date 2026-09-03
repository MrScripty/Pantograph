# Plan: Verification And Tooling Remediation

**Plan status:** `Planned`

**Current phase:** Milestone 0 — Claim authority and discovery

**Next slice:** Create the verification claim manifest and validator, then map
all current frontend tests and Rust workspace targets without changing their
schedules.

**Acceptance status:** `pending`

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

**Reports:** `none`

**Related ADRs:** [ADR index](../../../adr/README.md)

**Source audit:**
[04-verification-and-tooling.md](../../../audits/2026-09-03-current-standards/04-verification-and-tooling.md)

## Objective

Give Pantograph one truthful verification portfolio whose stable claims drive
test discovery, local commands, hooks, CI, launcher actions, browser evidence,
and release evidence without silent omission or evidence upgrades.

## Objective Acceptance

| ID | Observable criterion | Kind | Environment | Mode | Schedule | Owner | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| VT-A01 | The canonical portfolio validates every required claim's criterion, boundary, oracle, evidence kind, environment, mode, command/procedure, schedule, owner, state, and unavailable behavior. | `contract` | `not-applicable` | `automated` | pull request | Verification portfolio owner | `pending` | `npm run verify:portfolio` |
| VT-A02 | Every tracked frontend test below an owned test root is selected automatically, and every Rust workspace package/target has a claim or explicit governed disposition. | `focused` | `not-applicable` | `automated` | local and pull request | Frontend/Rust discovery owners | `pending` | Discovery fixtures and portfolio validation |
| VT-A03 | Traceability accepts only explicit staged or base/head input, evaluates the project impact map and prior/current map state, rejects missing facts, and cannot be satisfied by unrelated documentation. | `contract` | `not-applicable` | `automated` | pre-commit and pull request | Documentation/tooling owner | `pending` | Isolated Git fixtures |
| VT-A04 | Static gates state only properties they decide; accepted lint debt has stable identity and lifecycle, and an unregistered finding blocks. | `contract` | `representative` | `automated` | local and pull request | Language/tooling owners | `pending` | Structured diagnostic and debt fixtures |
| VT-A05 | Svelte compilation and graph pointer/keyboard/focus behavior pass compiler-aware and representative-browser evidence. | `user-workflow` | `representative` | `automated` | pull request | Frontend interaction owner | `pending` | Svelte check and browser result |
| VT-A06 | The exact packaged native binding loads with declared identity, and the exact desktop artifact reaches a bounded ready state on every required target. | `release-artifact` | `required-real` | `automated` | release candidate | Binding/desktop release owners | `pending` | Same-run package/load/start artifacts |
| VT-A07 | Local, hook, CI, launcher, and release entry points invoke portfolio claim IDs, preserve failures, report unavailable/cancelled/unselected work distinctly, and match current documentation. | `system` | `representative` | `automated` | pull request and release | Tooling/launcher owner | `pending` | Orchestration fixtures, CI evidence, final documentation review |

## Required Product Claim Seed

Milestone 0 records these stable IDs in `verification/claims.json`; that
manifest becomes current authority. A required claim may remain visibly
blocked, but weaker evidence cannot satisfy it.

| Claim ID | Observable result | Kind | Environment | Mode | Command/procedure | Schedule | Owner |
| --- | --- | --- | --- | --- | --- | --- | --- |
| VP-SCHED-001 | Scheduler-only workflow reaches its declared public terminal result without Tauri. | `system` | `representative` | `automated` | `npm run verify -- --claim VP-SCHED-001` | pull request | Scheduler/workflow service |
| VP-RUNTIME-TRUST-001 | Runtime/model trust rejects unauthorized identity or code before execution. | `contract` | `representative` | `automated` | `npm run verify -- --claim VP-RUNTIME-TRUST-001` | pull request | Runtime/model trust boundary |
| VP-RUNTIME-READY-001 | Dependency readiness proves declared versions and runtime requirements, not directory/import presence. | `system` | `representative` | `automated` | `npm run verify -- --claim VP-RUNTIME-READY-001` | pull request/release | Dependency environment service |
| VP-DURABLE-STATE-001 | Run, session, and diagnostic state survives its declared persistence/restart boundary. | `contract` | `representative` | `automated` | `npm run verify -- --claim VP-DURABLE-STATE-001` | pull request | Workflow service/diagnostics ledger |
| VP-HOST-BINDING-001 | Tauri and host bindings preserve canonical values and typed failures. | `contract` | `representative` | `automated` | `npm run verify -- --claim VP-HOST-BINDING-001` | pull request/host lane | Host-binding adapters |
| VP-GENERATED-AUTH-001 | Generated-component authorization, validation, timeout, and isolation fail closed at the real boundary. | `system` | `representative` | `automated` | `npm run verify -- --claim VP-GENERATED-AUTH-001` | dedicated runner | Generated-component service |
| VP-GRAPH-INTERACTION-001 | Pointer, keyboard, focus, escape, and parent-gesture behavior produce the declared graph result. | `user-workflow` | `representative` | `automated` | `npm run verify -- --claim VP-GRAPH-INTERACTION-001` | pull request | Svelte graph owner |
| VP-NATIVE-PACKAGE-001 | Same-run native package loads and reports declared version, ABI, and target identity. | `release-artifact` | `required-real` | `automated` | `npm run verify -- --claim VP-NATIVE-PACKAGE-001` | release/required target | Binding release owner |
| VP-DESKTOP-START-001 | Exact desktop artifact reports ready, remains healthy for the bounded observation, and exits cleanly in isolated state. | `release-artifact` | `required-real` | `automated` | `npm run verify -- --claim VP-DESKTOP-START-001` | release/required target | Desktop release owner |
| VP-IMAGE-REAL-001 | Editor submission reaches real image generation and retained-artifact inspection through the product path. | `user-workflow` | `required-real` | `either` | Claim command or manifest-owned operator procedure | release candidate | Image-workflow owner |

## Scope

### In Scope

- The claim manifest, its validation, and thin command/schedule adapters.
- Complete frontend discovery and explicit Rust package/target dispositions.
- Decision traceability, truthful static gates, and governed existing debt.
- Svelte/browser interaction evidence and exact-artifact release smoke.
- Verification/tooling documentation affected by these contracts.

### Out Of Scope

- Product fixes owned by the security, architecture, frontend, dependency,
  packaging, licensing, or documentation remediation plans.
- Selecting supported release targets or defining artifact/version/ABI identity;
  this plan consumes those decisions.
- Universal coverage thresholds, test schedules, tools, or commit topology.
- Substituting simulated hardware, models, services, or credentials for a
  required-real claim.

## Constraints And Assumptions

- Coding-Standards revision `82a0ddf315a08364357f6564018e37bdbeb72a1a`
  is the routed authority; a normative change requires re-routing.
- Shared files (`package*.json`, workflows, launcher, and policy docs) are
  integrated serially with adjacent remediation plans.
- Node 24 and Cargo metadata are expected to support the manifest/discovery
  adapters without a new parser dependency; Milestone 0 validates this.
- Frontend discovery currently owns `src/` and `packages/svelte-graph/src/`.
- The release plan must establish supported targets and exact artifact identity
  before release-smoke implementation starts.

## Binding Decisions

| Decision | Owner | Evidence | Supersedes |
| --- | --- | --- | --- |
| `verification/claims.json` owns claim identity and evidence/schedule facts; `verification/README.md` explains the boundary without copying live rows. | Verification/documentation | VER-02/03 and current Verification/Documentation workflows | Command labels and prose lists as claim authority |
| Frontend tests derive from owned roots; Cargo metadata is checked against package/target dispositions. Exclusions require identity, owner, reason, and revisit trigger. | Discovery owners | 19 frontend files and scheduler tests are silently omitted | Manual frontend argv and unexplained Rust subsets |
| `verification/tool-debt.json` admits only reviewed findings with stable matchers, owner, update authority, and retirement trigger. | Language/tooling owners | VER-01 | Advisory labels or total-count/changed-file ratchets without debt identity |
| Compiler-aware analysis and behavior evidence replace regex accessibility parsing and the blanket DOM-operation ban unless a rule proves distinct deciding value. | Frontend/tooling | VER-05/06 | Syntax proxy treated as ownership/accessibility proof |
| Traceability requires an explicit map and explicit mode/revisions; range mode evaluates prior and current maps and unresolved input fails. | Documentation/tooling | DOC-01 and current Documentation workflow | Source-root/README/heading defaults, inferred refs, any-doc satisfaction, silent skip |
| Release smoke exercises the exact same-run artifact. Retained source checks receive separate supporting claims or are deleted. | Release evidence | VER-04 | Source tests labeled release-artifact evidence |

## Claim-To-Evidence Map

| Claim | Deciding evidence and independent authority | Intended negative evidence | Unsupported/unavailable boundary |
| --- | --- | --- | --- |
| VT-A01/A02 | Validator compares manifest with tracked frontend paths, Cargo metadata, and configured consumers. | Added test/unmapped target fails with identity. | Dynamic untracked tests are outside discovery; missing inventory is `unavailable`. |
| VT-A03 | Git fixtures contain their own impact map and staged/range revisions. | Deleted map row, missing revision, renamed artifact, and unrelated doc fail. | Unrepresentable repository state is `unsupported`; missing inputs are `unavailable`. |
| VT-A04 | Native structured diagnostics are compared with reviewed debt identities. | Injected unknown or changed diagnostic fails. | An unstable/unparseable tool cannot support governed debt. |
| VT-A05 | Real rendering engine observes role/state/focus and pointer/keyboard result. | Missing cleanup/focus/gesture result fails. | Missing browser capability is `unavailable`, not component-test success. |
| VT-A06 | Same-run release manifest/checksum selects bytes; host load/identity and bounded ready/exit probes observe them. | Wrong/stale artifact or early exit fails. | Undeclared targets do not imply support; unavailable required targets block. |
| VT-A07 | Contract fixtures compare each consumer's claim IDs, inputs, status mapping, and docs link with the manifest. | Unknown ID or swallowed/unreported outcome fails. | Provider inability to represent a required status is `unsupported`. |

## Systemic Finding Audit

- **Invariant/owner:** every advertised claim has one evidence contract owned by
  `verification/claims.json`.
- **Bounded consumers:** `package.json`, `launcher.sh`, `lefthook.yml`, CI
  workflows, `scripts/`, frontend test roots, Cargo targets, release smoke, and
  verification policy docs.
- **Expansion facts:** a new product boundary, test root, workspace package,
  release target, or reachable consumer expands the population.
- **Dispositions:** each consumer invokes a claim, is supporting-only, or is
  removed; required-real gaps remain blocked.
- **Alternatives:** delete duplicate surfaces, consolidate dispatch, prefer
  native discovery, and retain validators only for distinct deciding value.
- **Stopping condition:** every bounded consumer/test/target has an evidence-
  backed disposition and VT-A01–VT-A07 pass.
- **Composition comparison:** one claim registry plus thin adapters replaces
  manually synchronized lists and evidence claims.

## Simplicity And Ownership Review

**Applicability:** `applicable`

- Independent concepts and dimensions: claim meaning, discovery, schedule, debt,
  documentation impact, and artifact identity remain separate.
- State, identity, value, time, policy, and mechanism: claim/debt state lives under
  `verification/`; Cargo, Git, release manifests, and tool output remain
  referenced authorities. Claim schema, product version, ABI, target, and
  source revision are distinct. Renaming a claim migrates all consumers;
  moving a test inside an owned root does not.
- Caller and composition-root knowledge: workflows and launcher know claim IDs and environment
  setup, not copied test lists or stronger evidence meanings.
- Representative change paths and forced owners: new tests are discovered; new Cargo targets need
  a disposition; target changes update release claims; durable knowledge
  changes update only mapped documentation.
- Stable Interfaces versus hidden knowledge: manifest schema and claim runner are
  stable; globs, selectors, matchers, and provider syntax remain internal.
- Independent evolution, testing, failure, and replacement: discovery, traceability, debt, browser, and release
  smoke have separate fixtures and typed failures.
- Necessary complexity and containment: the registry addresses demonstrated omissions and
  contradictions; it does not become another test framework.
- Deletion and cumulative machinery result: remove manual argv, duplicate suite prose, invalid regex
  checks, inferred traceability fallback, and false artifact-smoke labeling.

## Cross-Plan Dependencies

| Dependency | Required result | This plan consumes it |
| --- | --- | --- |
| Security/dynamic code (SEC-01–03) | Trust, authorization, timeout, and isolation contracts | VP-RUNTIME-TRUST-001, VP-GENERATED-AUTH-001 |
| Architecture/lifecycle/bindings (ARC-01–05) | Canonical scheduler execution and lossless host outcomes | VP-SCHED-001, VP-DURABLE-STATE-001, VP-HOST-BINDING-001 |
| Frontend/accessibility (FE-01–06) | Interaction, listener lifecycle, decoding, and component ownership | VT-A05, VP-GRAPH-INTERACTION-001 |
| Dependencies/release/docs (DEP-01/02, REL-01/02, DOC-01–03) | Dependency readiness, artifact identity, target matrix, documentation authority | VP-RUNTIME-READY-001, VT-A03, VT-A06 |

Claim definition/discovery lands first. Product-claim satisfaction follows its
owning implementation. Shared configuration and lockfiles integrate serially.

## Milestones

### Milestone 0: Claim Authority And Discovery

**Goal:** Establish the portfolio and make omissions observable without changing
schedules.

**Allowed write set:**

- `verification/README.md`
- `verification/claims.json`
- `scripts/run-verification-claim.mjs`
- `scripts/check-verification-portfolio.mjs`
- `scripts/tests/verification-portfolio.test.mjs`
- `package.json`
- `docs/development.md`
- `scripts/README.md`

**Tasks:**

- [ ] Add the required claim seed, schema/reference checks, and stable
  `npm run verify -- --claim <ID>` / `verify:portfolio` interfaces.
- [ ] Map every tracked frontend test and Cargo package/target to a claim or
  explicit disposition; fixture additions must expose omissions.

**Acceptance gate:** VT-A01 and VT-A02 pass; unexplained current omissions fail.

**Status:** `Planned`

### Milestone 1: Documentation Traceability

**Goal:** Replace retired documentation policy with explicit impact mapping.

**Allowed write set:**

- `scripts/check-decision-traceability.sh`
- `scripts/decision-traceability-map.tsv`
- `scripts/tests/decision-traceability/**`
- `package.json`
- `lefthook.yml`
- `.github/workflows/quality-gates.yml`
- `scripts/README.md`

**Tasks:**

- [ ] Remove source-root, per-directory README, fixed-heading, any-doc,
  inferred-revision, and silent-skip behavior.
- [ ] Require explicit map/mode/base/head as applicable, evaluate both map
  states, emit typed failure, and cover positive/negative Git fixtures.

**Acceptance gate:** VT-A03 passes locally and in explicit hook/CI modes.

**Status:** `Planned`

### Milestone 2: Static Gates And Governed Debt

**Goal:** Make formatter/compiler/lint checks truthful and prevent new debt.

**Allowed write set:**

- `verification/claims.json`
- `verification/tool-debt.json`
- `scripts/run-verification-claim.mjs`
- `scripts/check-verification-portfolio.mjs`
- `scripts/tests/verification-portfolio.test.mjs`
- `scripts/check-critical-antipatterns.mjs`
- `scripts/check-svelte-a11y.mjs`
- `eslint.config.mjs`
- `package.json`
- `package-lock.json`
- `docs/adr/ADR-017-rust-workspace-policy.md`
- `docs/development.md`
- `scripts/README.md`

**Tasks:**

- [ ] Re-run structured diagnostics; consume domain fixes or register each
  remaining finding with owner/rationale/update/retirement facts.
- [ ] Block unmatched findings, promote clean formatting, add compiler-aware
  Svelte checking, and delete/demote invalid custom checks after replacement.
- [ ] Correct false clean-baseline documentation.

**Acceptance gate:** VT-A04 passes, an unknown finding fails, and no document
overclaims Clippy or frontend lint status.

**Status:** `Planned`

### Milestone 3: Command And CI Reconciliation

**Goal:** Make each local, hook, launcher, and CI surface a thin scheduled claim
adapter.

**Allowed write set:**

- `verification/claims.json`
- `scripts/run-verification-claim.mjs`
- `scripts/check-verification-portfolio.mjs`
- `scripts/tests/verification-portfolio.test.mjs`
- `package.json`
- `lefthook.yml`
- `launcher.sh`
- `.github/workflows/quality-gates.yml`
- `.github/workflows/headless-embedding-contract.yml`
- `docs/development.md`
- `scripts/README.md`

**Tasks:**

- [ ] Reconcile `npm test`, `npm run check`, launcher, hooks, and CI around
  claim IDs; schedule by environment/risk/cost rather than labels.
- [ ] Preserve delegated exits and distinguish failure, unavailable,
  cancellation, and intentional non-selection in CI reporting.

**Acceptance gate:** VT-A07 orchestration fixtures and representative CI pass.

**Status:** `Planned`

### Milestone 4: Representative Frontend Evidence

**Goal:** Prove Svelte compilation and real graph interaction behavior.

**Allowed write set:**

- `verification/claims.json`
- `package.json`
- `package-lock.json`
- `tests/e2e/workflow-graph-interaction/**`
- `scripts/check-workflow-graph-interaction-gui-smoke.sh`
- `.github/workflows/quality-gates.yml`
- `docs/development.md`
- `scripts/README.md`

**Tasks:**

- [ ] Consume the frontend plan's accepted interaction contract and add only
  applicable role/state/focus/pointer/keyboard/gesture observations.
- [ ] Declare rendering engine, display/session, isolation, timeout, lifecycle,
  and unavailable behavior.

**Acceptance gate:** VT-A05 and VP-GRAPH-INTERACTION-001 pass in the declared
representative environment.

**Status:** `Planned`

### Milestone 5: Exact-Artifact Release Evidence And Closeout

**Goal:** Replace false release smoke, then consolidate current authority.

**Allowed write set:**

- `verification/README.md`
- `verification/claims.json`
- `verification/tool-debt.json`
- `scripts/check-runtime-redistributables-smoke.sh`
- `scripts/check-packaged-csharp-quickstart.sh`
- `scripts/check-uniffi-csharp-smoke.sh`
- `launcher.sh`
- `.github/workflows/release-verification.yml`
- `docs/development.md`
- `docs/adr/ADR-017-rust-workspace-policy.md`
- `docs/release.md`
- `docs/README.md`
- `scripts/README.md`
- `docs/plans/current-standards-remediation/verification-and-tooling/plan.md`
- `docs/plans/current-standards-remediation/verification-and-tooling/execution-ledger.md`
- `docs/plans/current-standards-remediation/verification-and-tooling/issues.md`

**Tasks:**

- [ ] Consume supported target/artifact identity, then load/start exact
  same-run artifacts with bounded environment, health, and termination checks.
- [ ] Relabel justified source checks as supporting evidence or delete them;
  remove stale suite prose and record final claim evidence.

**Acceptance gate:** VT-A06 passes for every required target, then all VT-A01–
VT-A07 evidence and remaining dispositions are recorded.

**Status:** `Planned`

## Governed Debt, Migration, And Rollback

- Each debt item records tool/version, rule, owner path/symbol, stable matcher,
  rationale, owner, update authority, blocking behavior, and retirement trigger.
  New/changed/unparseable findings block; required claims cannot become debt.
- Add the portfolio beside existing commands, then switch one consumer class at
  a time. Establish debt identity before changing gate status.
- Replace traceability atomically with its map/fixtures. The retired checker is
  not a fallback; failure leaves traceability explicitly unavailable.
- Observation-mode CI may validate orchestration but cannot claim acceptance.
  Remove superseded jobs when replacements pass.
- Release failure retains candidate artifacts/diagnostics under release policy
  and blocks release; source checks are not rollback evidence.

## Risks

- A registry can become duplicate ceremony; consumer/discovery validation and
  the retention rule bound it to demonstrated omission risk.
- Debt matchers can hide changed diagnostics; stable identities, explicit
  update authority, and negative fixtures make changes blocking.
- Browser/release gates may be costly or unavailable; schedule records must keep
  that state explicit rather than weakening claims.
- Adjacent plans touch shared configuration; serialize integration and rerun
  affected portfolio checks after each upstream change.

## Blockers

- `none` for the next slice. Milestones 4 and 5 wait on the named frontend and
  release authorities.

## Re-Plan Triggers

- Node/Cargo cannot provide complete discovery without materially more
  machinery than admitted.
- A new semantic owner, test root, Cargo target, supported platform, artifact
  identity, or reachable consumer expands the bounded population.
- Debt cannot be distinguished reliably, or representative/required-real
  environments cannot decide their selected claim.
- An upstream remediation changes claim semantics or shared surfaces after
  integration, or cumulative machinery exceeds its deletion value.

## Final Acceptance

- Acceptance status: `pending`
- Deferred follow-ups: `none`
- Final status: `Planned`
