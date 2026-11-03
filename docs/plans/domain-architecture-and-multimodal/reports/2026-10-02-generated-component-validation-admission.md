# Generated-component validation admission repair

Date: 2026-10-02. Scope: audit P-03. Status: implemented and independently
reviewed; composed gate passes, while exact-head CI and real-browser qualification
remain pending. Initial PR target: `fix/decision-traceability-ownership-2026-10-02`;
retarget `main` only after the prerequisite integrates. Admitted base:
`4938e405c7f656365eefdca492774ccae110c90d`. Branch:
`fix/generated-component-validation-admission`. No merge or release is authorized
by this report. The integration owner is the repository maintainer.

This is a bounded implementation under the active
[domain architecture plan](../plan.md). The parallel execution-repair owner owns
its plan, issues and ledger; this report supplies the validation slice's evidence
without competing edits to those controls. Coding-Standards was read at
`dcc56f26e884ade260770beceba2501d3746200d` (Core, Router, Implementation,
Verification/Oracles, Commit, Documentation, TypeScript/Async, Frontend,
Contracts/Protocols, IPC, Concurrency, Resilience, Diagnostics, Security,
Untrusted Execution, Code Design and Accessibility).

## Outcome and ownership

`ImportManager` must never call either generated-module import path after source
fetch or validation transport failure, malformed validation output, or explicit
invalid output. The Rust `validate_component` command in
`src-tauri/src/llm/commands/sandbox.rs` owns the wire result: `valid: bool` and
nullable `error: Option<String>`. The frontend decodes these consumed fields
before admission; absent error is supported by its existing optional-field
contract, and additional diagnostic fields are ignored. A type assertion or
truthy value cannot authorize import.

Import failures now distinguish `validation-unavailable`,
`validation-response-invalid`, `validation-invalid`, `import-failed`,
`import-timeout`, and `superseded`. No automatic retry is introduced. A caller or
user retries after restoring source/backend availability or correcting source.
Unavailable and malformed responses never enter the validation cache; explicit
valid/invalid results retain the existing content-hash cache behavior. Corrected
content is revalidated. An accepted imported component may still be returned
from the successful import cache until explicit invalidation.

`ComponentRegistry` owns candidate identity, accepted presentation state and retry
input. It retains the last accepted constructor/source/path/geometry while a
replacement is loading or failed, exposing a separate pending-update state.
Retry targets the failed candidate rather than the accepted source. The container
keeps its accepted render branch and displays the candidate failure and a native
retry button. Initial failures still produce the ordinary error placeholder.
HMR, manual refresh and registration share the same candidate lifecycle. Removed
or superseded candidates cannot publish late results. Import-manager invalidation
revokes admission/publication for that path; it does not cancel underlying IPC or
already started module evaluation.

### Bounded write set

- `src/lib/hotload-sandbox/services/ImportManager.ts`: fail-closed admission,
  runtime response decoding, typed failures, pending-request identity and timer
  cleanup
- `src/lib/hotload-sandbox/services/ComponentRegistry.ts`: accepted versus
  candidate state, retry input and current-request publication
- `src/lib/hotload-sandbox/types.ts`, `index.ts`: coordinated internal result and
  presentation contracts
- `src/lib/hotload-sandbox/components/ComponentContainer.svelte`: candidate status
  and retry while preserving the accepted render branch; correct initial loading
  state
- `src/lib/hotload-sandbox/services/ErrorReporter.ts`: explicit TypeScript import
  suffix only, so actual production modules run in the canonical Node test suite
- `validationAdmission.test.ts` and `test-support/importEnvironment.ts` beside
  those services: real implementation tests with only Vite and Tauri effects
  substituted
- This report

The caller changes are necessary because merely changing `{valid:true}` to failure
would otherwise discard accepted UI on registration and hide it on HMR failure.
The import manager owns path-level coalescing and invalidation; the registry owns
component-ID-level state. These are separate existing identities, not a new global
request counter or retry mechanism.

## Evidence and limits

The focused suite runs the actual `ImportManager`, `ComponentRegistry`,
`ErrorReporter` and `ValidationCache`, using Node resolution hooks only at Vite
and Tauri seams. Test cases exercise importer call counts, outcomes and registry
state; they do not copy the admission algorithm into a probe. Cases are isolated
by Node's test-file process and serial test ownership of the fetch/cache seam.
No real generated model code or production model calls are used.

Verification on the revised review candidate, 2026-10-02:

- `npm ci` completed with the existing lockfile and no manifest/lock changes
- `node --experimental-strip-types --test src/lib/hotload-sandbox/services/validationAdmission.test.ts`: 32 passed
- `npm run test:frontend`: 572 passed, zero failed
- `npm run typecheck`: passed
- Scoped ESLint and `npm run lint`: passed
- `npm run build`: passed (existing stale Browserslist data notice)
- `node --test scripts/run-frontend-tests.test.mjs`: 3 passed
- `npm run lint:critical`: fails at unchanged
  `src/components/workbench/IoInspectorPage.svelte:520`, `no-append-remove-child`;
  `npm run check` stops at this gate, so that command did not run its later steps
- `npm run lint:full`: one baseline error at
  `src/components/nodes/workflow/PumaLibNode.svelte:34`,
  `svelte/prefer-writable-derived`; no changed-file errors
- `npm run lint:a11y`: three baseline findings in IoInspectorPage,
  RunGraphSnapshot and SavedGraphInspectionSnapshot; none in this write set
- The original staged traceability check was blocked by four retired
  README-per-directory demands. Composition onto reviewed gate commit
  `9567186252f7ebb60794911baeeabce0daec5140` resolves that publication dependency;
  the composed staged gate passes without filler READMEs or a hook bypass
- Available Node/npm: 24.19.0 / 11.9.0; repository pins 24.12.0 / 11.6.2. npm reports
  this engine mismatch, so exact pinned-toolchain qualification is not claimed
- GUI attempt: local Chromium could not create required process sockets in the
  execution sandbox; the supported cloud browser refused the localhost test URL
  with `ERR_BLOCKED_BY_CLIENT`. No real-browser interaction, focus, retained DOM
  identity, or Tauri desktop user-workflow pass is claimed. The production Svelte
  build and deterministic registry assertions prove their narrower properties

The first independent review found three caller lifecycle defects: queued HMR
could resurrect a removed entry, completed refresh could overwrite newer user
geometry, and refreshing a shared path could falsely fail another live component
ID. The revised implementation checks queued entry/candidate identities, gives
later `updatePosition`/`updateSize` edits authority over starting candidate
geometry, and lets still-current consumers join only already-requested newer
path work (including its terminal failure). It does not start an automatic retry.
Seven new tests cover these cases, including clear/unregister/replacement during
a queued batch and shared-path valid/invalid results. One latest request promise
per path is retained for these consumers until the next request or cache clear.

Independent read-only re-review found no remaining blocking source issue in this
bounded admission/lifecycle slice and independently reran all 32 tests, passing.
The final changes after that review are indentation/comment-only. Focused draft publication is now prepared against gate [PR #2](https://github.com/MrScripty/Pantograph/pull/2).
Exact-head CI and applicable hosted review remain mandatory; the gate itself
still has unrelated failing CI. Unavailable
or failing aggregate evidence remains visible; a frontend subset is not Rust,
desktop, isolation or release acceptance.

## Follow-on P-04 and excluded protection claims

`src-tauri/src/hotload_sandbox/runtime_sandbox.rs` still spawns a native Boa thread
and returns timeout without joining or interrupting it. That timeout stops the
caller waiting; it does not establish cessation of worker effects. No infinite
loop was executed in this repair, and no native-worker/process redesign is part
of it. A separate proposal should establish owned interruptible evaluation or a
bounded worker-process lifecycle, concurrency admission, confirmed cessation,
and repeated-timeout/no-residual-activity tests in the actual native boundary.

The `validate_component` frontend command invokes a Node/esbuild validation
script; it is distinct from that Boa worker. This repair does not qualify either
validator's resource or termination behavior.

Generated code still executes in the application renderer. Shape/syntax
validation, a content hash, pre-import admission and global render-error handling
are not execution isolation or a security sandbox. The existing fetch → backend
path validation → Vite import sequence does not bind the exact evaluated bytes
against concurrent file changes; hash collisions and cache authority are also
outside this slice. An execution-trust/isolation contract and validation-to-use
binding require separate design and evidence. P-03 closure must not be described
as P-04 closure or a generated-code security guarantee.

## Composed-tree verification

The branch fast-forwarded onto gate `9567186` while preserving its entire staged
implementation patch byte-for-byte. Fresh executions on that source passed
32 validation-admission tests, all 572 frontend tests, TypeScript typecheck and
tooling lint. Staged traceability and whitespace checks also pass. The CommonMark
development dependency is supplied from the gate's qualified locked workspace
installation; existing application dependency versions remain unchanged.
Previously recorded build, baseline lint and browser results retain their original
scope. No browser/desktop acceptance or clean aggregate CI is implied.
