# Cloud-owned native CPU desktop qualification

The accepted chat candidate is frozen at
`0d7d573f36d8f25f3ae5c8adb25ab86c42e1dd8c`, tree
`4f8b991baa72b537d6cb1a96c7450c019aec1577`. Normal Git fetch confirmed this
source and main `c75fa2379a730833709ea8976075bf17a117040f` before the isolated
`qual/native-desktop-cpu-graph` worktree was created. PR60/PR61 metadata and merge
actions remain parent-owned. Frozen features, main and existing CI files are
unchanged.

## Intended acceptance

Run the existing production Tauri desktop under official `tauri-driver` and
WebKitWebDriver on a cloud-owned display. Save and reopen a canonical
`text-input.text → llm-inference(embedding).text → vector-output.vector` graph
through actual Tauri IPC, show its nodes and typed ports, submit through the
production editor/public scheduler, and compare retained native Candle CPU output
with the committed width-8 BERT numerical reference. Record exact source,
screenshots, saved graph, scoped run/artifacts and selected `candle.cpu`/`cpu`
metadata. A missing native prerequisite or blocked submit gate is a failure with
evidence, never a browser-mock pass.

The first branch-specific hosted workflow had only `contents: read` and a bounded
30-minute timeout. Existing workflow permissions, checks and timeouts are
preserved. It verified committed fixture hashes, recorded the proposed graph and
performed read-only native prerequisite admission **before any install or build**.
It did not invoke sudo, apt installation, sandbox disabling, privileged displays
or access-denial workarounds. The expected normal Linux prerequisites include
GTK/WebKit development libraries, WebKitWebDriver, Xvfb and tauri-driver. Official
[Tauri CI instructions](https://v2.tauri.app/develop/tests/webdriver/ci/) use sudo
to install system packages; that installation route conflicts with this task's
first attempt's no-privilege constraint and was not invoked. The authorized
setup successor below supersedes that dependency-installation constraint.

## Prepared route and honest scope

`prepare-native-desktop-cpu-fixture.py` copies the existing committed untrained
BERT fixture after verifying its reference manifest. It creates an isolated
project and seeds synthetic metadata into a separate Pumas library. This is
controlled discovery data, not real-user library/discovery acceptance. It
downloads no weights and creates no user-library mutations.

The new WebdriverIO test calls the production `save_workflow`/`load_workflow`
commands using actual WebView Tauri IPC, opens the production graph editor,
requires visible input/output handles, follows the normal interface-update
review when needed, and clicks the existing submit control. It requires one
scoped run and retained vector/selection metadata before asserting the actual
CPU oracle. Screenshots and DOM/error details are saved on native failure.
There is no replacement inference executor, synthetic output injection, IPC mock
or submit-gate bypass. Native library enrollment, current descriptor adoption,
editor submission and retained artifact shape remain unqualified until the
native test actually runs and passes.

Before any proposed desktop build, the complete effective all-target Cargo graph
for `pantograph --no-default-features --features
backend-candle,tauri/custom-protocol` is checked. It retains current Pumas
`26a84e32` and dynamic ORT `load-dynamic`/`disable-linking`, rejects download
features, and always sets `ORT_SKIP_DOWNLOAD=1`. No denied binary download is
retried. Normal Node/Rust dependencies are installed only after native admission;
existing no-build-download policy checks are also retained.

## Preserved first attempt

The local read-only probe exits 2: GTK, WebKit/JavaScriptCore/libsoup development
metadata, WebKitWebDriver, Xvfb, tauri-driver and a normal protoc command are
unavailable. Therefore no local native build, app launch or desktop screenshot
was attempted. Script syntax, fixture hash/copy/graph-port checks and complete
desktop Cargo feature inspection pass. Five existing native launcher tests and
the root critical/a11y/traceability gates also pass; these preparation checks do
not exercise the new native application path.

[Hosted run 37480189945, job 112325975969](https://github.com/MrScripty/Pantograph/actions/runs/37480189945/job/112325975969)
executed exact candidate `ecc4194adbcd5a198fb97cf47198427e3002773e`, tree
`53c60a1d27a713d6b2cc3dcd04415859a1c8b953`, with frozen chat `0d7d573f` as
its direct parent. It verified source ancestry and unchanged committed synthetic
fixture bytes, then failed native admission with exit 2. Ubuntu 24.04 runner
image `20260927.320.1` has Xvfb, but lacks WebKitWebDriver, tauri-driver, protoc,
and pkg-config metadata for gtk+-3.0, webkit2gtk-4.1,
javascriptcoregtk-4.1 and libsoup-3.0. Ordinary dependency installation, Cargo
policy/build steps and actual native Tauri execution were skipped. The always-run
evidence upload succeeded. No retry, privileged installation or sandbox change
was attempted.

The [committed evidence](../evidence/native-desktop-cpu/README.md) preserves the
downloaded artifact, job steps/log, local checks, exact source binding and hashes.
Artifact ZIP `11420323376` has SHA256
`d32a972650be65365b7a4c30931f924c7e72cc5a1d5c3d031a69a9d1d9d3fbfc`.
Its proposed graph and expected synthetic vector are preparation inputs; no
workflow was saved/reopened in the app and no actual native output or screenshot
was produced. Qualification is blocked pending a cloud-owned environment with
the native prerequisites available within the authorized permissions. The new
driver remains unqualified beyond syntax and fixture checks. A later docs-only
evidence commit preserves this executed source; the branch path filter avoids
repeating the known blocked run for evidence updates.

Real Pumas discovery, pretrained quality, GPU, post-start cancellation, broad
production loading and full release/desktop acceptance remain unqualified.
The accepted feature's controlled CPU/source qualification is preserved and does
not substitute for this native qualification.

## Authorized setup successor

After the first attempt, the owner confirmed that all agents may install needed
dependencies and explicitly authorized the repository's normal hosted Ubuntu
package-manager setup. The earlier no-install admission policy is superseded
for this isolated cloud runner; the failed run and its artifact remain intact.
The successor reuses `scripts/install-ubuntu-build-dependencies.sh` unchanged,
adds official Ubuntu `webkit2gtk-driver`/`xvfb`, and installs official
`tauri-driver` exactly `2.1.0` with `--locked`, using unchanged Rust `1.92.0`.
The upstream release tag points to
`447fa9f3f993fe77724189e355078b38ce20baea` and declares Rust minimum `1.90`.
Driver versions are independent of the app version in
[Tauri's CI guidance](https://v2.tauri.app/develop/tests/webdriver/ci/).

The complete driver graph is recorded before its compile/install and must have
no ONNX owner. Existing repository no-download checks and the effective desktop
feature audit still precede the application build, with `ORT_SKIP_DOWNLOAD=1`.
Existing contents-read permission, checks, action/toolchain pins and 30-minute
job timeout are preserved. The existing Cargo cache action is reused. No
security/sandbox/credential settings or user desktop are changed. Installation
logs and subsequent actual native evidence or runner failure will be retained
in the new exact-source artifact.

## Actual native successor result

[Run 37482087035, job 112332549762](https://github.com/MrScripty/Pantograph/actions/runs/37482087035/job/112332549762)
executed `df6498274c0c49670765c5a01d8536c416465201`, tree
`1883fe84310d8c11b64029a02506344134393a8a`. Official package installation,
Rust/Node setup, repository no-download checks, complete driver feature audit,
driver installation and native prerequisite admission all passed. Recorded
versions are GTK 3.24.41, WebKit/JavaScriptCore 2.52.6, libsoup 3.4.4 and
tauri-driver 2.1.0. The actual Tauri application built and launched using native
WebKitWebDriver on the cloud-owned Xvfb display with protections unchanged.

Real `save_workflow` and `load_workflow` round trips preserved all three nodes
and both encoded edges. The actual editor displayed Text, Embedding and Vector
ports and produced [native graph screenshot](../evidence/native-desktop-cpu/setup-successor/hosted-artifact/native-configured-graph.png)
and [failure screenshot](../evidence/native-desktop-cpu/setup-successor/hosted-artifact/native-failure.png).
The captured screenshot does not show connection wires despite the two saved
edges. Submission remained disabled for 120 seconds with exactly
“Inference validation is stale for the current graph”; the normal interface
update control was not applied. The test failed at this gate, before public
scheduler submission or actual CPU output. No accepted feature code or gate was
modified. The owner logged `NonCanonicalLayout` for the controlled synthetic
library path; this evidence does not establish the stale-validation root cause.

The [successor evidence](../evidence/native-desktop-cpu/setup-successor/README.md)
preserves the complete 15-member artifact, source binding, feature graphs,
installation/runtime logs, saved/reopened JSON, screenshots and gate reason.
Artifact `11423130630` ZIP SHA256 is
`fb338ae8e3f386ecd96238e959823750bfae88b2c8e8fb84a4ea7b1623523bcf`.
The original failed run `37480189945` remains preserved separately. Native build,
launch, saved/reopened data and node/port display are now evidenced; native
public scheduler execution and CPU output remain blocked and unqualified.
Real Pumas discovery, pretrained quality, GPU and post-start cancellation remain
unqualified. PR61 current-main integration is a separate worktree/task and is
not mixed into this candidate.

## Confirmed reopened-graph defects and repair

The real native gate exposed a production session-load defect. After creating an
edit session, `createSessionStores.loadWorkflowByName` rendered the persisted file
instead of fetching that session's canonical graph. Files without derived data
therefore used the frontend's topology-only `v1` fingerprint; the Rust validation
owner uses `semantic-graph-v2`, including authored data. Refresh requests were
correctly rejected as stale. The targeted reopening test fails on the old source
with the backend revision missing and passes when the loader fetches and applies
the canonical session snapshot. Failed/superseded snapshot loads retain existing
session transition fencing; no validation or submission check is loosened.

Both edge paths were present in the captured native DOM with valid nonzero lengths
and constant Y coordinates, e.g. `M282,153 C289,153 289,153 296,153`. Their SVG
glow filter used the default objectBoundingBox region, whose height is zero for
those paths. The [filter region specification](https://www.w3.org/TR/filter-effects-1/#FilterElement)
explains the clipping behavior. CSS drop-shadow replaces that bounding-box filter
while retaining endpoint gradients and reconnect controls. The native test now
records both aligned edges' geometry and computed paint, and its screenshot will
decide actual wire visibility.

The first verified local result is 11 passing session-store tests, including
mandatory owner revision, snapshot failure, and superseded snapshot cases;
the full frontend suite passes 672 tests. Native painting, refreshed validation,
submission and CPU output still require the fresh hosted run. Both failed attempts
and their original screenshots remain unchanged. PR61 `6d720149` and all frozen
chat/source candidates are preserved separately.

The controlled fixture also used uppercase `Synthetic-BERT-8` as its storage
component. Current Pumas explicitly requires the normalized artifact slug at
that depth, explaining its independent `NonCanonicalLayout` warning. The new
temporary fixture uses lowercase `synthetic-bert-8` consistently in path and
model identity; its display label and every committed weight/reference byte are
unchanged. This fixture correction is separate from the confirmed production
revision-loss and edge-paint defects, and does not qualify real-user discovery.

## First repair's actual native result and desktop renderer correction

[Run 37488222631](https://github.com/MrScripty/Pantograph/actions/runs/37488222631)
executed repair source `0f8f8ed2f324f00f8238cabb3243389a547a7d47`, tree
`eed585640386f2d4ac63a0018f0a1e55125433e5`. Official setup, feature audits and
real native build/launch/save/reopen passed. Its screenshot no longer reports
stale validation; it reports blocking diagnostics. The new edge assertion fails
because the desktop canvas registers a second edge component under `src/`,
which still has the old filter. This was an incomplete desktop repair, not a
stale source/build. The assertion correctly stops before submission or output.

The [third attempt's complete evidence](../evidence/native-desktop-cpu/reopened-graph-repair/README.md)
preserves all 13 artifact members, masked job log, unchanged native screenshot,
source patch, failing old-source regression and local passing checks. The desktop
component now receives the same drop-shadow fix as the reusable package,
preserving its insert-preview/drift overlays and reconnect controls. The native
harness records real owner validation responses unchanged to diagnose the new
blocking gate; it also saves edge geometry before assertions. A fresh native run
must establish painting, successful validation, submission and actual CPU output.

Inspection of the existing resolution owner also confirms that the authored
fixture omitted mandatory `runtime_source_context`; it returns
`missing_runtime_source_context` before resolving an interface. The fixture now
supplies `embedding.text`, `embedding.one-text` and `run_scoped`, using the
controlled embedding test's operation/shape and the existing canonical
cancellation mode. This changes fixture authorship only; no validation rule is
weakened and native owner results still decide acceptance.

## Native wire result and descriptor availability checkpoint

Run `37490528758` at `6eb84e949986799d70c5e7949a470cfb3bf86a52` visibly paints
both native wires and passes their zero-height geometry/drop-shadow assertions.
Submission remains blocked by the old fixture's missing runtime context. The
bounded production resolution check reproduces exactly that diagnostic and
accepts the corrected fixture request, with no resolution diagnostics.

Run `37490714747` at `ef2d0df6b0c78adf8aa884637efec9e8d08b4381`, tree
`9eb99c5b45c81ff9e84592e08e50986b0aeb6038`, passes native setup, build, launch,
save/reopen and edge assertions, then fails the actual Submit gate with
“Inference descriptor is unavailable.” No scheduler submission or CPU output is
qualified. The [complete fourth/fifth attempt evidence](../evidence/native-desktop-cpu/desktop-edge-context/README.md)
preserves both 17-member artifacts, original screenshots, source identities,
masked logs and bounded diagnostics.

The real Pumas owner probe on the same isolated fixture reports a valid HF
directory, selected artifact `main`, embedding task evidence and accepted Candle
backend hints; it does not indicate missing or invalid model setup. Further
descriptor availability diagnosis is required at the native owner boundary.
Tauri's official `invoke` is read-only, so the attempted capture produced empty
arrays. The successor harness instead subscribes to the supported validation
lifecycle event and reads the exact session/revision's current projection through
real IPC. It changes no validation result, runtime registry or submission gate.

## Exact owner diagnostics and CPU readiness repair

Run `37493778399` at `218423c6a314f5cf8ecdbcee1afb7ed20edfd360`, tree
`3eb266c8885626ab60318ea3635178e32755068c`, captures the real active-session
projection. The normal interface Apply/Save completes, and requested/current
revisions both equal `80fbe18c51d2f00f`. The owner returns exactly
`invalid_runtime_constraint` and `invalid_device_constraint`, with no available
runtime/device satisfying Candle/CPU. Typed model/port facts resolve; no public
submission or CPU output occurs. The [sixth attempt's complete artifact](../evidence/native-desktop-cpu/descriptor-availability/README.md)
preserves these responses, lifecycle events and unchanged screenshots.

The initial successor attempted generic native Candle configuration/startup.
That proposal was incorrect: `CandleBackend::start` rejects untyped startup and
requires scheduler-selected package and executable target. Field-builder tests
did not validate that integration. The attempt and its rejection are preserved
below; the generic startup changes are withdrawn in normal branch history.

The descriptor provider independently discarded every owner-advertised device
by publishing an empty device list. The new regression fails on that source with
`[]` instead of `[cpu]`; the repaired bridge preserves and deterministically
orders the validated owner device IDs. It leaves runtime lifecycle status and
missing-evidence gates intact. At `7fcd990b`, nine provider tests and 545 affected-runtime tests pass with
one optional test ignored. Strict all-target
Clippy and formatting pass. An initial broader-suite launch failed to find the
already installed Python shared library; a command-local library path allowed the
same source/binary to run successfully, and the initial failure remains recorded.
Native CPU submission/output still requires the fresh successor; pretrained,
GPU, real-user discovery and full production-loader qualification remain absent.


## Startup-regression failure and typed cold-load successor

[Run 37498074275](https://github.com/MrScripty/Pantograph/actions/runs/37498074275)
executes `7fcd990bf6800defcb4f1eae5e07a2d30cadd799`, tree
`6e1a0b50d139886cd5a8f68d93a89801a25ded1b`. The native binary builds;
startup tests compile/run with eight passes and one failure. The llama.cpp
comparison mistakenly reused canonical `cpu`; its actual CPU selector is
backend-local `none`. Production validation correctly rejected the fixture.
Exit 101 precedes launcher creation. The missing WebDriver launcher is downstream;
there is no native app session, scheduler submission or CPU output in this run.
The [complete seventh-attempt evidence](../evidence/native-desktop-cpu/startup-regression/README.md)
preserves all eleven members, inner test log, masked job log and source identity.

The successor withdraws generic startup and exercises the actual selector
contract with `none`, retaining rejection of canonical `cpu`. Existing selected
embedding execution validates package, target and decision before loading Candle.
Its scheduler evidence contract explicitly accepts `NotLoaded` without an
instance. The missing production integration is cold discovery: only existing
live registry records reached descriptor/dispatch sources, and stopped runtime
status was treated as not installed.

Hosted composition now enrolls a missing compiled Candle CPU owner only when
the gateway advertises available `candle.cpu`/`cpu`. Registration reports
`Stopped`, with no model, instance or lease; existing registrations are untouched.
The descriptor bridge recognizes that specific cold capability while failed,
unhealthy, stopping and missing-owner cases remain unavailable. This does not
load a model or authorize a package. Pumas target checks, dependency readiness,
resource admission, selected execution validation and actual typed loader remain
responsible for execution. Other runtimes and resource accounting are unchanged.
The native test records the real cold registry and uses the normal GUI/public
scheduler to initiate loading. It does not call legacy startup or assert fabricated
readiness. Final native output acceptance remains pending a source-bound run.

The affected runtime suite passes 548 tests with one optional test ignored.
Coverage includes a real Pumas owner over the committed fixture and hosted
composition resolving an available embedding descriptor while Candle remains
stopped, with no model, instance or reservation. Cold and ready descriptor cases,
missing/failed/stopping owner gates, existing failed registrations and the public
compiled-owner fact bridge are covered. These controlled tests qualify cold
discovery integration, not actual native GUI scheduler execution or broad loaders.


## Actual cold-owner native result and submission capture

[Run 37502222511](https://github.com/MrScripty/Pantograph/actions/runs/37502222511)
executes `42155032ef88c009f5fcd7680a3f10ea89c0bbd1`, tree
`39ac261c877d0f66b9e4bda3b49a8ec5611e802f`. Native build, all eight
startup tests, actual app launch, cold-owner assertions, save/reopen and wire
checks pass. The real registry reports Candle stopped, with no instance,
model or lease. Normal interface Apply/Save reaches an enabled Submit control;
the GUI click is recorded at 17:29:18.924 UTC.

The [complete eighth-attempt evidence](../evidence/native-desktop-cpu/cold-typed-discovery/hosted-attempt/README.md)
preserves all fifteen artifact members and source-bound logs. IO Inspector never
appears before the four-minute Mocha timeout. A fixture defect put a five-minute
inner inspector wait inside that deadline, so the session dies before failure
capture. Public execution outcome and native CPU output remain unobserved.
The current successor uses a 30-second inspector/error wait, preserves the global
deadline, records the exact real pre-submit projection, and captures GUI errors
and scoped run state on failure. It changes no production selection or gate.

## Missing proof and existing typed producer

[Run 37505022667](https://github.com/MrScripty/Pantograph/actions/runs/37505022667)
executes `7d89d84892d9265185c53a8bfb8f8fb27b6485c5`, tree
`6768d46f912af7c5cd2ef0afbcb0dc952b40d903`. Native setup/build,
all eight startup tests, cold-owner assertions, save/reopen and wire checks pass.
The actual pre-submit owner projection is current/executable, with matching
revision `aac0d972d0845775`, zero diagnostics and `submit_gate.allowed: true`.
The GUI rejects snapshot publication because the dependency requirements proof
is missing for `infer`. Its scoped run query returns no runs. There is no
scheduler execution or CPU output. The [complete ninth-attempt evidence](../evidence/native-desktop-cpu/cold-typed-discovery/submission-proof/README.md)
preserves all twenty-one artifact members, original screenshots/JSON and masked
job log. Artifact `11431084506` has SHA256
`26ea52a606c8eb0be1dcca82899e43fbad50bde2f221a43bb7c69bb6dc391898`.

The existing requirements-proof producer is the graph-associated typed dependency
Resolve action. The qualification fixture now authors its existing
`dependency-environment` sidecar and association edge: four authored nodes and
three edges, comprising the original three executable pipeline nodes plus a
dependency control. After normal interface Apply/Save, the harness calls that
public Resolve command with the real current graph revision and validation
session before clicking GUI Submit. It neither supplies a proof nor creates a
readiness receipt. Publication and execution guards remain unchanged. This does
not repair automatic submission of a graph without the sidecar; that graph's
missing-proof failure remains documented.

The inference definition overlay previously replaced every schema input with
model payload inputs, hiding the dependency association handle. It now preserves
only that schema-owned control after the payload interface. A regression fails
on the previous source and passes with the repair; other legacy inputs remain
excluded. Actual native visibility of all three saved edges remains required.

A real-Pumas hosted-composition regression first observes publication rejecting
the missing proof, calls the existing typed Resolve action, then publishes the
same scoped snapshot successfully. Candle remains stopped with no model,
instance or reservation. Both targeted tests and strict all-target runtime Clippy
pass. All 673 frontend tests, TypeScript checking, the frontend build, affected
ESLint (with actual Node/browser/WebDriver globals declared for the harness),
critical/a11y gates and formatting pass. These tests do not establish a
native scheduler run, actual CPU output, GPU, pretrained-model or full loader
qualification. The next source-bound hosted run must supply that evidence.

The first verification commands mistakenly used root `npm test`/`npm run check`,
whose aliases also execute Rust tests. Both passed (261 node-engine tests with
one ignored and 168 workflow-node tests), but those invocations lacked the
requested defensive `ORT_SKIP_DOWNLOAD=1`. No denied download was retried or
observed. The previously inspected affected-runtime graph covered their
dependencies; an exact default-scope graph is preserved afterward rather than
represented as a pre-build audit. Explicit frontend scripts were then used with
the defensive setting. The effective graphs contain no prohibited ORT download
features or historical Pumas source.

## Native typed Resolve succeeds; serialized-empty assertion corrected

[Run 37509685511](https://github.com/MrScripty/Pantograph/actions/runs/37509685511)
executes `77b9704920697ff01c9fa5b0bb49dd5b585c885f`, tree
`fae84ac50e846f25928c73b3fc5bb40eba5589ed`. Native build, all eight
startup tests, cold-owner assertions, save/reopen and all three visible wires pass.
The actual current-session Resolve producer returns `request_ready` with matching
graph revision `d0f7a8eb92be581d` and validation session. The harness then fails
because it expects an explicit empty diagnostics array. The Rust contract uses
`skip_serializing_if = "Vec::is_empty"`, so its valid response omits that field.
Submit is never clicked; no scheduler run or CPU output is established.

The [complete tenth-attempt evidence](../evidence/native-desktop-cpu/cold-typed-discovery/typed-proof-successor/hosted-attempt/README.md)
preserves all twenty-one members and source-bound logs. Artifact `11434358121`
has SHA256 `ebf7a5ee7f1eaef3d57fc791ff47064e414ae0b387acd8c6a226119897935f8c`.
The assertion successor accepts the contract's omitted-empty serialization and
additionally requires exact graph/session/revision/target/action attribution.
It changes no producer, proof, publication or runtime gate. Syntax and ESLint
checks cover the fixture change; previous runtime/frontend checks remain valid.
An initial ESLint invocation failed because the repository configuration does
not declare harness globals; both that log and the successful explicit-globals
check are retained. The next native run must still establish scheduler execution
and actual retained synthetic CPU output.

## Snapshot publication clears; scheduler rejects the control node

[Run 37511768640](https://github.com/MrScripty/Pantograph/actions/runs/37511768640)
executes `62e3371b0c947fbf088853f9128404a547c36990`, tree
`b77ff948c9a5d7ed5990ca5d9677ea2f13d0be56`. Actual typed Resolve
succeeds for the current revision/session, and GUI Submit passes publication.
The owner records scoped queued run `run_e8ac33d3-60c0-45c9-aba8-8355830157bd`.
The scheduler then rejects `deps` from `Invalid`. This run has no selected
runtime/device, no start/completion timestamp and zero retained output artifacts.
CPU execution/output are not established; this is not evidence of loader failure.
The [complete eleventh-attempt evidence](../evidence/native-desktop-cpu/cold-typed-discovery/typed-proof-successor/scheduler-control/README.md)
preserves all twenty-two members, original images/JSON and masked job log.
Artifact `11435774716` SHA256 is
`d8162747417705965ab6a666bbef3bedcb95ef353d72dc955d0192a503b772d4`.

The task projection already excludes dependency association edges from runtime
input bindings, but still emits the dependency control as an unsupported task.
Its descriptor explicitly assigns dependency actions to workflow-service and
retires embedded node execution. The repair omits only that canonical control
type from scheduler tasks. It preserves the authored graph, execution fingerprint,
association, actual Resolve producer and every proof/readiness/dispatch guard.
Ordinary unsupported nodes continue to project and fail normally. The old-source
regression reproduces the missing omission; separate classification coverage
checks that the repair preserves unsupported and materialization tasks.

All 926 workflow-service tests pass with the repair, together with strict
all-target selected-crate Clippy, formatting and critical/a11y gates. Default
dependency compilation retains an existing inference dead-code warning; no
unrelated warning repair is included. Exact complete all-target Cargo graphs
are inspected before both test and Clippy builds, using current Pumas and
dynamic ORT with `ORT_SKIP_DOWNLOAD=1`. The isolated qualification workflow's
path filter now includes this exact projection source so its repair triggers
the same native route; permissions, checks and deadlines remain unchanged.
Native CPU execution/output still requires the subsequent source-bound run.

## Authored text is missing from desktop request ingress

[Run 37514419876](https://github.com/MrScripty/Pantograph/actions/runs/37514419876)
executes `c3ef607630cfead9bfb38adac955bd0de95bf93d`, tree
`cc8aee7ef4bf2ecb91f6f9da8aa0c7d9ef83ac95`. Typed Resolve and GUI Submit
clear snapshot publication. The previous invalid-control-node error is absent;
the scheduler now reports no ready task while the graph is incomplete. Scoped
run `run_872043f9-43b1-48cf-9e2b-204a361ab8c7` remains queued, with no
selected runtime/device, no start/completion timestamps and zero retained outputs.
CPU execution/output remain unestablished. The [complete twelfth-attempt evidence](../evidence/native-desktop-cpu/cold-typed-discovery/typed-proof-successor/source-input/README.md)
preserves all twenty-two members and masked job log. Artifact `11435994583`
SHA256 is `259ffd7be87a3c41c7e0c3728ee14c2630fa98ecf30c76696edd4ee5c276c4d4`.

The exact desktop producer always submits `inputs: []`. Source-input tasks require
request-level port bindings; the saved text therefore never reaches scheduler
materialization. The successor forwards only explicitly authored root text
strings through the existing `WorkflowPortBinding` ingress contract. It preserves
blank/whitespace strings, skips unset/non-string fields and does not override a
connected text input or forward arbitrary inference data. Numeric, boolean,
selection and other input handling remain outside this narrow text-input repair.
Backend ingress validation, scheduler materialization and proof/dispatch guards
are unchanged. The two regressions fail with the extracted old empty-input
behavior, then pass with the repair; that reproduction is explicitly not a
pristine-old-HEAD build. All 675 frontend tests, TypeScript checking, the frontend
build and affected lint pass.

The fixture additionally assumed every successful run opens I/O Inspector.
Existing routing explicitly sends non-image runs to Scheduler. The successor
waits for that actual success destination, verifies the owner-retained run/output,
then opens I/O Inspector using normal workbench navigation. Production routing
is unchanged. Actual native scheduler CPU execution/output still requires the
subsequent source-bound run.

## Current result: readiness identity boundary remains blocked

[Run 37517106108](https://github.com/MrScripty/Pantograph/actions/runs/37517106108)
executes `ce6b7bc6e3c88c9c59f66a899a203c1715363ce9`, tree
`a8571236894c4bba3135d309559aa61fffc03bd9`. Native build/startup tests,
cold-owner assertions, save/reopen, three visible wires, typed Resolve and GUI
Submit clear their earlier boundaries. Authored text ingress advances the run
to dependency readiness admission. The actual GUI reports:
`scheduler dependency readiness admission failed: workflow service operation failed`.
Scoped run `run_5dda0631-cb25-4504-89ea-c6913f76e6d5` is queued with
execution-session resume state `dependency_readiness_pending`, no selected
runtime/device, no start/completion timestamp and zero retained outputs.
Actual native CPU execution/output remains unqualified. No loader failure,
GPU, pretrained-model or full production-loader qualification is inferred.

The [complete thirteenth-attempt evidence](../evidence/native-desktop-cpu/cold-typed-discovery/typed-proof-successor/readiness-admission/README.md)
preserves all twenty-two members, original images/JSON and masked job log.
Artifact `11438650692` has SHA256
`8575ede33b077dde517a0c8dfa77aa0883442b443cdd9f383cfdd11a0f89b080`.
The native wrapper obscures its inner workflow-service error. A separate local
diagnostic confirms an existing canonical identity mismatch: graph production
uses `task_type: None` and `platform_context: Some(host OS/arch)`, while readiness
reconstruction uses `task_type: Some(task kind)` and `platform_context: None`.
Both fields participate in the requirements hash. The actual public producer
and readiness lifecycle reject that reconstruction because its requirements ID
does not match the saved validation proof. This is a focused local reproduction
and source comparison, not an observed native inner-error capture.

The diagnostic passes by observing the retained rejection guard. Its exact patch,
complete audited Cargo graph and log are preserved; the temporary diagnostic
test is removed from production source. No readiness receipt or proof is
fabricated. Carrying the producer-owned canonical planning identity through
snapshot publication into readiness admission remains the next unresolved
integration boundary. No further hosted successor is active. The candidate's
926 workflow-service tests and 675 frontend tests remain green, while native
acceptance is explicitly blocked at this later boundary.

## Shared planning owner successor

The preserved `3b9e731ac678b00463d4f1b48ceb78322a096c04` candidate is the
parent of this repair. Its failed native evidence remains unchanged. The
requirements producer hashes both optional `task_type` and `platform_context`;
graph Resolve supplied only the host platform while scheduler reconstruction
supplied only the task type. Equal model, artifact and CPU target therefore
produced different requirements IDs. This is an owner construction mismatch,
not evidence that proof validation should be relaxed.

Workflow-service now owns one inference planning constructor used by both
graph Resolve and scheduler readiness. It includes the explicit task kind and
host OS/architecture in both requests while retaining caller attribution,
model/revision/artifact, runtime/device, selected bindings, overrides and traits.
The generic requirements hash, saved proof equality checks, dependency-control
task exclusion and authored root-text forwarding are unchanged. Historical
proofs made with the previous request shape require fresh validation and Resolve;
no unlike historical identity is accepted as current.

The regression runs the actual graph action request producer, round-trips its
proof-bearing executable snapshot, projects actual scheduler tasks and asks the
readiness lifecycle for its request. It uses the controlled CPU package identity
and target, without manufacturing a requirements ID or readiness receipt. The
unchanged package/target failed before repair with the exact saved-proof mismatch
and passes after repair. Model, model revision, artifact, device and runtime
changes still fail that guard; another graph revision cannot publish the proof.
The dependency sidecar remains outside executable tasks. These local descriptor
fixtures test the identity seam; the native attempt must separately establish
actual Pumas authority, saved/reopened GUI submission and retained CPU output.

The full workflow-service suite passes: 928 unit and 60 integration tests. Strict
selected-crate all-target Clippy passes; existing dependency warnings remain in
the retained logs. Complete Cargo feature graphs were inspected before builds,
with current Pumas `26a84e323cae566a46a8f76bef48fa1010aed48b`, ORT `load-dynamic`,
ORT-sys `disable-linking`, no binary download capabilities, and defensive
`ORT_SKIP_DOWNLOAD=1`. Native qualification remains pending for this successor;
no CPU output, GPU, pretrained model or full production-loader acceptance is
inferred from these local checks.

The established embedded default-feature suite passes all 537 tests, including
real-Pumas cold Candle descriptor and typed-proof checks. A Candle-only embedded
test attempt cannot compile existing tests that call the llama.cpp-gated
`InferenceGateway::new`; its failure is retained without changing unrelated test
constructors. This does not establish native CPU execution. The
[local shared-owner evidence](../evidence/native-desktop-cpu/cold-typed-discovery/shared-planning-identity/README.md)
contains the before-repair regression, complete effective graphs, test/Clippy logs
and gate checks. The native workflow path filter includes the actual repaired
owner sources so publishing this isolated successor triggers qualification;
permissions and deadlines are unchanged.

## Current result: identity repaired; native dependency readiness remains pending

[Run 37522366379](https://github.com/MrScripty/Pantograph/actions/runs/37522366379)
executes `5a629f7affd43fd2884899d3a2ed3feb5c3d42ff`, tree
`70651013a1c8c5c50447a860eec6035536330f51`, parent preserved `3b9e731a`.
Actual native build and all eight startup tests pass. Cold-owner, save/reopen,
three visible wires, executable current validation, typed Resolve and GUI Submit
are exercised. The saved-proof identity rejection is cleared. The actual GUI now
reports `Runtime not ready: runtime dependency readiness is pending for scheduler
task(s): infer`. Scoped run `run_370d45c5-335d-4858-9817-4dac3e9f331a` is queued
with resume state `dependency_readiness_pending`, no selected runtime/device,
no start/completion timestamp and zero retained outputs. Actual CPU execution
and output remain unqualified; no loader failure, GPU, pretrained model or full
production-loader qualification is inferred.

The [complete fourteenth-attempt evidence](../evidence/native-desktop-cpu/cold-typed-discovery/shared-planning-identity/native-attempt/README.md)
preserves all twenty-two members, original JSON/PNG and complete masked job log.
Artifact `11440194325` has SHA256
`d6c0723c0271deffefc83cd4aa9fb04a19410f9694806b69732ff2d76c4ae22d`.
The GUI test fails on the initial pending response after 9.3 seconds; it does not
capture the provider seed/snapshot result or observe readiness after the default
60-second producer poll. Source review identifies the next cold bootstrap seam:
hosted Resolve reads snapshots, but scheduler dispatch needs a valid requirements
payload before enqueueing snapshot work. The provider starts empty and its
missing-snapshot result carries no requirements/bindings. Hosted inventory also
defaults to Python probing, with other provider kinds not implemented in this
build scope. These are source findings, not native provider-result diagnostics;
exact pending cause and eventual readiness are still unqualified. No ready
receipt, requirements payload or snapshot is fabricated.

The same executor remains accessible following the disconnect callback. Source,
local tests and completed CI evidence are preserved without restarting or
duplicating work. No successor run is active. Main, PR 62 and protected worktrees
remain untouched. The shared identity repair is complete; the distinct native
readiness bootstrap/provider boundary remains blocked.

## Provider/bootstrap diagnostic successor

Continue from preserved `e6ec81121344901ba0e9f8a00ca2cb136276f08b` with a
bounded diagnostic capture before proposing another repair. Actual-owner warning
records include the graph provider DTO, scheduler requirements-seed result and
registry rejection, the queued/deferred task transition and producer registry
lookup failure. No result, proof, snapshot or execution policy changes. The GUI
harness retains its original failure and reads the same run for 70 seconds through
public scheduler/run/inspection queries, covering the normal producer poll.

A correction to the preceding source hypothesis: the deferral helper does enqueue
work after rejected requirements seeding. Actual diagnostics must distinguish
payload seed rejection from the subsequent registry lookup; queue creation is
not itself absent. Previous failure records remain unchanged as historical evidence.

Local verification passes: 928 workflow-service unit + 60 integration tests,
strict selected-crate all-target Clippy, 537 embedded default-feature tests,
affected harness ESLint, formatting and critical gate. Effective Cargo graphs
are inspected before builds with dynamic ORT, no download capabilities and
`ORT_SKIP_DOWNLOAD=1`. The [diagnostic candidate evidence](../evidence/native-desktop-cpu/cold-typed-discovery/bootstrap-diagnostics/README.md)
preserves those checks. No new readiness repair or native CPU qualification is
claimed before actual provider/bootstrap results are captured.

## Actual first blocking transition captured

[Run 37526479578](https://github.com/MrScripty/Pantograph/actions/runs/37526479578)
executes `7ec336032a4e9910a26f65ef2e34d930a786d0bb`, tree
`adc9932f9215cbf5d6b9b7a81f8e3b850b8f250b`. App build and all eight native
startup tests pass. The first actual blocked transition is graph Resolve's
provider result: `missing` / `unavailable`, failure `requirements_unavailable`,
with “No fresh dependency readiness snapshot matches the request.” It contains
no requirements/bindings, although request derivation returns `request_ready`.
Scheduler retains the same requirements ID, descriptor, graph revision,
validation session and saved snapshot `wfvalsnap_211237cd-4b7d-4488-92cd-55fb36f94665`.
The seed guard rejects the actual Missing result because only Resolved/Ready
results may seed payloads. Task `infer` becomes `paused_deferred`, state version
3, and queues one probe. The normal producer poll then reports `MissingPayload`
for the same requirements ID. Actual automatic retries reach deferred versions
5/7/9/11 with the same snapshot and identity; three read-only samples over 70
seconds retain queued `run_845edcb4-8b01-4cbf-b14b-6438fb4922da` with
`dependency_readiness_pending` and zero output artifacts.

The [complete fifteenth-attempt evidence](../evidence/native-desktop-cpu/cold-typed-discovery/bootstrap-diagnostics/native-attempt/README.md)
preserves all 23 members, complete masked job log and 12 extracted actual-owner
diagnostic records. Artifact `11443605285` SHA256 is
`50163fcaf05389a2e1a55bb459988150a9587902631eaefa35eea15cd677cf89`.
This establishes the cold requirements seed/snapshot circular dependency and
corrects the earlier source-only queue hypothesis. CPU execution/output remains
unqualified. No loader, GPU, pretrained model or broad production qualification
is inferred. No repair or synthesized readiness is present in this run.

## Authoritative cold requirements bootstrap repair

Ancestry inspection confirms clean `69e6b4163642349436373781cce730010d1536f6`,
tree `c9384336fb4d5dd6f1bfa07e9e4d9df5b53339be`, contains only diagnostic
instrumentation and preserved evidence after `e6ec8112`; no readiness repair.
Supported Git fetch returns the same candidate. Continue from that preserved
head without reset or history rewriting.

The async embedded producer now receives the existing hosted Pumas access.
On a genuine missing registry payload, it checks the canonical saved requirements
identity, current host platform and actual selected package facts, then queries
the current Pumas owner's dependency requirements API. A valid resolved empty
declared-binding set produces a scoped Resolved snapshot for the normal seed
consumer and an inventory Check snapshot. The registry accepts that complete
empty validated set while retaining non-ready/invalid and partial-set rejection.
Stale/mismatched entries, mismatched proofs, unavailable selected facts and
unresolved declared bindings stay rejected. No registry insertion, fake ready
receipt, altered qualification model metadata or alternate executor bypasses
the existing consumer/admission guards.

The [repair evidence](../evidence/native-desktop-cpu/cold-typed-discovery/pumas-bootstrap-repair/README.md)
includes the real Pumas -> async producer -> saved-proof-scoped seed consumer
regression and relevant negatives. Local checks pass: 21 environment tests,
988 workflow-service tests, 537 embedded default-feature tests, strict all-target
selected-crate Clippy for those three crates, harness ESLint, formatting and
critical gate. Complete feature graphs are audited before each build and
ORT_SKIP_DOWNLOAD=1 is set. Two focused setup failures are preserved: unwritable
default XDG configuration and an invalid negative dependency profile fixture;
their corrected final test and full suites pass. Production fixture metadata
is unchanged.

This first bootstrap supports the current Pumas owner's resolved absence of
indexed declared bindings. Declared profiles/selected bindings/overrides/trait
intents and cold local-client resolution remain unavailable. Runtime capability,
device/resource and loader decisions remain with their existing owners. Native
acceptance is still pending one actual saved/reopened GUI submission, automatic
resume of that same scoped run, and retained CPU output. Pending alone cannot
pass; no GPU, pretrained or broad production-loader claim is made.

## Native authoritative resolution captured; output still blocked

[Run 37530472028](https://github.com/MrScripty/Pantograph/actions/runs/37530472028)
executes `fffe5342a29f903136186f79de67c99cd32d7711`, tree
`5adc052e54363307373dc43e915cc21055ea1c8a`. App build and eight startup
tests pass. At 21:10:44.529Z/537Z the actual Pumas owner returns valid resolved
empty bindings for the same candle/linux-x86_64 model scope. The authoritative
resolution route is now exercised; post-resolution seed/admission is not yet
captured. The same queued run remains dependency_readiness_pending with zero
outputs in 61 observations over 120 seconds.

The harness also has a concrete response-shape error: the inspection DTO does
not contain io_artifacts. Rows require the public workflow_io_artifact_query.
Its final failure is Cannot read properties of undefined (reading 'some').
That test error is preserved and does not explain or erase zero actual output.
Correct the harness and capture the post-resolution seed/admission transition
before proposing another production repair. The [complete sixteenth-attempt
evidence](../evidence/native-desktop-cpu/cold-typed-discovery/pumas-bootstrap-repair/native-attempt/README.md)
preserves all 23 members, full masked job log and 13 actual records. Artifact
11445275701 SHA256 is
235b9d8ad91030c375eccf1b6124f9a031907664be2cef379148a44f07e53d3a.
Native CPU output, GPU, pretrained models and broad loader qualification remain
unqualified.

## Post-resolution boundary capture successor

Preserve the sixteenth-attempt evidence at `e4d79b98` before further edits. A
correction to the preceding response-shape description: the inspection DTO
does define io_artifacts, but serialization omits the field for an empty list.
The harness incorrectly assumed its presence. The successor uses the canonical
workflow_io_artifact_query response's explicit artifacts array; empty output
still fails qualification. The artifact body/read and producer-port contracts
are inspected against their current owners.

Additional actual-owner diagnostic records capture progress-loop entry/exit,
requirements seed storage, readiness proof resolution, the admitted task state,
and runtime dispatch selection entry/preparation. They retain existing outcomes
and proof validation, without another production repair or synthetic success.
The preceding native Pumas resolution does not establish those later transitions.
This first bootstrap also requires an explicit runtime and the current host
platform; automatic-runtime cold bootstrap remains unsupported.

## Actual post-seed blocker: readiness requests Resolve instead of Check

[Run 37533284903](https://github.com/MrScripty/Pantograph/actions/runs/37533284903)
executes 2c750701c3e1e073c1bf07280ec523269c97be03, tree
41ce4b9fe4de7550e35f0c7809009d9c51b3824c. Build and eight startup tests
pass. The normal requirements consumer stores the actual Resolved seed at
21:35:43.306Z. The next readiness proof is Resolved at 21:35:43.308Z, and
infer is paused_deferred version 13 at 21:35:43.309Z, reaching version 19 on
retries. No task becomes ready and no dispatch selection begins. Source tracing
confirms the readiness adapter requests Resolve again; inventory produced a
separate Check snapshot. This is the exact next shared-boundary defect.

The corrected artifact query returns an explicit empty array in all 61
observations over 120 seconds. The same run, execution session and saved proof
remain scoped and queued with zero outputs. The [complete seventeenth-attempt
evidence](../evidence/native-desktop-cpu/cold-typed-discovery/pumas-bootstrap-repair/post-resolution/native-attempt/README.md)
preserves all 23 members, full masked job log and 52 actual-owner records.
Artifact 11446316351 SHA256 is
b8970532236ac1460a1405a9dd604518fd0a2ee271dd38d09a666a77e77b53ea.
Preserve this failure before the bounded action correction: Resolve for seed,
Check for readiness, retaining missing/non-ready/mismatched result rejection.
CPU output and all broader loader/GPU/pretrained claims remain unqualified.
