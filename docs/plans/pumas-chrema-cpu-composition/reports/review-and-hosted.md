# Independent review and continued hosted qualification

Original PR78 source: `34dfedd27dde9a30274226e1ea26ec1e1b2de8e3`.
The original [local archive](verification.md) remains historical evidence for
that source; it does not certify the later lifecycle/input-isolation repairs.

## Original-head hosted observations

- [CPU Composition 38523394180](https://github.com/MrScripty/Pantograph/actions/runs/38523394180): passed; the exact baseline Pumas/Chrema tuple built and ran.
- [Headless 38523394032](https://github.com/MrScripty/Pantograph/actions/runs/38523394032): passed.
- [Runtime Separation 38523394026](https://github.com/MrScripty/Pantograph/actions/runs/38523394026): passed.
- [Quality 38523394065](https://github.com/MrScripty/Pantograph/actions/runs/38523394065): initial frontend job 115718050998 passed 731 unit tests, then Chrome 154 sandboxed startup exceeded its 10-second deadline before any browser acceptance. One ordinary failed-job rerun passed all13 jobs. Actual retry job 115722023455 log identifies source34dfedd, sandbox mode enabled, complete browser outcomes and cleanup completed. No source or sandbox flag changed. The initial failure remains disclosed.

## Review findings and repair

Independent GPT-6.1 Sol High review of original34dfedd reproduced:

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

Reviewer independently ran all19 integration contracts, zero failures/skips;
reproduced exit7 and exit0 descendants reaching terminal execution state;
and ran the original documented inspection command with the original prepared
binary/bundle, preserving exact run/vector/artifact links. No remaining concrete
finding on reviewed working diff over34dfedd:
SHA256 `8d49380e076f56dbb918b8d87ee5ca1915015b200886c117bd4f46c33e8a9c5a`.
Later changes before the repair commit add only review/evidence documentation.
Review was read-only; broad suites and hosted checks were not rerun by the reviewer.

Local repaired source: all19 integration contracts, example ESLint, workspace
rustfmt, affected example warning-deny Clippy, and existing4 native HTTP host
contracts passed. Current corrected-source CI is pending until publication and
must be observed separately. Untrained CPU fixture qualification and PR75's
required pretrained text T3 remain distinct.

## Candidate isolation

Parent separately authorized Pumas PR70
`df53db08e2ba224edfaf713522ddd410f7d4ac44` qualification. Its source receipt
SHA256 is `617b7fb1ec18c04274d7ad56d346d0a8e6d62b2b3358b1b1f18a139715fdb814`;
production roots equal `ef9eead9fe6595f5b3152ef8df19c5dace30190b`, verified
against the actual public checkout. Candidate changes are isolated in branch
`qualification/pumas-pr70-cpu-consumer-2026-10-14` and worktree
`/workspace/Pantograph-pumas-pr70`. PR78's baseline remains
`aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`; no new pin is implied by this
report. Initial candidate compilation and real native model API/run/inspection
smoke passed; final repaired-source consumer acceptance remains pending.
No upstream API failure has been observed so far. No release tag is created.
