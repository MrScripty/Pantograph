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

The landing page supports keyboard-accessible perspective tabs, success/expected-rejection
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

GitHub reports successful Pages deployment of static commit
`6630f0777e06ff0411085304131be78352dc5e5a`:

- Deployment: https://github.com/MrScripty/Pantograph/actions/runs/38175628066
- GitHub-reported public URL: https://mrscripty.github.io/Pantograph/
- Source: `gh-pages`, `/(root)`; public; HTTPS enforced.

The publication environment's outbound proxy rejects `mrscripty.github.io`,
including an approved escalated fetch. The web fetch tool also could not access
it. Public HTTP content and assets therefore remain unverified independently of
GitHub's successful deployment result. Open the returned URL in an authorized
browser to finish public desktop/mobile verification. Local Chromium checks
passed against the exact static bundle.

Demo links continue to identify the PR #71 branch while review is pending. Its
application code and shared workflows are unchanged by publication.

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

## Refreshed live-host captures

The gallery images and matching execution JSON are copied byte-for-byte from
approved PR #71 revision `7fc3584cca8bef647b60ebf303c4b5d6b37d98bd`. The
workbench at that revision queries a persistent Rust service over loopback HTTP
and reads retained scalar output artifacts. The Model Library capture shows a
real isolated empty Pumas catalog. At that capture revision Graph editor submission was outside the
demo host, and these captures establish neither native Tauri nor model inference.
The separate Node Lab authoring surface remains unavailable.

Use the pinned linked demo guide for the runnable host; this isolated publication
branch imports presentation assets only and does not import the runtime changes.
Citation-anchor, mobile reader, and inline-icon fixes remain intact.

The capture refresh passed 1,220 combined portfolio/presentation checks. All 14
copied image/recording instances match the approved source bytes. The existing
`gh-pages` deployment workflow completed successfully at the commit above; the
public host remains blocked by this environment’s outbound proxy.

## Source-only presentation clarification

This revision has not been published. Appearance review and clear approval are
required before another Pages update. The three perspectives are labeled Graph
screenshot, Recorded run, and Architecture diagram. The recording controls inspect
saved JSON; the architecture is authored HTML, not inference evidence. The normal
gallery begins with the genuine retained-output I/O Inspector. The earlier Graph
capture and intentional rejection remain visible and explicitly identified.

`IMAGE_AUDIT.md` records the exact visible messages and deployed/source byte
comparison. Content-derived asset query strings are cache hygiene; a public CDN
or browser-cache mismatch cannot be confirmed in this environment. All eight
workbench views have concise coverage labels. New app source `679622e9` adds
saved-workflow submission and a Network metrics adapter, but no new captures have
been delivered; Settings and Node Editor authoring remain unqualified.

The revised source passed 1,262 combined static and browser checks, and all 22
GitHub destinations on the two landing pages returned HTTP 200. The original
manuscript hashes, bibliography anchors, mobile wrapping and SVG icons are intact.
