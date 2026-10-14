# Independent review and continued hosted qualification

Original PR78 source: `34dfedd27dde9a30274226e1ea26ec1e1b2de8e3`.
The original [local archive](verification.md) remains historical evidence for
that source; it does not certify the later lifecycle/input-isolation repairs.

## Original-head hosted observations

- [CPU Composition 38523394180](https://github.com/MrScripty/Pantograph/actions/runs/38523394180): passed; the exact baseline Pumas/Chrema tuple built and ran.
- [Headless 38523394032](https://github.com/MrScripty/Pantograph/actions/runs/38523394032): passed.
- [Runtime Separation 38523394026](https://github.com/MrScripty/Pantograph/actions/runs/38523394026): passed.
- [Quality 38523394065](https://github.com/MrScripty/Pantograph/actions/runs/38523394065): initial frontend job 115718050998 passed 731 unit tests, then Chrome 154 sandboxed startup exceeded its 10-second deadline before any browser acceptance. One ordinary failed-job rerun passed all 13 jobs. Actual retry job 115722023455 log identifies source 34dfedd, sandbox mode enabled, complete browser outcomes and cleanup completed. No source or sandbox flag changed. The initial failure remains disclosed.

## Review findings and repair

Independent GPT-6.1 Sol High review of original 34dfedd reproduced:

1. A leader terminating on SIGTERM cancelled pending escalation while its
   descendant continued executing. The owning helper now preserves escalation
   after leader close. The same family was extended to nonzero and successful
   early exit: it drains executing group members and preserves the original
   exit error; successful exit with background group members is refused.
2. A symlinked ancestor under a sealed evidence tree passed final-file hashes
   and redirected artifact manifest writes outside the private copy. Inventory
   now rejects root/ancestor and complete-tree symlinks. The private copy is
   dereferenced and compared with original hashes before opening owner stores.
3. The prepared binary still selected fixtures from its compile-time source
   checkout. Preparation now copies/hashes the complete fixture tree, and the
   producer gets an explicit sealed fixture root. Invalid explicit selection
   fails instead of falling back to checkout data. Prior exact preparations
   still support retained-state inspection, which needs no fixture inputs;
   they cannot produce new workflows until rebuilt with sealed inputs.

Reviewer independently ran all 19 integration contracts, zero failures/skips;
reproduced exit7 and exit0 descendants reaching terminal execution state;
and ran the original documented inspection command with the original prepared
binary/bundle, preserving exact run/vector/artifact links. No remaining concrete
finding on reviewed working diff over 34dfedd:
SHA256 `8d49380e076f56dbb918b8d87ee5ca1915015b200886c117bd4f46c33e8a9c5a`.
Later changes before the repair commit add only review/evidence documentation.
Review was read-only; broad suites and hosted checks were not rerun by the reviewer.

Local repaired source: all 19 integration contracts, example ESLint, workspace
rustfmt, affected example warning-deny Clippy, and existing 4 native HTTP host
contracts passed. Untrained CPU fixture qualification and PR75's required
pretrained text T3 remain distinct.

## Exact repaired-source acceptance

Qualified Pantograph source: `0dc45263d014bb9b42884242547424070ad65cc1`.
Independent GPT-6.1 Sol High review accepted this exact commit, with 19 contracts
passing, zero failures/skips and no remaining concrete findings. Fresh preparation
built the locked CPU probe and public Chrema package, sealed inputs, completed
the workflow and verified its output/provenance in a separate process.

All four hosted workflows passed for this repaired source:

- [CPU Composition 38525390950](https://github.com/MrScripty/Pantograph/actions/runs/38525390950): job 115724433829 passed 19 contracts, zero failures/skips, then completed the concrete run and fresh retained-state inspection.
- [Quality 38525391091](https://github.com/MrScripty/Pantograph/actions/runs/38525391091): all 13 jobs passed. Frontend job 115724434989 identifies source 0dc45263, 731 unit tests, sandbox enabled and complete browser outcomes/cleanup.
- [Headless 38525390920](https://github.com/MrScripty/Pantograph/actions/runs/38525390920): passed, including packaging.
- [Runtime Separation 38525391006](https://github.com/MrScripty/Pantograph/actions/runs/38525391006): passed.

[Hosted evidence artifact 11876380376](https://github.com/MrScripty/Pantograph/actions/runs/38525390950/artifacts/11876380376)
has metadata size 214481 bytes and advertised digest
`752270cd7be3de5342706b0fe9839dc8086e0a5ffa11c0839a772bb616e13254`.
Actual job logs were retrieved and retained. Downloading the signed artifact URL
locally was denied by the network proxy (CONNECT 403 Forbidden); the ZIP bytes
and advertised digest were not independently checked. This does not replace
the independently inspected local bundle.

The [corrected local evidence archive](cpu-evidence-0dc45263.tar.gz) contains 49
files, 245557 bytes; SHA256
`35c94a114fbd93b475cadf1e85f80636b276db156691c7573c4d04ead9313cf0`.
It retains the complete owner bundle, actual corrected-head hosted CPU/frontend
logs, original failed/retried Chrome logs, and local contract/Clippy/HTTP/ESLint
receipts. SQLite credential/client/session tables contain zero rows; populated
records describe only this synthetic fixture workflow.

Prepared binary SHA256:
`93256c9aba2815a7c3a75024ca1bcce5b78825fcdaa2411bab2df63ea4b1247f`.
Cargo.lock SHA256:
`42c43401def2b14f41eead4f533f3b48f463a80602271d87602cec705d0ac002`.
Local preparation `/workspace/scratch/demo-prepared-0dc45263`; bundle
`/workspace/scratch/demo-bundle-0dc45263`. Chrema run
`d04627f5-0d63-4dc4-845d-57b644d060e6`; Pantograph run
`run_adb4048c-f61a-4039-8629-185ffcb0a44b`.
Documentation-only publication does not change this qualified executable source.

## Candidate isolation

Parent separately authorized Pumas PR70
`df53db08e2ba224edfaf713522ddd410f7d4ac44` qualification. Its source receipt
SHA256 is `617b7fb1ec18c04274d7ad56d346d0a8e6d62b2b3358b1b1f18a139715fdb814`;
production roots equal `ef9eead9fe6595f5b3152ef8df19c5dace30190b`, verified
against the actual public checkout. Candidate changes are isolated in branch
`qualification/pumas-pr70-cpu-consumer-2026-10-14`. Its clean task-created worktree
`/workspace/Pantograph-pumas-pr70` was retired after verifying pushed head
`3649df2d3c4c718aa278e809f27cbc2a915f8c47` and preserving the branch. An exact
copied candidate CLI subsequently verified the sealed bundle using a fresh native
process/private owner-store copy, with original bundle hashes unchanged.
PR78's baseline remains
`aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`; no new pin is implied by this
report. Qualified candidate source is
`039cee079d273a8d4968df56a4b35e8eb8aacbbe`; evidence head 2971ffc1 records
19 integration contracts, 598 embedded-runtime tests (2 existing ignores), 4 HTTP
contracts, Clippy/format/ESLint and pin/no-build-download gates. Independent review
accepted this exact source. Of 1,116 parsed locked packages, only Pumas version
and source changed. No consumer API repair was required or observed API failure
hidden. The candidate branch's
`docs/plans/pumas-pr70-cpu-consumer/reports/consumer-compatibility.md` contains
the exact receipt/archive. Hosted candidate CI, migration, pretrained T3 and
release adoption remain parent-owned. No release tag is created.
