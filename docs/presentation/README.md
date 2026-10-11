# GitHub Pages presentation source

This is a static presentation of the node editor, real scheduler recordings,
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

## Publication state and next step

At preparation time, GitHub's repository metadata reported `has_pages: false`
and an empty homepage. No existing Pages workflow or book-building configuration
was found. The Pages settings endpoint was unavailable in this environment.
No Pages settings were enabled or changed, and nothing was deployed.

After the branch is reviewed and merged, the repository owner can enable GitHub
Pages with **GitHub Actions** as its source, then add a reviewed deployment
workflow that runs `npm ci` and `node scripts/build-presentation.mjs`, uploads
`dist/presentation` with `actions/upload-pages-artifact`, and deploys it with
`actions/deploy-pages` using the `github-pages` environment. That deployment needs
`pages: write` and `id-token: write`. No auto-deploy workflow is included in this
change, so pushing the branch does not publish a site.

The expected project URL would be `https://mrscripty.github.io/Pantograph/` once
enabled and successfully deployed. It is not currently presented as a live link.
The GitHub source links target `main` for use after merge; the new demo links do
not become available on `main` until then.
