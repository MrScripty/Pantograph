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

The new branch-specific hosted workflow has only `contents: read` and a bounded
30-minute timeout. Existing workflow permissions, checks and timeouts are
preserved. It verifies committed fixture hashes, records the proposed graph and
performs read-only native prerequisite admission **before any install or build**.
It never invokes sudo, apt installation, sandbox disabling, privileged displays
or access-denial workarounds. The expected normal Linux prerequisites include
GTK/WebKit development libraries, WebKitWebDriver, Xvfb and tauri-driver. Official
[Tauri CI instructions](https://v2.tauri.app/develop/tests/webdriver/ci/) use sudo
to install system packages; that installation route conflicts with this task's
no-privilege constraint and is deliberately not invoked.

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

## Current result

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
