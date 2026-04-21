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
| Root `dependencies` and `devDependencies` | Repository root | The root app currently owns the executable, TypeScript compiler, ESLint config, and Node test command that run package tests. |

`packages/svelte-graph/package.json` intentionally has no package-local
`scripts` today. The package tests are run by the root-owned
`npm run test:frontend` command, which also covers app-level graph integration
tests that consume this package.

If this package adds a package-local `build`, `test`, `lint`, or code generation
script, the package must also declare the dev dependencies needed by that script
in `packages/svelte-graph/package.json`. Hoisted or root-only tooling must not
become an implicit dependency of a package-local command.

## Source Reference

Implementation-level contracts for `packages/svelte-graph/src/` are documented
in `src/README.md`.
