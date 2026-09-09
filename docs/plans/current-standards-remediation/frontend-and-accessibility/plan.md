# Plan: Frontend And Accessibility Remediation

> Superseded on 2026-09-08 by the [domain architecture and multimodal plan](../../domain-architecture-and-multimodal/plan.md).
> The remaining body is historical scope/evidence, not implementation authority.
> Outstanding claims and findings transfer to the successor; none are accepted by supersession.

**Plan status:** `Superseded`

**Current phase:** Superseded by the domain architecture and multimodal plan.

**Next slice:** `none`

**Acceptance status:** `pending`

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

**Reports:** `none`

**Related ADRs:** [ADR index](../../../adr/README.md)

**Source audit:** [03-frontend-and-accessibility.md](../../../audits/2026-09-03-current-standards/03-frontend-and-accessibility.md)

## Objective

Give Pantograph one decoded frontend projection of Tauri state, one reusable graph-editor owner, lifecycle-safe asynchronous UI work, validated browser persistence, and claim-matched accessible interaction evidence.

The target composition is:

```text
Rust/Tauri producer -> Tauri transport adapter -> action decoder -> domain adapter/store -> Svelte projection
                                                    |
                                                    +-- invalid / unsupported / unavailable

Pantograph workbench -> @pantograph/svelte-graph public Interface -> WorkflowBackend port
                                                                  /                 \
                                                     Tauri adapter                   configured test adapter
```

The frontend owns presentation state, rendering lifecycle, and interaction adaptation. It does not infer domain state, connection policy, or successful work from TypeScript generics, persisted bytes, the rendered tree, or an async start.

## Scope

- Every Tauri `invoke` response and `listen` event consumed under `src/`, including workflow commands/projections and `TauriWorkflowBackend`.
- Frontend timers, subscriptions, listeners, animation work, refresh loops, and overlapping state-applying promises under `src/` and `packages/svelte-graph/src/`.
- Competing app/package graph components, stores, helpers, deep imports, connection intent, and local compatibility fallback.
- Browser-persisted state read by the root app or graph package.
- User-facing controls and status semantics implicated by FE-06, plus graph pointer, keyboard, focus, cancellation, and parent-gesture behavior.
- Focused deterministic tests, real producer/consumer contract evidence, and representative browser evidence for those outcomes.

## Non-goals

- Owning Rust domain validation, scheduler policy, runtime readiness, graph validity, or persistence of backend workflows.
- Dynamic generated-component authorization/execution; the security remediation owns it.
- Selecting an external accessibility standard, conformance level, certification, screen-reader product, or modality not established by the product contract.
- Replacing all Svelte imperative DOM use. It remains valid where the renderer requires it and ownership, synchronization, and cleanup are proven.
- Creating a universal decoder registry, async-scope framework, or generic state-management layer.
- Owning test discovery, ESLint/checker policy, CI scheduling, or browser-runner orchestration; the verification/tooling plan owns those mechanisms.
- Changing package/release version policy. The release/documentation remediation owns public support and compatibility promises.

## Constraints And Assumptions

- `@pantograph/svelte-graph` declares that it owns reusable graph-editor components, stores, and interaction helpers; the root app owns workbench composition and Pantograph-specific projections.
- The existing `WorkflowBackend` Interface is a real seam: `TauriWorkflowBackend` and the configured mock/test adapter are materially distinct Adapters.
- A TypeScript type or Tauri generic is never runtime proof. Domain adapters receive `unknown`, decode the complete action-specific value, and expose only closed validated variants.
- No external accessibility conformance authority is currently established. Acceptance is limited to the named Pantograph tasks, WebKit/Tauri desktop platform, pointer and keyboard interaction, programmatic role/name/state, focus lifecycle, and visible status/error feedback defined here.
- Package exports may be consumer-facing because `@pantograph/svelte-graph` has an exported package Interface. Before removing or incompatibly changing an export, Milestone 0 must identify consumers and the release owner must classify the compatibility promise.
- Shared `package.json`, scripts, CI, and browser harness files remain with verification/tooling. Product implementation and evidence orchestration integrate serially.

## Objective Acceptance

| ID | Observable criterion | Kind | Environment | Mode | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| FE-A01 | Every in-scope Tauri response/event is action-specifically decoded from `unknown` before dispatch; malformed, mismatched, extra-field, unknown, and unavailable-decoder cases preserve typed outcomes. | `contract` | `representative` | `automated` | `pending` | Decoder fixtures plus the real Tauri producer/consumer claim `VP-HOST-BINDING-001` |
| FE-A02 | Every in-scope async owner observes success/failure/cancellation/supersession, releases resources at teardown, prevents duplicate work, and prevents a stale invocation from mutating current state. | `integration` | `simulated` | `automated` | `pending` | Owner-level lifecycle tests with controlled schedulers/clocks and explicit terminal assertions |
| FE-A03 | The root app consumes the package graph Interface without a parallel full editor, copied interaction policy, or deep source import; product-specific workbench composition remains in the app. | `integration` | `representative` | `automated` | `pending` | Package-consumer build plus graph user workflow in the real WebKit/Tauri host |
| FE-A04 | Connection affordance and commit state come from backend-authored intent; absent/mismatched intent is unavailable or invalid and never guessed valid by local port inspection. | `contract` | `representative` | `automated` | `pending` | Connection-intent fixtures plus real Tauri graph-interaction path |
| FE-A05 | Every in-scope browser-persisted record is decoded from `unknown` with its owned schema, bounds, migration/version behavior, and typed invalid/unsupported/unavailable outcome before state application. | `contract` | `simulated` | `automated` | `pending` | Storage-adapter contract tests covering legacy, current, future, malformed, unavailable, repeated, and isolated stores |
| FE-A06 | The selected editor/configuration/inspection tasks expose meaningful role, name, state, focus, keyboard/pointer activation or cancellation, feedback, and cleanup in the representative WebKit/Tauri host. | `user-workflow` | `representative` | `automated` | `pending` | `VP-GRAPH-INTERACTION-001` plus a verification-owned frontend-accessibility browser claim for non-graph affected tasks |

Supporting format, lint, compiler, and build gates prove only their named static properties. Exact command, environment facts, result, and implementation revision must be recorded in the ledger before an acceptance claim becomes satisfied.

## Binding Decisions

| Decision | Owner | Replaces |
| --- | --- | --- |
| Only `src/lib/ipc/` imports raw Tauri `invoke`/`listen`. Its small transport Interface accepts action-owned decoders; domain services and stores expose validated values. | Frontend IPC adapter | Direct typed Tauri calls and `invoke<T>` as proof |
| Decoders live with the action/domain contract, not in one universal schema registry. The transport maps only transport lifecycle and shared typed outcome representation. | Domain adapter plus Contracts | Copied response shapes and envelope-only validation |
| `@pantograph/svelte-graph` owns reusable graph rendering and interaction policy. The app may retain a thin, explicitly named Pantograph Adapter for workbench/system-graph composition, not a copied editor implementation. | Graph package; app composition root | Parallel `WorkflowGraph`, `NodePalette`, and `WorkflowToolbar` owners |
| Backend-authored connection intent is authoritative. The test Adapter returns configured intent; it does not claim semantic parity through a hand-copied Rust compatibility table. | Workflow backend producer | Visual fail-open compatibility fallback |
| Each Svelte module/store owns the identity and lifecycle of work that can outlive a call. Mechanisms stay local unless multiple owners share one proven invariant. | Component/store lifecycle owner | Detached promises, async mount cleanup, and global counters |
| Each persisted-record owner decodes its own shape through an injected browser-storage Interface. Recognized legacy data migrates explicitly; unknown future data is retained and reported, not silently overwritten. | Store/domain owner | Partial `JSON.parse` and guessed defaults |
| Accessibility acceptance is a user-task contract, not the regex checker. Compiler/lint diagnostics remain supporting evidence under verification/tooling. | Frontend interaction owner and Verification | Syntax-only accessibility confidence |

## Systemic Finding Audit

- **Invariant families and canonical owners:** independently produced values belong to action decoders; runtime-backed domain state belongs to Rust/Tauri producers; graph interaction belongs to the package; product composition belongs to the app; invocation identity belongs to the affected component/store; each persisted record belongs to its store; user-access outcomes belong to the named user task.
- **Bounded authority, representation, and reachable consumers:** all raw `invoke`/`listen` imports and wrappers in `src/`; all state-applying promises, timers, listeners, subscriptions, observers, and animation callbacks in `src/` and `packages/svelte-graph/src/`; the three parallel graph components and their helper/store imports; all `JSON.parse` and browser-storage reads in those roots; all affected interactive Svelte markup and the representative browser host.
- **Expansion facts:** a new command/event, package consumer/export, persistence key/schema, async owner, user task, supported modality/platform, or externally selected conformance promise expands the population and requires a plan update.
- **Consumer dispositions:** every occurrence is classified as canonical owner, validated Adapter, presentation consumer, test Adapter, migrated legacy record, justified renderer mechanism, or deleted. No `unknown`, guessed-success, detached, deep-import, or unowned survivor is accepted.
- **Alternatives considered:** delete parallel modules and fallbacks before adding machinery; consolidate raw transport imports while keeping action decoders local; use the existing `WorkflowBackend` seam; use scoped identity/cancellation instead of a global async framework; inject the platform `Storage` Interface instead of adding a storage service; replace syntax-only checks with claim-directed compiler/browser evidence.
- **Evidence-backed stopping condition:** the bounded searches have recorded dispositions, FE-A01 through FE-A06 are satisfied, the package/root dependency direction is clean, and no affected user task relies on stale state, guessed validity, inaccessible required interaction, or unobserved work.
- **Repaired-composition comparison:** one transport Adapter plus domain decoders replaces many trusted generic calls; one package editor plus one app composition Adapter replaces parallel implementations; owned lifecycle and record decoders replace ambient browser state and best-effort cleanup.

## Simplicity And Ownership Review

**Applicability:** `applicable`

- Independent concepts and dimensions: IPC representation proof, domain projection, reusable graph interaction, product composition, invocation lifecycle, persisted UI state, accessibility outcome, and verification orchestration change for different reasons and remain separate.
- State, identity, value, time, policy, and mechanism: current workflow/session/connection values come from decoded producer state; invocation identity is scoped to the state it may update; persistence identity is the owned storage key plus schema version; time matters only to animation, refresh, and supersession owners; accessibility policy is derived from named tasks; Tauri, Svelte, WebKit, timers, and `localStorage` are mechanisms.
  - **Canonical authority scope and referenced authorities:** Rust owns serialized domain facts and connection decisions; frontend domain adapters own decoding/projection; the package owns reusable editor behavior; the app owns workbench composition; record owners reference browser storage; Verification owns claim execution.
  - **Version roles and owned promises:** IPC action schemas, package `0.1.0`, persisted-record versions, app/release version, and backend graph revision are distinct. No general IPC protocol version is invented. Release owns package compatibility; each store owns its record migration; graph revision owns connection-intent freshness.
  - **Supported compatibility overlaps and consumer matrix:** the root app is the proven package consumer; external package consumers are `unavailable` until M0 inventory. Tauri and configured test implementations are the two `WorkflowBackend` Adapters. Legacy/current persisted formats overlap only for the declared migration window.
  - **Material identity-invalidation effects:** a new graph revision invalidates stale connection intent; a newer invocation supersedes state application by an older one; unmount/dispose invalidates component work; a storage-version change invokes migration or typed unsupported; session/workflow identity changes prevent cross-session application.
- Caller and composition-root knowledge: `App.svelte` constructs/injects the Tauri backend and package graph context; `GraphPage.svelte` composes package controls with workbench state; components know validated domain methods and presentation outcomes, not raw command names, wire casts, Rust compatibility rules, or transport lifecycle.
- Representative change paths and forced owners: a Tauri response-field change touches the Rust contract, its domain decoder, and contract fixture—not arbitrary components; a reusable graph interaction change touches the package and browser claim—not a copied app editor; a Pantograph-only system graph change touches the app Adapter; a persisted shape change touches one record decoder/migration; a focus behavior change touches the owning interaction and browser evidence.
- Stable Interfaces versus hidden knowledge: stable Interfaces are the domain methods, `WorkflowBackend`, package exports, store actions/results, and typed outcomes. Wire field layout, raw Tauri errors, listener handles, timers, generation tokens, storage serialization, and DOM mechanism remain hidden implementation knowledge.
- Independent evolution, testing, failure, and replacement: action decoders have isolated fixtures; package interaction runs with a configured Adapter; app composition runs with Tauri; store lifecycles and persistence use controlled substitutes; the real browser proves semantics the substitutes cannot. Each can fail with its own typed outcome without activating an alternate authority.
- Necessary complexity and containment: action-specific runtime proof, current-invocation identity, graph revision, record migration, and browser focus/pointer behavior are inherent. They are contained in deep Modules whose Interfaces give callers Leverage and Locality; no giant registry or pass-through wrapper is admitted.
- Deletion and cumulative machinery result: delete direct Tauri imports, permissive casts/defaults, app/package duplicate editors, copied port policy, deep imports, async mount cleanup, and superseded syntax-only assumptions. Retain only one small transport Adapter, domain decoders, the existing backend seam, scoped lifecycle mechanisms, record decoders, and claim-matched evidence.

## Milestones

### Milestone 0: Contract And Population Inventory

**Goal:** Turn the audit's representative findings into complete bounded migration matrices before product edits.

**Allowed write set:** this plan directory only.

**Tasks:**

- [ ] Record every raw Tauri response/event consumer with action, producer, decoder owner, dispatch consumer, and typed outcomes.
- [ ] Record every lifecycle-owned async occurrence with protected state, invocation owner, completion, cancellation/supersession, teardown, and disposition.
- [ ] Record package exports/importers, local graph parallels, deep imports, legacy store/service bridges, and connection-policy consumers.
- [ ] Record every `JSON.parse`/browser-storage occurrence by record owner, schema, bounds, persistence promise, migration, and consumer.
- [ ] Record supported affected user tasks, role/name/state, pointer/keyboard/focus/feedback requirements, platform, and evidence claim. Classify syntax-checker findings as confirmed defect, justified mechanism, or false proxy.

**Acceptance gate:** Every bounded occurrence has one owner and non-blocked disposition; unknown external package consumers and missing user-contract facts remain explicitly `unavailable` and trigger re-planning before incompatible change.

**Status:** `Planned`

### Milestone 1: Decode The Complete Tauri Consumer Surface

**Goal:** Make runtime decoding unavoidable before independently produced values enter frontend state.

**Allowed write set:**

- `src/lib/ipc/`
- IPC-consuming files recorded by M0 under `src/services/`, `src/backends/`, `src/stores/`, `src/components/`, and `src/lib/hotload-sandbox/`
- `src/App.svelte`
- `src/lib/tauriConnectionIntentWire.ts`
- adjacent `*.test.ts` files for those consumers
- `packages/svelte-graph/src/types/backend.ts` only if the validated Adapter result contract must change

**Tasks:**

- [ ] Introduce the small injectable Tauri transport Interface and production Adapter; make it the only raw `invoke`/`listen` importer.
- [ ] Move complete, closed, action-specific decoders beside their domain adapters and migrate all M0 consumers from `invoke<T>`, assertions, empty/default values, and envelope-only proof.
- [ ] Decode category/action, payload, nested fields, enums, bounds, correlations, and explicit extra-field policy before dispatch; preserve invalid, unsupported, unavailable, transport failure, and cancellation distinctly.
- [ ] Add representative valid fixtures from actual Rust serialization plus one-condition negative fixtures; prove listeners never receive raw payloads.

**Acceptance gate:**

```bash
node --experimental-strip-types --test src/lib/ipc/tauriTransport.test.ts src/lib/ipc/tauriContractMatrix.test.ts src/services/workflow/workflowIpcContract.test.ts
npm run typecheck
npm run verify -- --claim VP-HOST-BINDING-001
```

The M0 source scan must show no raw Tauri transport import outside the one Adapter. Compiler success alone does not satisfy FE-A01.

**Migration/rollback:** Migrate one domain adapter at a time, removing its old route in the same slice. Roll back decoder, adapter, and callers together; never retain a permissive fallback.

**Status:** `Planned`

### Milestone 2: Establish One Graph Editor And Connection Authority

**Goal:** Make the package the reusable graph-editor owner and the backend the connection-policy owner.

**Allowed write set:**

- `packages/svelte-graph/src/index.ts`
- `packages/svelte-graph/src/components/`
- `packages/svelte-graph/src/context/`
- `packages/svelte-graph/src/stores/`
- `packages/svelte-graph/src/types/`
- `packages/svelte-graph/src/backends/`
- `packages/svelte-graph/src/workflowConnections.ts`
- `packages/svelte-graph/src/portTypeCompatibility.ts`
- adjacent package graph tests and READMEs
- `src/components/WorkflowGraph*`
- `src/components/NodePalette.svelte`
- `src/components/WorkflowToolbar.svelte`
- `src/components/UnifiedGraphView.svelte`
- app graph helper/test files identified by M0
- `src/components/workbench/GraphPage.svelte`
- `src/components/workflowToolbarEvents.ts`
- `src/stores/architectureStore.ts`
- `src/config/architecture.ts`
- `src/stores/storeInstances.ts`
- affected app graph tests and READMEs

**Tasks:**

- [ ] Inventory package export compatibility, then make root graph composition consume public package exports only.
- [ ] Move product-only system-graph/workbench behavior into a thin named app Adapter or explicit inputs; delete parallel full components, copied interaction helpers/tests, legacy synchronization, and deep source imports.
- [ ] Remove local production port-compatibility inference and fail-open `true`; missing/stale backend intent returns unavailable/invalid and prevents commit.
- [ ] Make the configured test Adapter return explicit candidate/commit fixtures instead of claiming Rust-policy parity.
- [ ] Remove the retired `execute_workflow` projection and other architecture descriptions invalidated by the consolidation.

**Acceptance gate:**

```bash
node --experimental-strip-types --test packages/svelte-graph/src/workflowConnections.test.ts packages/svelte-graph/src/backends/MockWorkflowBackend.test.ts src/components/graphOwnerContract.test.ts
npm run typecheck
npm run build
npm run verify -- --claim VP-GRAPH-INTERACTION-001
```

The import/export inventory must show no deep package import and no app-owned duplicate full editor. Backend rejection and unavailable intent must be observable in both deterministic and representative paths.

**Migration/rollback:** Change package Interface, app Adapter, tests, and docs atomically. Preserve compatible public exports unless M0 and release authority explicitly admit removal; do not restore copied app policy as rollback.

**Status:** `Planned`

### Milestone 3: Own Frontend Async Lifecycles

**Goal:** Ensure every in-scope operation has scoped invocation authority and observable cleanup.

**Allowed write set:** M0-classified lifecycle-owner files under:

- `src/components/`
- `src/services/`
- `src/stores/`
- `src/lib/hotload-sandbox/`
- `src/App.svelte`
- `packages/svelte-graph/src/components/`
- `packages/svelte-graph/src/stores/`
- `packages/svelte-graph/src/workflowGraphWindowListeners.ts`
- adjacent lifecycle tests

**Tasks:**

- [ ] Register Svelte teardown synchronously; track async listener registration so unmount-before-resolution still removes the listener and classifies completion.
- [ ] Repair canonical graph window listeners, `DeviceConfig`, `createViewStores`, and every other M0-invalid owner for duplicate starts, overlap, cancellation/supersession, dependency changes, and teardown.
- [ ] Scope generation/request identity to the protected state; observe a superseded promise even when its result cannot be applied.
- [ ] Retain cleanup handles for timers, animation frames, subscriptions, and observers; test failure and repeated mount/dispose, not only successful update.

**Acceptance gate:**

```bash
node --experimental-strip-types --test packages/svelte-graph/src/workflowGraphWindowListeners.test.ts packages/svelte-graph/src/stores/createViewStores.test.ts src/components/deviceConfigRefreshScope.test.ts src/components/frontendLifecycleContract.test.ts
npm run typecheck
npm run verify -- --claim VP-GRAPH-INTERACTION-001
```

**Migration/rollback:** Change each owner and its lifecycle evidence together. Quiesce timers/listeners before interactive rollback; never reintroduce `onMount(async ...)` cleanup or silent stale-result discard.

**Status:** `Planned`

### Milestone 4: Validate And Migrate Browser Persistence

**Goal:** Treat browser bytes as untrusted persisted contracts without making UI caches a second domain authority.

**Allowed write set:**

- `packages/svelte-graph/src/stores/createViewStores.ts`
- `packages/svelte-graph/src/stores/createSessionStores.ts`
- their adjacent tests/types
- `src/stores/promptHistoryStore.ts`
- `src/stores/graphSessionStore.ts`
- `src/stores/timelineStore.ts`
- `src/stores/undoStore.ts`
- `src/services/HotLoadRegistry.ts`
- `src/components/WorkflowToolbar.svelte`
- M0-classified persisted-record decoder/test files adjacent to those owners
- affected store/component READMEs

**Tasks:**

- [ ] Decode each key from `unknown`, including nested actions, identifiers, enum values, numeric bounds, array limits, extra fields, and cross-field rules.
- [ ] Inject the browser `Storage` Interface into store factories/record owners where tests currently depend on ambient globals.
- [ ] Define a current record version and explicit recognized-legacy migration. Preserve unknown future records and report unsupported; do not overwrite malformed/unavailable data as successful state.
- [ ] Keep these records presentation/cache state. Never reconstruct workflow/domain truth from browser storage.

**Acceptance gate:**

```bash
node --experimental-strip-types --test packages/svelte-graph/src/stores/createViewStores.test.ts packages/svelte-graph/src/stores/createSessionStores.test.ts src/stores/browserPersistenceContract.test.ts
npm run typecheck
```

The contract tests must cover recognized legacy, current, future version, malformed nested value, bounds, unavailable storage, repeated restore, and two isolated store instances.

**Migration/rollback:** Readers land before writers. Keep the recognized legacy key/shape for the declared compatibility window or write a parallel current key where the old reader cannot tolerate the new envelope. Removing legacy data requires release/documentation authority and explicit user impact; backend workflows are never deleted by this migration.

**Status:** `Planned`

### Milestone 5: Repair Semantics And Prove Accessible User Tasks

**Goal:** Make the selected graph editor, configuration, and inspection tasks perceivable and operable in the real frontend environment.

**Allowed write set:**

- affected files recorded by M0 under `src/components/` and `packages/svelte-graph/src/components/`
- adjacent interaction/presenter tests and styles
- component/package READMEs that state the retained interaction contract

**Tasks:**

- [ ] Repair confirmed `PumaLibNode`, `IoInspectorPage`, and inventory findings by user outcome; justify and lifecycle-test imperative renderer access instead of blindly deleting it.
- [ ] For selected controls/statuses, expose purpose, role, name, value/state, availability, validation/error relation, and change notification required by the task.
- [ ] Prove initial/moved/restored focus, visible focus, keyboard and pointer activation/cancellation, Escape behavior, pointer capture/release, and parent graph-gesture conflict wherever applicable.
- [ ] Ensure invalid, unsupported, and unavailable states remain visible and operable rather than becoming stale/empty success.
- [ ] Have verification/tooling replace or demote regex findings only after compiler/browser evidence covers their real claims.

**Acceptance gate:**

```bash
npm run lint:full
npm run typecheck
npm run verify -- --claim VP-GRAPH-INTERACTION-001
npm run verify -- --claim VP-FRONTEND-ACCESS-001
```

`VP-FRONTEND-ACCESS-001` must be registered and orchestrated by verification/tooling before this milestone. Its representative facts include the supported Tauri/WebKit target, display/session, isolated application state, bounded lifecycle, role/name/state observations, and real keyboard/pointer/focus results. Missing browser capability is `unavailable`, not component-test success.

**Migration/rollback:** Semantic and interaction changes roll back with their browser evidence. Do not retain inert controls, pointer-only fallback, or syntax-checker exceptions in place of task behavior.

**Status:** `Planned`

### Milestone 6: Objective Verification And Closeout

**Goal:** Satisfy FE-A01 through FE-A06 and remove transition-only machinery.

**Allowed write set:** this plan directory only.

**Tasks:**

- [ ] Re-run all milestone evidence on the final revision and record exact environment/results in the ledger.
- [ ] Re-run the five bounded population searches and record every retained occurrence and owner.
- [ ] Run verification-owned frontend discovery/compiler/static gates without weakening or relabeling failures.
- [ ] Apply the deletion test to the transport Adapter, package/app composition, lifecycle mechanisms, persisted decoders, and evidence; remove superseded tests rather than layering duplicates.
- [ ] Close or explicitly supersede every issue disposition.

**Acceptance gate:**

```bash
npm run verify -- --claim VP-HOST-BINDING-001
npm run verify -- --claim VP-GRAPH-INTERACTION-001
npm run verify -- --claim VP-FRONTEND-ACCESS-001
npm run typecheck
npm run lint:full
npm run build
npm run test:frontend
```

The three `verify` claims are deciding evidence. Compiler, lint, build, and discovered tests are supporting gates unless their named static property is itself under review.

**Status:** `Planned`

## Blockers

- `none` for Milestone 0.
- Milestone 5 and final FE-A06 acceptance wait for verification/tooling to provide the representative browser runner and register `VP-FRONTEND-ACCESS-001` without weakening `VP-GRAPH-INTERACTION-001`.
- An external package consumer or formal accessibility conformance promise discovered in M0 blocks incompatible implementation until its owner and required evidence are added.

## Re-Plan Triggers

- A new Tauri action/event, package consumer/export promise, persistence key/schema, async owner, user task, modality, platform, or conformance authority expands the bounded population.
- The real Rust producer contradicts a proposed frontend decoder or cannot preserve the required typed outcome.
- Product-only behavior cannot be expressed through the package Interface without exposing Pantograph domain policy or duplicating graph implementation.
- A retained local compatibility rule is required by a real non-Tauri package consumer; define that consumer's authority and evidence before admitting it.
- Representative WebKit/Tauri evidence cannot observe a required role, focus, pointer, keyboard, cleanup, or user-visible result.
- A milestone requires a file outside its allowed write set, an alternate state authority, or materially more permanent machinery than the composed-design review admits.

## Cross-Plan Dependencies

| Plan | Required result / ordering |
| --- | --- |
| Verification and tooling | Claim discovery/runner precedes final contract evidence; its representative frontend milestone owns WebDriver orchestration and must add `VP-FRONTEND-ACCESS-001`. It consumes this plan's interaction contract. Shared package/scripts/CI changes are serial. |
| Architecture, lifecycle, and bindings | Tauri/runtime work must preserve typed producer outcomes and one workflow execution authority consumed by FE-A01/FE-A04. Coordinate changes to `TauriWorkflowBackend` contract evidence. |
| Security and dynamic code | Owns generated-component authorization and execution. Frontend projection decodes its result but does not redefine trust or sandbox policy. |
| Dependencies, release, and documentation | Owns `@pantograph/svelte-graph` compatibility/publication facts, supported Tauri/WebKit targets, app/package versions, and retirement timing for legacy persisted formats. |
| Active image-generation graph work | Shared package/app graph files must be integrated or explicitly handed off before M2/M3; do not implement competing graph ownership changes concurrently. |

## Final Acceptance

- Acceptance status: `pending`
- Deferred follow-ups: `none`
- Final status: `Superseded`

The plan becomes `Accepted` only after FE-A01 through FE-A06 are satisfied on one recorded final revision, every bounded occurrence has a disposition, migration/compatibility effects are closed, and no permissive decoder, guessed connection validity, competing full graph owner, stale async application, unvalidated persisted record, or inaccessible selected interaction remains.
