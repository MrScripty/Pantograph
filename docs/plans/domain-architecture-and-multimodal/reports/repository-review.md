# Repository Coverage And Prior-Claim Reconciliation

Date: 2026-09-08. Source baseline: `2ba2efb1cd1a06b657f7227bf74caae99f275dfc`
plus the observed worktree. This is bounded M0/M1 read-only investigation,
not a complete audit or implementation admission. The integrator owns current
dispositions, old-plan supersession and acceptance in the active plan.

## Standards And Evidence Scope

Coding-Standards HEAD is `366c1d90a24bbfb50973f62b155a5f3396c0f107`, matching
the plan. Its only observed dirty item was untracked
`tools/standards_engine/prototypes/a2/proportionality-routing.prototype.html`;
it is not a normative input to this report. Core and Router were read, followed
by Planning, Implementation, Verification, Development Proportionality,
Documentation, Tooling, Build, Release, Contracts and Licensing (including
their unconditional Requires). Release is examined for existing artifact
promises, not to authorize publication. No legacy standards navigation file
was treated as reinstating retired rules.

The inventory and claim mapping require no language source changes. Subsequent
binding work must route Library, Interop, Language Binding, Generated Contract,
IPC and the applicable Rust profiles; test/CI work routes Tooling/Verification
and affected Rust/TypeScript tooling; launcher work routes Launcher,
Dependencies and Cross-Platform; actual target evidence routes Verification's
platform detail. These routes are investigation assignments, not claims that
their complete obligations were reviewed here. No Godot or concurrent proposal
integration applicability was established by these read-only reports.

Evidence was local manifest/configuration/source and ledger inspection,
`git ls-files`, tracked-test comparison, and worktree/revision inspection.
No builds, model runs, suite execution, package generation or external network
queries were performed. Old audit test results remain dated evidence.

## Bounded Maintained Population

`git ls-files` identifies 1,482 tracked files: 768 Rust, 298 TypeScript,
126 Svelte and 15 Python source files. Counts describe inventory, not review
completion. Include maintained untracked plan reports and changes when
integrating; do not infer that every untracked file is disposable.

| Population | Owner / reachable boundaries | Coverage disposition |
| --- | --- | --- |
| All 23 workspace members in root `Cargo.toml` (22 under `crates/`, plus `src-tauri`) including source, tests, examples, build scripts, manifests and feature combinations | Rust application/library owners; workflow, scheduler, inference, runtime, persistence, diagnostics, native/IPC boundaries | Entire population in DA-01; execution/runtime reviewers cover critical path first; unexamined leaf behavior remains M1/M4 work |
| `src/` and `packages/svelte-graph/`, including package exports, generated-component loader and tests | Desktop composition and reusable graph package | Desktop reviewer; root/package manifests and discovery examined here; interaction/async/accessibility acceptance remains pending |
| Python under `crates/inference/{audio,depth,onnx,torch}/`, embedded-runtime `python_runtime_bridge.py`, and two Python CLI smokes in `scripts/` | Worker protocols and process adapters; scripts are consumers | Runtime reviewer; all 15 tracked Python files included even though there is no Python host binding package |
| `bindings/csharp/`, `bindings/beam/` and Rust UniFFI/Rustler crates | Host projections, generated metadata, application lifecycle and artifact loading | Support boundaries inventoried here; full conversion/event/shutdown review outstanding |
| `scripts/` (26 tracked files), `launcher.sh`, `lefthook.yml`, `.github/workflows/` (3 workflows), `eslint-rules/`, ESLint/Vite/TS/Tailwind/PostCSS configuration, HTML entry point | Build, validation, operation, hooks, CI and development tooling | First findings below; every maintained runner still needs claim-specific disposition, including Python CLI and real-model smoke paths |
| Root Cargo/npm manifests and locks, toolchain/version files, three Python requirements files, `src-tauri/tauri.conf.json`, capabilities, icons and other assets | Dependency resolution, native build, desktop capability and distributed content owners | In scope; source identity and licensing/support gaps remain open |
| `tests/e2e/workflow-editor-image-generation/` plus inline/crate/package tests and host smoke projects | Existing desktop WebKit/WebDriver path and lower-level oracles | Keep proof kinds distinct; frontend test discovery gap confirmed; Rust tests need per-target execution coverage disposition |
| Tracked `.pantograph/` workflows/orchestrations and READMEs | Saved graph consumers and runtime configuration | Persisted fixtures remain in scope even when current ignore patterns would ignore newly created files |
| `shared-resources/models/models.db`, `assets/`, metadata and notice files | Persisted model metadata/content provenance and packaging consumers | Explicit review population, not automatically excluded as generated; owner/retention/provenance still to establish before modification |
| `README.md`, `ARCHITECTURE.md`, `CHANGELOG.md`, `docs/`, package/binding/script READMEs | Current operation, architecture, support and acceptance knowledge | Current guides/ADRs in scope; old reports/terminal plans remain dated evidence and need no cosmetic rewrite |

The workspace names are authoritative in `Cargo.toml`: inference, node-engine,
workflow-nodes, pantograph-{diagnostics-ledger,dependency-environment-service,
dependency-planning,inference-interface-contracts,managed-dependencies,
media-conversion,node-contracts,path-security,runtime-attribution,
runtime-identity,runtime-host-contracts,runtime-registry,scheduler,
timing-contracts,embedded-runtime,frontend-http-adapter,workflow-service,
rustler,uniffi}, and src-tauri. Rustler is omitted from `default-members` but
remains a workspace/consumer review obligation. Workspace checks and doc tests
do not establish execution of every integration test target.

| Separately inventoried material | Exclusion / retained obligation |
| --- | --- |
| `target/`, `dist/`, `node_modules/`, `.venv/`, Python caches | Generated/build/downloaded output; review producers, pinned inputs and actual required artifacts, not vendored dependency internals as first-party source |
| Runtime-authored `src/generated/*` and `.pantograph/generated-components.git/` | Untrusted/generated user content; exclude as maintained source, retain validator, discovery, non-execution and history contract obligations |
| Ignored `.pantograph` artifacts/SQLite/runtime records and user-created graphs; `data/`, `launcher-data/`, resource caches | Local state, downloaded data and runtime environments; no blanket source edit/delete authority; review lifecycle, schema and consumers where reachable |
| External Pumas repository, Git/registry dependencies, system packages, Action implementations and model artifacts | External owners; Pantograph owns selected identity, boundary, license/provenance and consumption evidence. External source writes require separate authority |
| Protected Pumas proposals, ignored user `agent_plan.md`, `.claude`, `.agents`, `.codex`, `.git` internals | User/local-tooling authority and unrelated work; exclude from product remediation edits, preserve intact. Proposals may inform dependency gaps without becoming accepted contracts |

No tracked first-party root is excluded merely for age or file size. Assets,
tracked database contents and templates still need explicit provenance and
consumer disposition; this inventory does not label them accepted.

## Consumers, Targets And Authority Gaps

[`ADR-010`](../../../adr/ADR-010-binding-projection-ownership-and-support-tiers.md)
is the existing binding support owner: native Rust supports implemented
execution-platform surfaces; C# supports only generated/native smoke and
packaged-quickstart-covered surfaces; BEAM is experimental; Python host bindings
are unsupported until a real package/generation/load path exists. Python model
workers are separate from Python host bindings. Local C#/BEAM fixtures are real
consumers but do not prove absence of external users. Root npm is private,
Rustler/UniFFI declare `publish = false`; the graph package has public exports
and no `private` declaration. None of these fields proves that independently
deployed consumers do or do not exist. Preserve external-consumer discovery as
an explicit precondition before incompatible exported-surface removal.

All three CI workflows select `ubuntu-latest`. Quality gates use Rust 1.92.0,
Node from `.node-version`, and a BEAM lane with OTP 27.0 / Elixir 1.16.3.
Headless workflow CI adds .NET 8.0.x and C# generator 0.9.0 and labels its
native artifact `linux-x64`. Packaging scripts branch on Linux/Darwin/Windows
shell families and allow `PANTOGRAPH_PACKAGE_PLATFORM`; those implementation
branches do not establish a tested support matrix or actual architecture/ABI.
The real desktop runner is the existing WebKit/Tauri image fixture; its runtime
capability is owned by the other review lanes.

[`docs/release.md`](../../../release.md) explicitly says there is no accepted
release workflow or verified multi-platform matrix. Keep target triples,
architecture, ABI/cohort, release units, supported older versions, acquisition
channels, candidate source and exact-artifact evidence unresolved until their
owners settle them. Root npm/Tauri version `0.0.0` and workspace/graph package
version `0.1.0` are different observed identities; their difference alone is not
a defect without the release/compatibility contract.

`LICENSE` contains Apache-2.0 while Cargo declares `MIT OR Apache-2.0`.
README now reports this conflict accurately. Repository-owner selection of
project terms remains required for DRD-A03/DA-07; this report makes no license
choice or legal compatibility conclusion. Publication also lacks authorized
destination/channel, credentials owner, withdrawal procedure and final
candidate; publication is outside this plan. These gaps block their dependent
claims, not model-path work or reversible tooling repair.

## Complete Old Finding And Acceptance Disposition

Mappings below transfer obligations for fresh review; they do not accept old
findings as proven current defects or close claims. Unless explicitly noted,
all remain pending, with original kind/environment/mode retained by reference
to the old acceptance row. DA aggregate IDs cannot downgrade those dimensions.

| Audit ID | Carried obligation | New claim / milestone owner |
| --- | --- | --- |
| CSA-01 | Exact model-code authorization and diffusion worker/cache identity | DA-04, DA-02; runtime M2 before real-model execution |
| CSA-02 | Fail-closed generated-source admission, validator lifecycle, renderer non-execution | DA-04, DA-07; runtime/desktop M2–M4; live execution remains deferred |
| CSA-03 | Sole scheduler/runtime-host execution authority | DA-02, DA-03; execution M2 |
| CSA-04 | Thin checked bindings, owned processes/events and observable shutdown | DA-02, DA-04, DA-05; runtime/bindings M2/M4 |
| CSA-05 | Complete Tauri producer/consumer decoding | DA-05, DA-04; desktop M3/M4 |
| CSA-06 | Truthful static/command/CI claims and governed remaining debt | DA-01, DA-07; coverage M1/M4/M5; current docs already corrected as dated history |
| CSA-07 | Complete frontend discovery and Rust target evidence population | DA-01, DA-07; tooling M4 (bounded pilot possible) |
| CSA-08 | Exact packaged artifact execution and target/version identity | DA-07; release/binding M4/M5, blocked where target/candidate authority unavailable |
| CSA-09 | Exact-impact traceability and one current plan authority | DA-01, DA-07; authority handoff M0; tooling M4 |
| CSA-10 | Accessibility, frontend async/listener lifecycle, real interaction evidence | DA-04, DA-07; desktop M3/M4 |
| CSA-11 | Dependency/lock/source/Python/license/SBOM authority | DA-01, DA-07 and execution prerequisites; runtime/coverage M2/M4/M5 |

| Old acceptance IDs | Preserved observable obligation | New acceptance / sequence |
| --- | --- | --- |
| SDC-A1, SDC-A2 | Rust-authorized loaders; strict worker identity/cache segregation | DA-04/02 M2 |
| SDC-A3, SDC-A4 | Typed admission; unchanged destination/history on rejection; timed-out validator killed/reaped | DA-04 M2/M4 |
| SDC-A5, SDC-A6 | Every restore/HMR/retry/history path avoids generated execution; real desktop hostile discovery proves non-execution and usability | DA-04/07 M3/M4/M5 |
| ALB-01, ALB-06 | Session/scheduler-only runtime runs; retired feature/export/adapters removed while retained non-runtime behavior works | DA-02/03 M2/M4 |
| ALB-02 | Checked Rustler boundary, no binding-owned runtime/block_on/detached callbacks; real BEAM host | DA-02/04/05/07 M4/M5 |
| ALB-03 | Application-lifetime UniFFI surface; checked conversions/errors and bounded event behavior through generated C# | DA-02/04/05 M4 |
| ALB-04, ALB-05 | Both process adapters own child/readers; bounded events and explicit cancellation/drain; admission-close and complete/incomplete/failed shutdown through C# | DA-04/05 M2/M4/M5 |
| FE-A01 | Action-specific complete response/event decoding and real Tauri producer/consumer evidence | DA-05/04 M3/M4 |
| FE-A02 | Async terminal observation, cleanup, no duplicates/stale state | DA-04 M3/M4 |
| FE-A03, FE-A04 | Package-owned graph interface, no competing app policy/deep source import; backend-authored connection intent | DA-02/03/05 M3/M4 |
| FE-A05 | Browser persistence bounds/version/migration/typed outcomes before application | DA-05 M3/M4; simulated original evidence does not close new cold-reopen promise |
| FE-A06 | Selected graph/configuration/inspection tasks have accessible names/state/focus, pointer/keyboard/cancellation/feedback/cleanup in WebKit/Tauri | DA-07 M3/M4/M5 |
| VT-A01 | Complete claim facts, owner, procedure, schedule, state and unavailable behavior | DA-01/07 M0/M1/M5; replace prescribed registry mechanism only if equivalent proportionate evidence preserves every fact |
| VT-A02 | Automatic frontend discovery and every Rust package/target claimed or explicitly dispositioned | DA-01/07 M4/M5 |
| VT-A03 | Exact staged or explicit base/head traceability, prior/current impact-map state, no unrelated-document escape | DA-01/07 M4 |
| VT-A04 | Static gates claim only their decidable property; stable governed debt and new-finding failure | DA-07 M4/M5 |
| VT-A05 | Compiler-aware Svelte and real graph pointer/keyboard/focus evidence | DA-07 M3/M4/M5 |
| VT-A06 | Exact packaged native load/identity and desktop ready state on every required target | DA-07 M5; retains required-real release-artifact kind |
| VT-A07 | Local/hook/CI/launcher/release agree on selected claims and distinct failure/unavailable/cancelled/unselected states and documentation | DA-07 M4/M5; no new registry imposed solely by old tool prescription |
| DRD-A01, DRD-A02 | All npm/Cargo/Python/Git/Actions/system/generator/release-tool requirements have owned identities and lifecycle; single active locked resolution/Pumas identity | DA-01/07 M1/M2/M4 |
| DRD-A03 | Owner-selected project/package license and provenance/obligations/notices agree in source and shipped artifact | DA-07 M4/M5; blocked on owner terms; retains release-artifact/representative/manual |
| DRD-A04 | Single current plan, unique ADR index, truthful setup/status/standards docs | DA-01 M0/M4; image-specific plan-owner wording superseded by this unified plan; accepted consolidation remains historical evidence |
| DRD-A05 | Complete artifact-role/version/revision/target/architecture/ABI/cohort/consumer contract | DA-07 M0/M4; unresolved release authority |
| DRD-A06 | Pinned-tool metadata/notices/SBOM/checksums/provenance describe final bytes | DA-07 M5; retains required-real release-artifact proof |
| DRD-A07 | Immutable authorized candidate, complete required matrix and exact artifact load/start; publication blocked until claims/destination authority resolved | DA-07 M5; candidate acceptance retained, publication deferred/out of scope |
| DRD-A08 | Launcher checks actual declared versions/identities; mutation only under --install and recheck; preserves dependency/delegate failures | DA-04/07 M4 |
| IMG-A01 | One inference interface drives displayed ports, graph validation, materialization and exact dispatch diagnostics | DA-02/03 M2/M3 |
| IMG-A02 | Every compatible 1..N group uses canonical scheduler/host dispatch, including solo assignment | DA-02/03/04 M2 |
| IMG-A03, IMG-A05 | Real owners pass affected suites; no fabricated runtime facts, raw paths or direct/legacy/frontend policy execution | DA-02/03 M2 |
| IMG-A04 | Real editor submission and retained image in I/O Inspector | DA-03 M3; strengthened to dependent text→image with both outputs |
| CSR-A01, CSR-A02, CSR-A03 | Aggregate SDC, ALB and FE acceptance respectively | DA-02/03/04/05/07 M2–M5 via rows above |
| CSR-A04, CSR-A05 | Aggregate VT and DRD acceptance respectively | DA-01/07 M4/M5; artifact claims remain artifact claims |
| CSR-A06 | Every CSA disposition closed and no critical/high finding silently downgraded; one final material revision/candidate | DA-01/07 M5; old-child Accepted status requirement superseded by single-plan claim reconciliation, not by omitting obligations |

Old child ledgers record plan creation, not accepted product remediation. The
old image ledger explicitly has no accepted real editor-to-artifact run.
The accepted documentation-consolidation ledger preserves its 2026-09-03
cleanup/link/index results; those results do not prove executable consumers of
removed documentation were migrated (see RR-02).

PORT-I01 transfers to the license blocker above; PORT-I02 to external-consumer
discovery; PORT-I03 to claim-specific unavailable real environments; PORT-I04
to the integrator's serial shared-file ownership. None is dropped by retiring
the old portfolio. Historical issue links remain investigation evidence.

## First Actionable Findings And Bounded Pilots

### RR-01 — Frontend discovery still omits 19 maintained tests (High)

`package.json` hard-codes 71 test filenames. Comparing them with tracked
`src/**/*.test.ts` and `packages/**/*.test.ts` finds 90 files, including these
19 omitted suites:

```text
packages/svelte-graph/src/connectionDragState.test.ts
packages/svelte-graph/src/cutInteraction.test.ts
packages/svelte-graph/src/graphRevision.test.ts
packages/svelte-graph/src/horseshoeInvocation.test.ts
packages/svelte-graph/src/horseshoeSelector.test.ts
packages/svelte-graph/src/reconnectInteraction.test.ts
src/components/deviceConfigPresenters.test.ts
src/components/deviceConfigRefreshScope.test.ts
src/components/nodes/workflow/inferenceValidationDisplay.test.ts
src/components/nodes/workflow/pumaLibNodeState.test.ts
src/components/reconnectInteraction.test.ts
src/components/workbench/graphInspectionPresenters.test.ts
src/components/workbench/nodeLabPresenters.test.ts
src/components/workbench/settingsPagePresenters.test.ts
src/components/workflowInferenceDriftEdgeOverlays.test.ts
src/components/workflowValidationProjectionOverlays.test.ts
src/lib/tauriConnectionIntentWire.test.ts
src/services/workflow/WorkflowGraphValidationLifecycleSubscriptionService.test.ts
src/services/workflow/WorkflowService.graphInspection.test.ts
```

Owner: frontend test tooling. Consumers: developer command and quality-gates
CI, which both invoke `npm run test:frontend`. Fix candidate: replace filename
registration with bounded automatic discovery preserving Node's test process
status; use the pinned Node runtime's native facility where sufficient.
Exact candidate write set: `package.json`, and only if native discovery cannot
satisfy tracked-root selection, `scripts/run-frontend-tests.mjs` plus a focused
runner test. No package-lock change is inherently necessary.
Admission must state whether local untracked tests are included; VT-A02 requires
all tracked tests and does not require excluding untracked developer tests.

Deciding evidence: selected set contains all 90 files; a newly added temporary
nested test is discovered in an owned root; a failing test makes the canonical
command fail; paths outside owned roots are not selected; `npm run test:frontend`
passes the real full set. Run affected script ESLint if a runner is added.
The test count alone is not a success oracle. This is a useful local behavior
pilot, not a broad verification-registry project. No suite pass is claimed here.

### RR-02 — Packaging consumes a deleted documentation path (Medium)

`scripts/package-uniffi-csharp-artifacts.sh:76,91,107,122` copies or declares
`docs/headless-native-bindings.md`, which does not exist; the current guide is
`docs/headless-workflow.md`. The missing copy necessarily fails after the build
and generator succeed. `.github/workflows/headless-embedding-contract.yml`
calls the packager before the packaged quickstart. This is an executable
consumer regression after the documentation migration, independent of broader
artifact identity defects.

Owner: packaging/documentation integration. Candidate write set:
`scripts/package-uniffi-csharp-artifacts.sh`, with
`bindings/csharp/PACKAGE-README.md` only if package-facing guidance must change.
The destination choice must keep manifest entries and actual bundled paths
consistent. Check the current guide's local links in its packaged layout;
copying a current guide must not introduce broken package-relative navigation.
Do not restore a deleted duplicate guide as a new authority.

Deciding evidence: `bash -n`, bounded isolated packaging fixture with fake
build/generator commands that verifies both archives' actual guide bytes and
manifest targets and propagates a missing source failure. This simulation
proves the copy/manifest repair only. Existing real package/quickstart command
remains the binding artifact gate when tooling is available; neither the fixture
nor this repair closes DRD-A05–A07. This can be a modest multi-consumer pilot
if the complete package documentation dependency set remains bounded.

### RR-03 — Retired traceability policy remains executable (High)

`scripts/check-decision-traceability.sh:4–25` still assigns directory-based
documentation obligations and eleven mandatory headings; `resolve_diff_range`
guesses branches/revisions, and `changed_files_for_mode` excludes deleted paths.
Those contradict current Documentation's exact impact/source-state contract.
Owner: tooling/documentation; carried CSA-09/VT-A03, M4. A bounded source-impact
and prior/current Git fixture contract must precede replacement; do not write
new READMEs to satisfy it or disable required traceability silently. This is
not the first low-risk pilot because its real impact authority is unresolved.

### RR-04 — Dependency/target scaffolding does not prove its labels (Medium / unresolved)

Root Cargo and `Cargo.lock` select Pumas `f87c3da...`; quality CI still checks
out sibling Pumas `66c0c11...`, although current manifests reference the Git
workspace dependency. Establish whether any remaining consumer uses that
sibling before consolidating it. Python requirements retain unconstrained
packages and a Linux x86_64 CPython 3.12/CUDA-specific wheel, so importing a
package or having a venv cannot prove a supported dependency profile.
Packager platform overrides can label bytes independently of target identity.
These support continued CSA-11/08 review, not an unreviewed dependency update
or assumed platform matrix. Owner: dependency/release tooling; M4, with runtime
prerequisites resolved earlier where they block the selected mixed fixture.

## Stopping Point

Coverage population and old-claim mapping are bounded, legal/release/consumer
unknowns remain visible, and two reversible tooling repairs have concrete
owners and checks. This suffices for integrator admission decisions while
execution-path work proceeds. It does not close DA-01: full binding lifecycle,
per-target Rust test execution, remaining launcher/checker behavior, persisted
database provenance, all asset obligations and final artifact claims still
need source-backed disposition before M5.
