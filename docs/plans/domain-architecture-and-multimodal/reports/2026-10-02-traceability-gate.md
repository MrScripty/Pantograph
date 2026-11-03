# Decision Traceability Gate Reconciliation

Date: 2026-10-02

Plan authority: [domain architecture and multimodal workflows](../plan.md).
Admitted base: `4938e405c7f656365eefdca492774ccae110c90d`.
Standards inspected: `dcc56f26e884ade260770beceba2501d3746200d` of Coding-Standards.
This later inspected revision is explicit; it is not a silent replacement of the
plan's historical planning baseline.

## Finding And Policy Decision

The previous checker required a README at every changed source directory, eleven
universal headings, path-list-selected host/producer headings, and a README or
any ADR edit. This contradicted the current Documentation workflow,
[repository documentation rules](../../../README.md), and the active plan's
Documentation Proportionality. Its process-substitution diff collection also
hid failed Git reads as empty successful checks.

Replace that mechanism with declared decision-to-guide impact checking. The
[project map](../../../../scripts/decision-traceability-map.json) and its
[owning guide](../../../../scripts/README.md#decision-traceability-operation)
select exact durable decision sources, affected knowledge and documentation
profiles. The decision is scoped gate reconciliation, not an exception to
contract documentation or permission to disable verification.

The admitted rows cover host/service and scheduler-session authority
(ADR-001/011), runtime lifecycle/readiness/observability (ADR-002/003/007),
canonical/composed-node and migration ownership (ADR-006/009), contributor
workspace policy (ADR-017), and the map's own coverage. Each decision was read
against its canonical guide before selecting the row. The gate validates an
update to that owner only; an unrelated README, plan or ADR cannot substitute.
Changed unmapped ADRs produce an unresolved mapping diagnostic.

Mixed implementation files are intentionally not triggers. A local repair that
preserves a contract needs no guide churn. Source changes that alter consumer
semantics still require canonical documentation, review and contract tests;
path comparison cannot decide those semantics. Existing scheduler-only,
host/binding, worker-protocol and structured-producer validation remains intact.
The removed heading checks never proved those contracts.

## Change Input And Migration

- The existing hook explicitly selects staged mode. All evaluated map, guide,
  decision and reference content comes from HEAD and the index tree. An unstaged
  repair cannot satisfy a staged obligation.
- Range mode requires explicit base and head commit refs, reports resolved IDs,
  and reads both snapshots. The affected CI job fetches history and supplies
  event base/head IDs; it does not guess a branch or adjacent revision.
- Both prior and current maps contribute obligations. Row removal, trigger
  deletion/rename and owner migration cannot erase the previous obligation.
- Initial map adoption is supported only from the exact audited no-map base
  above, with the repository-selected map path. Its decision sources and owners
  must resolve in that base. There is no override to skip an arbitrary absent
  prior map. Retire the migration case when supported bases all carry the map.
- Missing modes, revisions, maps, owners and referenced files fail explicitly;
  malformed and unreadable Git input cannot become a successful empty diff.

Local file links in current mapped owners and changed Markdown are checked.
Unchanged documents are checked when a referenced ADR or prior/current canonical
guide target changes, preserving rename/deletion protection. Remote URLs and heading fragments are not checked.
Untouched historical links remain outside scope. One superseded security plan
already names a never-created proposed trust/isolation ADR; the gate does not
turn that historical proposal into current authority or silently repair it.

## Write Set And Ownership

- `scripts/check-decision-traceability.sh`: retained shell entrypoint
- `scripts/check-decision-traceability.mjs`: Git-snapshot gate
- `scripts/check-decision-traceability.test.mjs`: isolated regression cases
- `scripts/decision-traceability-map.json`: bounded project impact coverage
- `package.json` / `package-lock.json`: development-only CommonMark parser and its locked dependency closure
- `scripts/README.md`: current gate operation and coverage owner
- `docs/development.md`: replace the now-retired-gate limitation
- `.github/workflows/quality-gates.yml`: only the affected job's history,
  explicit inputs and focused test invocation
- This report: dated deciding evidence

No runtime implementation, other CI job, standards checkout, active plan,
issues or execution ledger is edited here. The integration coordinator owns
master-plan reconciliation and downstream runtime branch dependencies.

## Independent Review Repair

Independent review found false positives for balanced-parenthesis destinations,
tracked directories and fenced examples, mapping-key ordering churn, and a
retained-prior-owner migration gap. The revised candidate uses the established
CommonMark parser instead of owning a regex Markdown grammar, treats tracked
trees as valid link targets, compares normalized maps, and requires a retained
old owner to explain migration. Deleted old owners remain permitted only with a
current replacement and repaired readers. Three regression cases cover these
findings. The parser is a pinned development dependency, BSD-2-Clause licensed;
no production dependency version or existing override changes.

Revised focused verification passes (24 cases). The integration owner independently
reviewed staged tree `90f286152a59c27bff775e73ceda2b8386dbd575`, including the
complete gate, regression additions, package/lock delta, wrapper and CI inputs,
and reproduced all 24 cases. That narrow review accepted the four repairs
without another finding. This report-only update records the result; unrelated
baseline CI and final remote-head qualification remain unsatisfied. `npm ci --ignore-scripts` installed the locked dependency closure.
CommonMark 0.31.2 and its new entities 3.0.1, mdurl 1.0.1 and minimist 1.2.8
entries are development-only; all prior version/integrity identities are unchanged.
The installed package retains its copyright/license notices. No third-party
source is vendored or copied into the product. The gate owner maintains the
parser dependency and retires it if the gate no longer consumes Markdown.
Node/npm here are 24.19.0/11.9.0 rather than the pinned 24.12.0/11.6.2; exact
pinned-toolchain qualification remains a hosted-CI obligation.

## Verification And Limits

- Node's ordinary test runner: 24 focused cases pass, covering code-only changes,
  required guide updates, unrelated documents, index/range isolation, missing and
  contradictory inputs, missing maps, malformed maps, broken references,
  removed/moved rows and paths, nonregular files, spaces/newlines, and failed or
  malformed Git diffs. Test commits have explicitly synthetic identities and
  live only in disposable temporary repositories.
- Real staged adoption from the audited base passes using the repository map.
- Repository tooling lint and focused ESLint over both new JavaScript files pass; Bash and Node syntax
  checks pass. The checkout uses its isolated locked dependency tree;
  no global configuration is changed. The reviewed repair adds the development-only
  parser dependency described above.
- `npm run lint:no-new` remains blocked before traceability by the existing
  `IoInspectorPage.svelte:520` DOM-mutation finding. This slice does not waive
  that gate or claim repository-wide lint success. The a11y check independently
  reports the same three existing findings as the clean base (Inspector and
  RunGraphSnapshot reviewed-ignore annotations; SavedGraphInspectionSnapshot
  keyboard activation).
- Passing this gate establishes selected traceability and file-reference
  integrity, not semantic prose completeness, all standards compliance, runtime
  correctness or release readiness. Independent narrow review and integration-owner
  approval permit draft publication. Merge remains conditional on exact-head
  CI and applicable hosted code review; neither is claimed complete here.

The candidate branch is `fix/decision-traceability-ownership-2026-10-02`, targeting
main. Its isolated checkout is retained for review and downstream dependency
coordination; it is not an accepted integration or cleanup instruction.

The existing pre-push `npm test` also passed on 2026-10-02: node-engine
258 passed / 1 ignored, workflow-nodes 168 passed. This proves its declared
library scope, not the unrelated failing aggregate/desktop gates.

## Hosted Review: Pull-request Range Selection

CodeRabbit reviewed exact head `9567186252f7ebb60794911baeeabce0daec5140` and
identified the PR target-tip versus fork-point mismatch. The workflow now
resolves a unique PR merge base explicitly, while push events retain their
before/head range. The gate continues to compare exactly the supplied snapshots;
it does not silently reinterpret an explicit range. Ambiguous/no common bases
fail. A diverged-branch regression covers unrelated target-only map additions.
This correction is separate from unchanged baseline CI failures.

Correction verification: 25 focused tests pass. The actual workflow shell was
extracted and executed against a disposable divergent Git fixture: PR mode
selected its fork point, and push mode preserved the exact before/head input.
Workflow YAML, Bash syntax, scoped ESLint and whitespace checks pass. The
integration owner independently reviewed staged tree
`85ab1fe64e85c7f299d53f99e5ebb74625730607`, read the actual workflow delta,
reproduced the divergent-PR regression and passed staged whitespace checks.
That bounded review found no new source issue and accepted publication.
This report-only addition records the review; exact-head hosted CI and hosted
review qualification remain outstanding.
