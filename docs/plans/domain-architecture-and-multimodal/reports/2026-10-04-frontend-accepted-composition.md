# Accepted frontend composition qualification

Isolated branch: `qualification/frontend-accepted-composition`.
Source base: `ca9edde6850cd58ade0b7e534bb4f58e704c2c64`.
Composition commit: `81ced09473d932a47aa55161421abdfa43bfc7f5`.
Qualified source tree: `746fd66ed1062565ab03eca39f1845b26754c427`.

## Exact admitted source and isolation

The parent reported independent acceptance of these four exact repairs:

| Repair | Exact accepted commit |
| --- | --- |
| Undo persistence validation | `b504b8387dcf8e2cebe1dd28674e6f66d9eec115` |
| View animation and persistence timer lifetime | `934147092e33b1adfb0d0c17424dd12e22608efa` |
| DeviceConfig initialization lifetime | `6a3ef2cf8a42c0ef0c0823c3d5af5384774a0a44` |
| Application/package graph mount lifetime | `9d2fe06f48002b9d5d09fdacc24f3047efa4692e` |

A single ancestry-preserving composition commit has ca9 and all four accepted
commits as parents. The 19 changed source/documentation paths are exactly the
union of those disjoint repairs. Each changed blob matches its accepted source
commit. Graph consumers, helper implementation and public exports are composed
together. Remote source heads were read and matched before composition.

Post-base native fixture/report branch `827b1b264a5918e2339c503f447aeb01008ac9b4`
and scanner branch `9996ddfd8b4c15c11de4d7141e20e0165dd0f12c` are absent from this
ancestry and write set. No scripts, workflows, Rust/native source, credentials,
network settings, protected PR44/47 refs or external review metadata were changed.
This frontend milestone is independent of the separately blocked native-qualified
publication. The final report successor changes only this Markdown evidence file.

## Actual combined checks

Environment: published Debian cloud checkout, Node 24.12.0/npm 11.6.2 matching
repository pins. Existing installed dependencies passed `npm ls --depth=0`.
No dependencies or toolchain pins were changed.

| Command | Actual result |
| --- | --- |
| `node --experimental-strip-types --test src/stores/undoPersistence.test.ts packages/svelte-graph/src/stores/createViewStores.test.ts src/components/deviceConfigLifecycle.test.ts src/components/frontendLifecycleContract.test.ts packages/svelte-graph/src/workflowGraphWindowListeners.test.ts` | 73 passed |
| `npm run test:frontend` | 619 passed; zero failed/skipped/cancelled |
| `npm run lint:full` | passed |
| `npm run typecheck` | passed |
| `npm run build` | passed; existing stale Browserslist-data notice |
| `TRACEABILITY_MODE=range TRACEABILITY_BASE_REF=ca9edde6850cd58ade0b7e534bb4f58e704c2c64 TRACEABILITY_HEAD_REF=81ced09473d932a47aa55161421abdfa43bfc7f5 npm run lint:no-new` | critical, nine accessibility scanner tests, accessibility scan and traceability passed; 19 paths, zero mapped impacts |
| `git diff --check ca9edde6850cd58ade0b7e534bb4f58e704c2c64 HEAD` | passed |

The final documentation successor also receives explicit ca9-to-head traceability
and whitespace checks. Local detailed logs use the prefix
`/workspace/pantograph-cache/frontend-combined-`; these cache files are not
assumed to transfer to a fresh environment. The tracked results above preserve
the meaningful evidence. No fresh hosted workflow or external review is claimed.

This is combined frontend qualification. Controlled timers, deferred services,
recording storage and execution of actual mount callbacks do not execute Svelte
DOM interactions, desktop IPC, package/native loading or model inference. Those
checks were not executed. Visual, keyboard/focus behavior in WebKit/Tauri,
real cold reopen, release artifacts, all FE-A02 owners and whole FE-A05 acceptance
remain unqualified. The parent owns final all-source integration and review.

## Next bounded source candidate: view record validation

The active repository review retains FE-A05 browser persistence bounds, versions,
migration and typed outcomes before application. Its historical owner-specific
population includes `createViewStores.ts`; the successor transfers those
obligations rather than accepting them by supersession.

On this combined source, `restoreViewState()` still trusts parsed fields before
applying them. A controlled in-memory storage probe supplies
`{"viewLevel":"group","orchestrationId":42,"dataGraphId":"graph","groupStack":{"length":1}}`.
The actual store reports `true`, applies the numeric identity and object stack,
and breadcrumb evaluation throws `TypeError: $groupStack is not iterable`.
An explicit persistence call writes that malformed state back. No real browser
storage is read or written by this probe. Its source and output are retained as
`/workspace/pantograph-cache/view-persistence-gap-probe.mjs` and `.log`.

A proportionate next goal is an owner-local decoder for the existing unversioned
view record, validating its enum, optional identities and bounded string stack
before any application, with controlled malformed/nested/unavailable/repeated/
isolated-store evidence and preservation of rejected bytes. Preserve valid record
serialization and the accepted navigation/timer lifetime. Inject a small storage
Interface where needed for evidence; do not create a universal registry or alter
backend domain authority. Future-version/migration and public outcome changes
need an explicitly admitted compatibility scope; this milestone does not choose
those policies or implement this candidate.

Other inspected persistence owners, including package/application last-graph
restore, retain raw parsed-record trust. They are separate candidates, not part
of this view-record proposal or the accepted composition write set.
