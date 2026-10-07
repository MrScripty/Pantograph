# Native inspector SVG pointer qualification

This QA-only candidate starts from `5ef776316508f29f366bc0317c38104d4eb4be4b`,
tree `26d8209862ce4526e88a8f637e45c65b30cd5842`. The prior source repair and
qualification branches, failed runs and original artifacts are preserved.

## Failure and scope

[Run 37580177402](https://github.com/MrScripty/Pantograph/actions/runs/37580177402),
job `112657794451`, executed that exact source. Artifact `11464911469` has archive
SHA-256 `b1152378d3d6793900d20ece37d08c763faa8cc2b75844ad99e58e874872aa0f`.
All 26 original members and the full masked job log are preserved under
`/workspace/qualification-evidence/native-desktop-cpu/inspector-projection-validation`.

The actual screenshot, DOM and diagnostics now show the selected run header,
four-node captured graph, current projection and artifact card, with no JavaScript
failures or pending snapshot. The card still belongs to `prompt`: selection of
`vectors` never succeeded. The log records a displayed SVG group and a successful
WebdriverIO clickable check, followed by repeated native `element not interactable`
errors for the group click. There is no vector Read or displayed preview evidence.

The selected group has a painted 190-by-104 body rectangle, text and badge children.
`RunGraphSnapshot.svelte` handles bubbling clicks on its group and Enter/Space on
the focusable group. The native artifact records WebKitGTK 2.52.6. Its
[element layout implementation](https://raw.githubusercontent.com/WebKit/WebKit/webkitgtk-2.52.6/Source/WebKit/WebProcess/Automation/WebAutomationSessionProxy.cpp)
requires the clicked element itself in the center-point hit list. Its
[click implementation](https://raw.githubusercontent.com/WebKit/WebKit/webkitgtk-2.52.6/Source/WebDriver/Session.cpp)
rejects layout errors before dispatching a mouse interaction. In contrast, the
installed WebdriverIO clickable check accepts a hit on any descendant. WebKit's
[SVG container hit testing](https://raw.githubusercontent.com/WebKit/WebKit/webkitgtk-2.52.6/Source/WebCore/rendering/svg/RenderSVGContainer.cpp)
targets painted children; direct container targeting requires additional conditions.

This explains an automation locator/actionability mismatch and does not demonstrate
an unclickable application. The failed run did not record client rectangles or hit
lists, so that exact geometry is still an inference from the pinned engine source.
The candidate scopes the same visible vector node, selects its first direct body
rectangle and issues a normal WebDriver element click. No application code,
selection state, handler, IPC transport or synthetic event dispatch is changed.
Read-only client-rectangle and center-hit observations are saved before that click
to confirm the diagnosis in the fresh native run, including real occlusion if any.

## Verification and acceptance

Portable regression tests distinguish a painted descendant from an absent container
hit, retain foreign occlusion and viewport clipping, and avoid invented coordinates
when there are no client rectangles. They exercise diagnostics, not native input
dispatch. Existing scoped output tests remain required. No local WebKit/Xvfb is
installed; Chromium cannot start with its existing sandbox ownership, and its
sandbox configuration has not been changed.

The existing workflow must freshly select the same completed run, display its graph,
click the visible vector body, show the exact retained vector artifact card, use
Read and display all eight finite values equal to the actual retained body. The
scoped completed `candle.cpu` timeline assertion and CPU oracle remain unchanged.
Success requires that native evidence; portable tests alone do not establish display
acceptance. No GPU, pretrained model or full production loader qualification follows.

No Rust changes or local Cargo build are needed. The existing qualification workflow
keeps its full effective feature audits and `ORT_SKIP_DOWNLOAD=1`; model/runtime
binary downloads, main changes, PR creation and merges are outside this candidate.
