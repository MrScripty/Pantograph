# FE-A05 bounded undo persistence validation

This slice addresses the existing browser-persistence obligation retained by M4
and DA-05. It does not close FE-A05 for other storage owners, introduce a record
version, migrate data, or qualify desktop cold reopen.

## Admission and behavior

The parent admitted whole-record rejection for malformed/oversized undo data,
retaining stored bytes and invoking no recovery/deletion callbacks. The existing
`pantograph-unified-undo` key, unversioned valid shape and 32-entry limit remain.
Opaque entry IDs and commit hashes must be nonblank strings; no hash format or
new string-length restriction is imposed. Timestamps must be nonnegative safe
integers, as produced by the existing millisecond timestamp writer. Positions
must be integers from -1 through the last entry; an omitted legacy position
still defaults to the last entry. No wall-clock freshness or ordering policy is
added. Nested invalid entries reject the whole record instead of being filtered.

Rejected or unreadable storage initializes an empty in-memory history and logs
`UNDO_STORE_LOAD_FAILED` through the existing application logger. Persistence
remains disabled for that store instance: later actions and clear operate in
memory and log `UNDO_STORE_SAVE_FAILED` instead of replacing the stored bytes.
There is no automatic recovery write, removal or schema migration. Valid or
absent records retain normal writes and error reporting.

The store factory owns its callbacks and receives storage/logging dependencies;
the application retains its existing singleton and exported action/entry types.
The operation bodies are unchanged after logger/constant renaming. Valid full
histories still age their oldest action only on a new push; the timeline's Git
cleanup owner is not changed. Rejected entries never enter that callback path.
New user actions may form and age a fresh in-memory history under the existing
policy even when persistence remains blocked.

## Evidence and limits

The base `ca9edde6850cd58ade0b7e534bb4f58e704c2c64` restored a null nested action
that threw in undo, accepted cursor 99 while reporting canUndo, and loaded 33
entries. The read-only probe used isolated storage and no timeline/Git callback.

Focused decoder/store tests use recording callbacks only. They cover malformed
and mixed records, invalid JSON, bounds, opaque identifiers, legacy cursor
fallback, repeated restore, unavailable storage, byte preservation after new
pushes/clear, normal undo/redo and reread, redo truncation, valid capacity aging,
new-history aging after rejection, callback failure, save failure and isolated
stores. No permanent Git-object deletion or Tauri invocation occurs in tests.

The 35 focused tests and all 582 discovered frontend tests pass on pinned Node
24.12.0/npm 11.6.2. TypeScript, full lint, production build and the staged
critical-lint/accessibility/traceability gate pass. The final scope is five paths
with zero mapped decision impacts; semantic review remains required. The production build emits the existing stale Browserslist-data notice.
No dependency manifests, audit gates, native downloads or review branches change.
This is deterministic frontend contract evidence, not GUI/native/model or full
persistence-migration acceptance. Independent review remains with the integrator.
