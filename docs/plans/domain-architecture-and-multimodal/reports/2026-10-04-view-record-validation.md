# View record validation qualification

Isolated successor branch: `repair/view-record-validation`.
Exact base: `bb45b81f72b258f71121a794184eef812769632d`, tree
`e1d1e723cd8818d1b3f5760feeec64b2faa52a46`. The accepted frontend composition
and all four reviewed source commits remain unchanged in ancestry. Post-base
native fixture/report `827b1b` and scanner `9996dd` branches remain excluded.

## Reproduced FE-A05 defect and bounded repair

The active repository review retains browser record validation before application.
On the exact base, a record with a numeric orchestration identity and an object
instead of a string stack changes state, reports success without subscribers,
and breaks breadcrumb iteration. With an active breadcrumb subscriber, restore
reports failure but still leaves the malformed fields applied. Enabling
persistence also installs writeback without reading the malformed stored bytes,
allowing its initial debounce to overwrite them with default/local state.

Two focused regressions fail against the exact base component store source:
no state application on rejection, and no subscription installation for malformed
storage. The original script probe also fails before this repair. After repair,
it returns false, retains default valid state, evaluates breadcrumbs without an
error and records zero writes. All probes use controlled storage rather than a
real user's browser data.

The 38-line owner-local decoder validates every present owned field of the
existing unversioned record: the three view-level literals, nullable string
identities, and an array containing only strings. Numbers, including nonfinite
values produced by JSON exponent overflow, are invalid in these owned fields.
No numeric viewport or animation field belongs to this persisted record.
Missing legacy fields retain current state; explicit null identities clear their
prior values. Empty strings, arbitrary existing identity lengths/group counts,
partial/empty records and ignored extension fields remain compatible. No new
schema version, migration, domain inference or product bound is introduced.

Initial persistence startup validates and restores before subscription callbacks
can schedule defaults. An explicit restore or previous persistence scope marks
initialization complete; re-enabling validates storage without replacing later
local edits. Duplicate owners still share one debounce and final release cancels
its timer. Synchronous subscriber writes are suppressed during restoration;
existing pending work snapshots the complete restored state. Before every write,
the owner rechecks storage and validates the outgoing writable-store snapshot.

Invalid/unavailable storage, invalid outgoing data or a failed write closes
writeback for that instance and cancels its subscriptions/pending timer. Original
bytes are retained; local store operations remain available. An explicit later
restore may apply externally corrected valid data to memory, but never resumes
writes. There are no automatic recovery writes. A fresh instance validates again.
The additive optional `ViewStoreOptions.storage` uses the existing browser
getItem/setItem Interface; default browser localStorage and public action return
types remain. The existing timer fixture now supplies getItem as well as setItem.

## Actual evidence and limits

Validation uses repository-pinned Node 24.12.0/npm 11.6.2. No dependencies,
permissions, credentials or network settings changed.

- 55 focused cases pass: 40 new decoder/storage cases plus all 15 accepted view
  lifecycle cases. They cover breadcrumb iteration, partial/null/repeated
  restores, malformed JSON/root/nested types, nonfinite owned fields, valid
  roundtrip serialization, subscriber/writeback ordering, unavailable storage,
  failed writes, isolated storage/instances and retained-byte behavior.
- Two compatibility regressions exposed a repeated-start reset in the initial
  repair draft; the initialization phase fixes it. Both compatibility cases pass
  on the exact baseline and final repair, protecting edits between scopes or
  after an explicit restore.
- All 659 discovered frontend tests pass, with zero failures/skips/cancellations.
- Full lint, TypeScript and production build pass. The build retains its existing
  stale Browserslist-data notice.
- Staged critical checks, nine accessibility scanner tests, accessibility scan,
  traceability (seven paths, zero mapped impacts), and whitespace checks pass.
  Exact final source identities are recorded in the milestone commit and handoff;
  semantic contract review remains parent-owned.

Detailed local evidence uses `/workspace/pantograph-cache/view-record-*`; these
cache files are not assumed to transfer to a fresh environment. This tracked
report preserves the actual results and scope. Native IPC, GUI/WebKit interaction,
real browser cold reopen, model loading/inference and release acceptance were not
executed. Whole FE-A05 acceptance, including other record owners and future-version
policy, remains open. Parent owns independent review and final all-source
integration. No PR44/47 refs or external review requests are changed here.
