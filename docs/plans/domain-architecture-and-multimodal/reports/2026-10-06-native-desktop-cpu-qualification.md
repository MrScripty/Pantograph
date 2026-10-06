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
