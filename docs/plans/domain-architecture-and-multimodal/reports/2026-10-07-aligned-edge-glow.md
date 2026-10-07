# Aligned graph edges: painted glow correction

This scoped report records a local rendering correction over combined production
`85536d3982fc1fe3f6b1c1a90da30793b7537b04` (tree
`5dd020e1becc791d99b8bcae2ae9c3c83e9e193a`). Current main was freshly fetched as
`038dacaaa98ebd007c32e13d4608726ca5ccf63a`. PR64, PR65 and PR67 retain their
separate source scopes. This report does not mark native execution accepted.

## Actual failure and contract

Native run `37647955544`, attempt 1, on `2ba8f140370dda896819bb067edc4c51e5311d8f`
failed at `native-desktop-cpu.e2e.mjs:95`. Its original DOM has all three saved
paths and the restored dependency input, but both horizontal paths have a
zero-height bounding box and reference the default object-bounding-box SVG glow
filter. The original screenshot paints the angled dependency connector while
omitting the horizontal strokes. The original archive, all 18 members and full
masked job log remain intact.

Visible saved connectors are the rendering requirement. The qualifier's exact
`drop-shadow` assertion is an implementation choice: an appropriately bounded
user-space filter could also meet that requirement. We do not change the
assertion merely to pass it. Independent painted-output tests reproduce a real
production failure, so the correction is justified separately from that choice.

The [SVG bounding-box contract](https://www.w3.org/TR/SVG11/coords.html#ObjectBoundingBoxUnits)
excludes degenerate object geometry even when its stroke has painted extent.
The [filter region contract](https://www.w3.org/TR/filter-effects-1/#FilterEffectsRegion)
uses object bounding boxes by default and clips filter output to that region.
[CSS shorthand filter regions](https://www.w3.org/TR/filter-effects-1/#FilterRegionForShorthands)
cover the painted and expanded content instead.

## Small production delta

Both reachable `ReconnectableEdge.svelte` owners replace the SVG glow definition
and reference with `filter: drop-shadow(0 0 2px white)`. The desktop registry and
app subgroup editor use the app component; package defaults and its subgroup
editor use the package component. Gradients, selected widths, drift/insert
overlays, hit paths and reconnect anchors retain their existing structure.
There is no graph data, runtime, persisted schema or native assertion change.
White glow preserves the glowing-edge intent; its appearance is not promised
pixel-identical to the old doubled source-colored blur.

## Executed local evidence

The controlled Chromium harness mounts the actual materializer, BaseNode,
SvelteFlow and each production edge owner. It spreads only its browser fixture
positions to keep midpoint probes unobscured. The native fixture is unchanged.
For selected and unselected states, a screenshot is compared to the same scene
with the painted paths hidden; the assertion checks pixel output and does not
require a particular filter implementation.

Both baseline owners fail: each horizontal edge contributes zero changed pixels;
the angled control contributes 39/42. After correction, all six probes per owner
pass: horizontal edges contribute 35 pixels in both states; angled probes
contribute 33/37. Selected widths remain 1.5px versus 1px; gradients retain their
source/white/white/target stops. Saved graph load, definition/runtime refresh and
JSON replay retain four nodes and all three stored/rendered connector IDs.
The browser is existing CPU Chromium with `--no-sandbox --disable-gpu`; this is
controlled component rendering, not native WebKit, security or model evidence.

All 675 frontend tests, typecheck, production build, full ESLint, critical and
accessibility gates pass. Full 4481-line native all-target normal/build/dev Cargo
features match the prior composition after exact checkout path substitutions:
dynamic ORT, disable-linking and Pumas26a84, with download features absent.
`ORT_SKIP_DOWNLOAD=1`, hub/transformer offline guards remain set. The separate
default desktop graph was also inspected. No Rust source or Cargo pin changed.

Evidence lives in `/workspace/qualification-evidence/aligned-edge-glow-native`.
Prior publication evidence's 78 checksum-listed files were verified unchanged.
Independent read-only diagnosis accepts the production defect and narrow delta;
final exact-source review and one existing native qualification are the next
gates. Admission, execution, artifact Read and inspector results must be reported
individually from that new attempt. No GPU, real-user discovery, pretrained or
full production loader qualification follows from these local checks.
