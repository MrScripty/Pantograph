# Accepted all-source candidate qualification

Separate branch: `qualification/all-source-accepted-candidate`.
Qualified composition commit: `5eb2f087e36d1dec1377f78bed5b7afd067c282e`.
Qualified source tree: `2826057b536babcca9aeae7260fd25d46b50a9a3`.

## Exact admitted lineage and intersections

The composition has exactly these two parents, without rewriting either lineage:

| Accepted source | Exact head | Exact tree |
| --- | --- | --- |
| Frontend composition and view-record validation | `fd1ace640ddb51ef906991a6d658707d5ef034f5` | `fbf1a0d15e345024b5c3403d9c55a5aba09059b2` |
| Native fixture/scanner integration | `41701d68fb3748f0b422b247427e4ca6d51ccbd5` | `879b59cbd7ff61e7c88c9542f577724c93fb3fdb` |

Both supplied remote heads and trees were verified before composition. Their
single merge base is `ca9edde6850cd58ade0b7e534bb4f58e704c2c64`. Relative to it,
the frontend lineage changes 24 tracked paths and the native/scanner lineage
changes 10. Their path intersection is empty. The resulting 34-path diff is
exactly their union, and every changed blob equals its accepted parent blob.
No conflict resolution or additional source repair was necessary.

All accepted ancestry is retained: the four undo/view/device/graph-mount repairs,
frontend checkpoint bb45, view-record fd1ace, native fixture/report 827b1b and
scanner 9996dd. The native integration's exact parents are 827b1b and 9996dd.
The new composition changes ten paths relative to fd1ace, and 24 relative to
41701d68. The final evidence successor adds only this Markdown report.

The scanner blob is exactly `5590b4aeab50a859b8bc7e7678426d746f033be4`, equal to
both 41701d68 and accepted 9996dd. Its tests, scan wrapper, baseline, workflow
range gate and aggregate inference CI step are preserved. As a semantic
intersection check, all three frontend-changed Svelte components have byte-equal
markup/style after their script relative to ca9. Their changes concern lifecycle
scripts, while the accepted scanner conservatively evaluates rendered name
evidence. Combined compiler, scan and contract evidence pass. The native Python
lock addition remains cfg(test); its accepted source is unchanged.

## Actual combined checks

Environment: published cloud checkout; Node 24.12.0/npm 11.6.2 and Rust 1.92.0,
matching repository pins. No dependencies, credentials, permissions, network or
paid/speed settings were changed.

| Command | Actual result |
| --- | --- |
| `npm run test:frontend` | 659 passed; zero failed/skipped/cancelled |
| `node --test scripts/*.test.mjs` | 63 passed; zero failed/skipped/cancelled |
| `npm run lint:full` | passed |
| `npm run typecheck` | passed |
| `npm run build` | passed; existing stale Browserslist-data notice |
| `cargo fmt --all -- --check` | passed; source formatting only |
| `TRACEABILITY_MODE=range TRACEABILITY_BASE_REF=ca9edde6850cd58ade0b7e534bb4f58e704c2c64 TRACEABILITY_HEAD_REF=5eb2f087e36d1dec1377f78bed5b7afd067c282e npm run lint:no-new` | critical checks, 27 exact scanner tests, accessibility scan and traceability passed; 34 paths, zero mapped impacts |
| Explicit 41701d68-to-composition `npm run traceability` | 24 paths, zero mapped impacts; passed |
| Explicit historical main 4938e405-to-composition `npm run traceability` | 251 paths, one mapped impact; passed with declared legacy-map adoption |
| `git diff --check ca9edde6850cd58ade0b7e534bb4f58e704c2c64 HEAD` | passed |

The final report successor also receives explicit native-integration/common-base/
main-base traceability and whitespace checks; source blobs remain exactly the
qualified composition's. Semantic contract review remains parent-owned.
Detailed local evidence uses `/workspace/pantograph-cache/all-source-*`; these
cache files are not assumed to transfer to a fresh environment. The tracked
results and identities above preserve the meaningful evidence.

## Qualification limits and coordinated handoff

No concrete blocker was found in the selected combined checks. This is one
coherent accepted-source candidate with fresh frontend/tooling/static gates,
not a new native/WebKit or release acceptance claim. The tooling suite includes
controlled GUI-wrapper fixtures; those fixtures do not launch a real desktop,
browser or model. Native IPC, native package/linkage/loading, C#/BEAM hosts,
GUI/WebKit interaction, real browser cold reopen, model loading/inference and
packaged release execution were not performed here. Rust unit/native builds
were not rerun in this milestone; the parent owns refreshed native qualification.
The reported CDN restriction remains; no denied artifact retry or bypass was
attempted. Existing source reports retain their scoped evidence and limits.

This task does not update PR44/47 or request external/bot review. Parent owns
those refs, review coordination and final integration. No new source goal,
schema policy, gate relaxation, redesign or unrelated repair is introduced.
