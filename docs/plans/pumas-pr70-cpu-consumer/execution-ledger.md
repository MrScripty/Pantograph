# Candidate qualification ledger

2026-10-14: Verified parent-provided PR70 head and fetched its source-foundation
receipt. Created an isolated Pantograph candidate worktree/branch from PR78
34dfedd; baseline branch and prepared binaries remain unchanged. Cloned public
Pumas into a fresh source-only checkout at exact candidate head. Candidate
production roots equal its reported main, but have substantial changes since
Pantograph's older aa1d79ae pin; real consumer qualification is necessary.

Pinned source/manifest/CI/demo lock to exact df53db08. Initial resolution selected
three unrelated existing socket2 bindings; restored baseline selections and
rebuilt `--locked`. Only Pumas version/source remains changed in the lockfile.
Candidate pin commit f8d327fc was followed by normal merge 039cee07 of reviewed
baseline repair 0dc45263. Original PR78 pin remains aa1d79ae. Staged traceability
initially lacked commonmark; existing same-lock tooling cache link enabled an
unchanged successful gate, without installation or bypass.

Exact candidate 039cee07 preparation built/audited public Chrema, compiled locked
CPU source and sealed fixture/package/binary/source identities. All 19 contracts
passed, zero skips; the concrete run completed and separate native/CLI inspection
verified exact output and owner provenance. Embedded-runtime 598 passed / 2 existing
ignores; 4 HTTP contracts, warning-deny example Clippy, rustfmt/ESLint and all
pin/no-build-download metadata checks passed. Independent GPT-6.1 Sol High
read-only review reran 19 contracts, inspected receipt/source/owner contracts and
accepted the bounded edge with no functional findings. Native API descriptor 1,
model-ref 1/package-facts 3/valid HF-directory binding is compatible here.

[Complete candidate receipt/archive](reports/consumer-compatibility.md) identifies
the exact tuple and limits. Both original and candidate results use an untrained
fixture; no pretrained T3, migration, other-platform, hosted candidate or release
qualification is implied. Candidate worktree/ref disposition is recorded at
handoff; parent owns adoption. No tags or upstream source changes.

Handoff disposition: preserve the pushed candidate branch and exact source
039cee07, remove only its clean task-created worktree after this documentation
commit is pushed/reachable. The same-lock node_modules cache symlink is removed
first; its target remains intact. The baseline ledger records the actual removal
and subsequent fresh native inspection from a private owner-store copy.
