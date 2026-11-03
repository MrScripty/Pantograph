# FE-A02 view-store timer ownership

The active M4 coverage retains the async-owner obligation from FE-A02. Source
inspection confirmed the two gaps in the existing `createViewStores` owner:
old animation timers clear newer navigation state, and repeated automatic
persistence starts create duplicate subscriptions and writes.

A base-ca9 probe started a 10 ms group animation followed by a 1000 ms animation.
After the first caller completed, the second remained pending but isAnimating
was false and zoomTarget null. A second isolated probe started persistence twice
and observed two pending timers and two writes for one snapshot. Neither probe
used Tauri, models or production storage.

The repair gives each store one animation timer. A new animated navigation
cancels the old timer and settles its caller; reset and immediate breadcrumb
navigation do the same. Only the current timer clears transient state. Graph
navigation state still changes immediately and no-op navigation preserves the
current animation. The existing Promise<void> signatures are retained.

Automatic persistence shares subscriptions and its existing 500 ms debounce
while any enable handle remains owned. Handles release idempotently; final
release unsubscribes and cancels the pending write. Re-enabling starts a fresh
scope. Storage shape, restoration and migration policy are outside this slice.
The package guide records these lifecycle semantics.

Fifteen deterministic Node timer tests cover single/overlapping navigation,
reset, each animation route, immediate breadcrumbs, no-op behavior, isolated
stores, duplicate persistence starts, independent ownership, final cleanup,
restart and disabled persistence. The supersession regression fails against the
base implementation (only its runtime TypeScript import spelling is normalized
for Node). All 562 discovered frontend tests, full lint, TypeScript and production
build pass on Node 24.12.0/npm 11.6.2. The build retains the stale Browserslist-data
notice. Staged critical/a11y/traceability checks pass (nine accessibility
checks, four traced paths, zero impacted requirements), as does whitespace
validation. Exact results are recorded in the commit message.

This is controlled frontend lifecycle evidence. It does not qualify browser
interaction, GUI/native/model execution, persisted view decoding, or all FE-A02
owners. The branch is directly atop ca9 and disjoint from the frozen
fixture/scanner/undo repairs; the integrator owns independent review.
