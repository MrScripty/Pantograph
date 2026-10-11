# GitHub Pages presentation source

This is a static presentation with a workbench gallery, real scheduler recordings,
inference ownership boundaries, and the repository's five research manuscripts.
It does not execute Rust, load models, or simulate live scheduler state.

## Build and preview

```bash
npm ci
node scripts/build-presentation.mjs
python3 -m http.server 8080 --bind 127.0.0.1 --directory dist/presentation
```

Open `http://127.0.0.1:8080`. The build uses the existing CommonMark dependency,
verifies the manuscript hashes against the research source manifest, renders
six reading pages (five manuscripts plus the research index), and copies real
screenshots, recording JSON, original Markdown and research figures. Generated
output remains under ignored `dist/`; original research editions are unchanged.

The landing page supports keyboard-accessible perspective tabs, success/failure
recording selection, inspection of original diagnostic event payloads, raw JSON
downloads, and manuscript contents navigation. All assets are local and work
under a project subpath such as `/Pantograph/`.

## Publication without an application merge

The approved static presentation can be published independently of PR #71 and
application CI. Build it, copy only the generated public files to the root of a
separate `gh-pages` branch, omit this preparation README, and retain `.nojekyll`.
No application source, `.github` workflows, private files or local QA logs belong
in that branch. The public recorded-demo JSON and research sources remain part
of the approved presentation.

In repository **Settings → Pages**, select **Deploy from a branch**, branch
**gh-pages**, folder **/(root)**, then **Save**. This requires no `main` merge and
no custom workflow. The REST equivalent is `POST /repos/MrScripty/Pantograph/pages`
with `{"build_type":"legacy","source":{"branch":"gh-pages","path":"/"}}`.
The credential needs Pages write and Administration write. Do not grant new
permissions or bypass a denial without specific authorization.

Both repositories reported Pages disabled before this publication work. The
expected project URL is `https://mrscripty.github.io/Pantograph/`; it is not a
verified live URL until the Pages deployment succeeds and the public site is
checked. Demo links continue to identify the PR #71 branch while review is
pending. Its application code and shared workflows are unchanged by publication.

## Publication validation

Local Chromium checks cover 1440, 768, 390 and 320 px, resource loads, keyboard
tabs, both recordings, event inspection, raw JSON links and no-JavaScript reading.
All six research reading pages are checked at 390 and 1440 px, including contents
navigation and horizontal overflow. Every local file link and HTML fragment is
checked, including the inference book's bibliography anchors.

The renderer preserves only paired empty `ref-NNN` anchors while retaining
CommonMark's safe handling of other raw HTML. The original manuscript hashes
remain unchanged. Reader text wraps long source identifiers on narrow screens;
contents links use a vertical layout. Inline SVG avoids missing external-arrow
glyphs in Chromium. These changes concern the static presentation only.
