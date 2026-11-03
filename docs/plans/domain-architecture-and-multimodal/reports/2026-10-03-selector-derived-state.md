# Selector State Derivation

The current Full lint job reports one error in PumaLibNode.svelte: a `$state`
value is copied from `data.model_id` by an effect. Replace that pair with writable
`$derived` state. The existing selection handler can still update the value
optimistically, and incoming node data remains authoritative. No selection,
hydration, cache or error-handling code changes.

The locked Svelte 5.55.4 supports writable derived values; official documentation
covers this optimistic-update pattern from 5.25 onward:
https://svelte.dev/docs/svelte/$derived#Overriding-derived-values

Verification: existing selector-state and model-options-cache tests pass 15/15;
full ESLint, TypeScript typecheck, direct Svelte client compilation and whitespace
checks pass. No dependencies or Rust sources change. No browser interaction claim
is made. Independent review and exact-head hosted qualification remain pending.
The separately bot-reviewed/native-qualified PR #15 head remains unchanged;
critical/a11y lint, Clippy and dependency audit still have separate work.
Independent integrator source review accepted tree
`0435e312fab157a8d9e3d6b5492e3f0197aa44e6` after checking the component and
selection handler. Optimistic writes and source-driven resynchronization remain;
no broader error/hydration behavior change is implied. Hosted gates remain required.
