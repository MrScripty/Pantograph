# Frozen CPU embedding graph evidence

This evidence-only branch publishes the unchanged bounded executor archive and
its 22 plain-text members for independent read-only review. Implementation stays
frozen at `2961b1480c103ec242c6359a45ceee6bb5d999f1`, tree
`fff2f10bb15d396c698e88ce01de283f6ad4b3b6`, parent
`c75fa2379a730833709ea8976075bf17a117040f`. No source changes, builds, dependency
installations, external weights or private/unrelated artifacts are included.

`source-binding.json` records the archive digest and source patch binding.
`source.patch` is byte-for-byte equal to the parent-to-frozen-source Git diff for
its 24 listed product/test paths. Its SHA-256 is
`95966596715675253301a1cdfd6770196931c6132a0a8aa090fed5a6bb14c20f`.
The unchanged `workflow-cpu-embedding-graphs-evidence.tar.gz` SHA-256 is
`e95e2b79e438139e2a6361a125b70551ac959f5994980b3a25e3d86930cd4c24`.
All 21 hashes in the original `manifest.sha256.json` were verified before
publication; `SHA256SUMS` covers all published files except itself.

Deciding logs are `public-cpu-graph.log`, `rust-tests-final.log`,
`clippy-final.log`, `frontend.log` and `features.txt`. `test-counts.json` records
2,465 Rust test cases plus one doctest; six existing optional cases remain
ignored. Initial Rust fixture failures in `rust-tests.log` are retained and
explained by the implementation report; they were corrected without weakening
production validation. Source-only review is in `independent-review.txt`.
Earlier traceability/staging identities are historical pre-commit observations,
not claims about this evidence-only tree.

See the [implementation report](../../reports/2026-10-06-workflow-cpu-embedding-graphs.md)
for reproduction commands, source scope and controls. Public saved/reopened
graph tests run actual Candle CPU inference with synthetic, untrained fixtures
and controlled descriptor/Pumas/readiness/dispatch facts. They do not qualify
production loading, live Pumas, pretrained quality, GPU, native batch throughput
or desktop execution. The effective Cargo graph uses current Pumas and dynamic
ORT/disabled linking without `download-binaries`; builds also used
`ORT_SKIP_DOWNLOAD=1`. This publication adds no new qualification claim.

Verify the published files from this directory:

```bash
sha256sum -c SHA256SUMS
```
