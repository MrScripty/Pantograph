# Canonical reopening repair: native desktop component gap

[Run 37488222631](https://github.com/MrScripty/Pantograph/actions/runs/37488222631)
executed `0f8f8ed2f324f00f8238cabb3243389a547a7d47`, tree
`eed585640386f2d4ac63a0018f0a1e55125433e5`. All official setup and no-download
checks passed; real Tauri saved/reopened three nodes and both encoded edges.

The screenshot now reports **Inference validation has blocking diagnostics**,
rather than stale revision. The native test fails before submission because its
computed edge filter still uses the old bounding-box SVG region. The desktop
canvas registers a separate `src/components/edges/ReconnectableEdge.svelte`;
the package renderer repaired by this source is not that component. This is a
missing desktop repair, not a stale checkout/build. Native wires, successful
validation, submission and CPU output remain unqualified by this attempt.

The complete 13-member hosted artifact and masked job log are preserved. Text,
HTML and logs use lossless timestamp-free gzip; JSON and PNG bytes are unchanged.
Artifact `11424721900` ZIP SHA256 is
`e2d90ae07b9c0fc6fcfddcdcef3a24eec5c1463519768f4f44612f4c9b6011ab`.
Local evidence preserves the failing old-source revision regression, 11 passing
session tests, 672 passing frontend tests, typecheck/build/lint/gates, complete
feature graph, source identity and source patch. `SHA256SUMS` binds this folder.
Both earlier attempts and screenshots remain unchanged. No gate was relaxed.
