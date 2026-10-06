# @pantograph/svelte-graph

## Purpose

`@pantograph/svelte-graph` contains the reusable Svelte graph editor package
used by the Pantograph desktop app. The package owns graph editor components,
stores, and interaction helpers; the root app owns product orchestration and
repository-wide tooling.

## Dependency Ownership

This package declares only the dependencies that are part of its package
consumer contract:

| Manifest section | Owner | Rationale |
| ---------------- | ----- | --------- |
| `peerDependencies.svelte` | `@pantograph/svelte-graph` | Consumers must provide the Svelte runtime used to render package components. |
| `peerDependencies.@xyflow/svelte` | `@pantograph/svelte-graph` | Consumers must provide the graph rendering library that the package components integrate with. |
| Root `dependencies`, `devDependencies`, and `overrides` | Repository root | The root app currently owns the executable, TypeScript compiler, ESLint config, Node test command, and transitive security pins that run package tests and harden the shipped dependency graph. |

`packages/svelte-graph/package.json` intentionally has no package-local
`scripts` today. The package tests are run by the root-owned
`npm run test:frontend` command, which also covers app-level graph integration
tests that consume this package.

If this package adds a package-local `build`, `test`, `lint`, or code generation
script, the package must also declare the dev dependencies needed by that script
in `packages/svelte-graph/package.json`. Hoisted or root-only tooling must not
become an implicit dependency of a package-local command.

## Source Reference

Saved workflow reopening reads the canonical edit-session graph after creating
the session. The backend owns its semantic revision, including authored data;
file-only or topology-only fallback revisions cannot authorize validation or
submission. A failed or superseded snapshot load does not publish a stale graph
or session handle. Workflow metadata remains the saved file's metadata.

Reconnectable edges use CSS drop-shadow for their glow. Horizontal and vertical
paths can have zero-area geometry bounds; an SVG filter region based on those
bounds clips away the line. Endpoint gradients and reconnect controls are retained.

Implementation lives under `packages/svelte-graph/src/`. Repository-level
development and verification guidance is in
[`docs/development.md`](../../docs/development.md).

## Navigation Animation Lifetime

Each view-store instance owns one navigation animation timer. New animated
navigation supersedes the prior animation; reset and immediate breadcrumb
navigation cancel it. Superseded/cancelled callers settle their existing
`Promise<void>` without clearing a newer target or animation state. Navigation
state still changes immediately, and the current animation clears its transient
target only when its configured duration finishes. No-op navigation preserves
the current animation. Store instances keep independent timer ownership.

Automatic view persistence also has one shared subscription/debounce scope per
store. Each `enablePersistence()` call returns an independent, idempotent release
handle. Persistence remains active until the last handle is released; final
release unsubscribes and cancels the pending write. Re-enabling starts a fresh
scope. The existing storage shape and 500 ms debounce are unchanged.


View persistence reads the existing unversioned record's `viewLevel`, nullable
string `orchestrationId`/`dataGraphId`, and string-array `groupStack`. It validates
all present owned fields before applying any of them. Missing legacy fields leave
current state intact; explicit null identities clear that identity. Empty strings,
partial records and ignored extension fields remain compatible. No new length or
group-count limit or schema migration is introduced.

`enablePersistence()` initializes valid stored state before installing the first
shared writeback scope. After an explicit restore or earlier scope, enabling
validates storage without replacing current edits. Each write rechecks existing
storage and validates its outgoing snapshot. Restoration suppresses synchronous
subscriber write attempts; an already pending debounce observes the complete
restored snapshot. Invalid JSON/records or unavailable storage stop writeback
for that instance and retain
the stored bytes. Later reads may update memory from externally corrected data,
but they do not restart writes. There are no recovery writes. A fresh instance
must validate storage again. `ViewStoreOptions.storage` can supply a per-instance
`getItem`/`setItem` boundary; it otherwise uses browser `localStorage`.
