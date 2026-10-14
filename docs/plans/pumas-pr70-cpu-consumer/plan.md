# Pumas PR70 CPU consumer compatibility

Status: `Accepted` for the bounded local CPU consumer edge.
Acceptance: `satisfied`. Current phase: candidate handoff. Next slice: `none`.
Parent owns successor adoption, hosted candidate gates and any release action.

Baseline Pantograph source: `0dc45263d014bb9b42884242547424070ad65cc1`.
Qualified consumer source: `039cee079d273a8d4968df56a4b35e8eb8aacbbe`.
Candidate Pumas: `df53db08e2ba224edfaf713522ddd410f7d4ac44`, PR70; its
receipt identifies production source equivalent to main `ef9eead9`. The current
Pantograph baseline pin `aa1d79ae2d53f0acb088bc85223ecc2ff46af12b` remains
untouched in PR78. Chrema stays `508c8b8cb724b49c655dab8bc4c8336190e516c7`;
Candle stays `88ed7911de9e88196b1f55b199145d22647f415e`.

## Objective and M1 write set

Establish whether the exact Pumas candidate satisfies this actual consumer edge:
locked Rust compilation, Pumas model/descriptor/facts/load target, public
Pantograph CPU scheduler execution and fresh-process retained-state inspection,
composed through public Chrema. Untrained fixture results do not qualify PR75's
pretrained text T3. No release tags or upstream source writes.

M1 (`Accepted`) changes only root `Cargo.toml`, `Cargo.lock`,
`.github/workflows/quality-gates.yml`,
`examples/pumas-chrema-cpu-demo/compatibility-lock.json`, and this plan directory.
Manifest, resolved source, CI checkout and demo lock move together only on this
candidate branch. API breaks trigger a bounded upstream/consumer proposal to
parent before any source repair outside this write set. No transitive updates
unless resolution demonstrates they are required by the new Pumas declaration.

Acceptance: exact resolution/no-build-download contract; actual native
descriptor/package/load-target API; 19 reviewed integration contracts;
fresh separate-process read; relevant owner tests. Record failed
API/build/runtime attempts as failures. A successful receipt identifies all
consumed commits, binary/source hashes and observed gates, not release approval.
All selected gates passed: 19 integration contracts, 598 embedded-runtime tests
with2 existing ignores, 4 HTTP host contracts, example Clippy/format/ESLint and
pin/no-build-download metadata checks. Independent GPT-6.1 Sol High review
accepted exact039cee07 with no functional findings. [Receipt and raw evidence](reports/consumer-compatibility.md).

## Composition and isolation

Composed-design applicability: `not-applicable` for new architecture; this slice
changes the existing dependency edge only. It inherits the baseline composition
with identical owners/contract boundaries and reruns real consumer observations.
Replan if candidate changes those ownership or persisted/runtime semantics.

Task branch `qualification/pumas-pr70-cpu-consumer-2026-10-14`, worktree
`/workspace/Pantograph-pumas-pr70`, integration owner parent; never merge or
retarget PR78 implicitly. Retain candidate ref for review/handoff. Remove the
task-created clean worktree after exact committed head is reachable from that
retained ref and evidence records the disposition. No other worktree cleanup.

No observed API/build failure on this selected edge. Parent/Pumas owner owns
candidate promotion and APIs. Historical library migration, pretrained T3,
other targets and hosted candidate CI remain outside local acceptance. Read
baseline repository guidance and current public Coding-Standards; available
AGENTS.md/.agents skills remain absent. [Ledger](execution-ledger.md) and
[issues](issues.md) own dated findings.
