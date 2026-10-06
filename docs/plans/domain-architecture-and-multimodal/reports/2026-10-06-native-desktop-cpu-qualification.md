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
desktop Cargo feature inspection pass. The hosted job is the decisive check of
whether normal repository cloud permissions provide the required native setup;
its artifacts must distinguish admission failure from native runtime/visual
evidence.

Real Pumas discovery, pretrained quality, GPU, post-start cancellation, broad
production loading and full release/desktop acceptance remain unqualified.
The accepted feature's controlled CPU/source qualification is preserved and does
not substitute for this native qualification.
