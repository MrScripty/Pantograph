# DeviceConfig initialization lifetime qualification

Source base: `ca9edde6850cd58ade0b7e534bb4f58e704c2c64`, retaining the complete
PR44 stacked ancestry. Isolated branch: `repair/device-config-lifecycle`.

This is a bounded FE-A02 repair. The active repository review retains FE-A02
async terminal observation and cleanup; the frontend audit explicitly names
DeviceConfig's destruction race. The component's asynchronous mount callback
awaited the initial device request before constructing its refresh scope. A
mount destroyed during that request had already run cleanup when the continuation
created a new interval and applied device and embedding-mode state.

A controlled script probe runs the actual component script after TypeScript
transformation, injecting deferred services, lifecycle callbacks, identity runes,
and a recording interval API. On the base, destroying before the initial device
response then resolving it releases all three subscriptions but leaves one active
interval, starts a mode read, and applies both late values. On this repair, the
same sequence leaves zero intervals, starts zero mode reads, and retains the
initial device/mode values. The mount callback returns synchronously. This is
script-level lifecycle evidence, not Svelte DOM or native IPC execution.

The mount now owns an initialization scope before asynchronous work begins.
Duplicate starts share one promise. Destruction permanently closes that scope,
stops its refresh owner, and blocks later mode application or refresh creation.
Device reads capture the same owner and observe successful and rejected late
responses without mutating unmounted component state. Live initialization
failures are observed through the component's existing console error channel.
Backend requests are allowed to settle; this slice introduces no backend
cancellation, mutation, credential, network, or configuration policy.

Ten deterministic helper tests cover duplicate starts, immediate destruction,
destruction during both reads, live and late failures, independent remounts,
idempotent cleanup, and reentrant destruction during refresh creation. The
component probe also verifies its wiring and late device-state suppression.
All 557 discovered frontend tests, full lint, TypeScript, production build,
staged critical/a11y/traceability (four paths, zero mapped impacts), and whitespace
checks pass on Node 24.12.0/npm 11.6.2. The build retains its existing stale
Browserslist-data notice. Exact results are recorded in the milestone commit
message.

GUI, model inference, native IPC, and browser interaction were not executed.
Save-command lifetime and foreground/background device refresh ordering are
outside this initialization/teardown slice; this report does not close all
FE-A02 owners. No frozen fixture/scanner/undo/view source is changed. The branch
is directly atop ca9; the integrator owns independent review and integration.
