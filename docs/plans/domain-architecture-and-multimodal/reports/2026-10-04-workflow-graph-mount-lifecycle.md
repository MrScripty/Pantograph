# WorkflowGraph synchronous teardown qualification

Source base: `ca9edde6850cd58ade0b7e534bb4f58e704c2c64`, retaining the complete
PR44 stacked ancestry. Isolated branch: `repair/workflow-graph-mount-lifecycle`.

This bounded FE-A02 slice addresses the two canonical graph mounts identified
by the frontend audit and retained in the active repository-review requirements.
Both components registered six window listeners, awaited node definitions in an
asynchronous mount callback, and only then returned the listener remover. Svelte
requires synchronous mount cleanup; the returned promise did not register the
remover. A late definition response also applied state after unmount, and a
rejected request escaped the mount callback without a terminal error observer.

Both mounts now return cleanup synchronously. The shared graph-specific owner
registers the existing listeners with their existing identities and capture
options, owns one deferred definition read, and closes permanently at teardown.
Cleanup is idempotent; live definitions apply once, late responses settle without
application, and read failures are observed through the existing console error
channel while live. No backend request cancellation or connection/keyboard
policy changes are introduced. The existing listener registrar remains exported
for compatibility; the new mount helper and its options are additive exports.

Twelve controlled contract tests parse each actual Svelte mount callback and
execute that callback with deferred services, recording stores/listener targets,
and the real production mount owner. They assert synchronous cleanup before
resolution, live application, exact handler cleanup, no keyboard callback after
cleanup, live/late rejection observation, immediate destruction, and independent
remount authority. Both synchronous-cleanup assertions fail against the exact
base component sources. The existing six-listener identity/options test also
passes. This tests the actual callback wiring, but does not execute Svelte DOM
mounting or native IPC.

All 559 discovered frontend tests, full lint, TypeScript, production build,
staged critical/a11y/traceability (six paths, zero mapped impacts), and whitespace
checks pass; exact results are recorded in the milestone commit. The build retains its
existing stale Browserslist-data notice. Validation uses Node 24.12.0/npm 11.6.2.

GUI, native/model execution, async connection intents and other graph operation
lifetimes are outside this mount/read slice. It does not close every FE-A02
owner or claim native qualification. No frozen fixture/scanner/undo/view/device
source is changed. The branch is directly atop ca9; the integrator owns
independent review and integration.
