# Desktop text-seed precision evidence

The frozen base is `6cf549dd04933bd49a15b4527732d6a5db6e3c71`, tree
`790791fba8cfc904664cc79f65f0672293332cff`. Fetched main `d61b86fc` has the exact
reviewed `d0c9788c` repair tree and is incorporated normally at a clean boundary.
`qualification.json` records identities, source hashes, boundaries and results;
`SHA256SUMS` hashes the archived bytes. Original seed evidence remains unchanged.

`original-rounding.log.gz` preserves the failure before implementation.
`browser.log.gz` is the passing actual-component Chromium regression: fourteen
groups, with raw invalid input retained and rejected on repeated save/load/replay.
The graph store and BaseNode shell are fixtures, and submission invokes the real
guard. `rust-wire.log.gz` proves omission, zero, JavaScript boundary integers and
full u64 through actual JSON text on the unchanged typed API.

Reproduce using existing installed dependencies:

```sh
node --test src/components/nodes/workflow/desktopSeedPrecision.test.ts
node scripts/check-desktop-seed-precision-browser.mjs
npm run test:frontend
npm run typecheck
npm run build
export ORT_SKIP_DOWNLOAD=1
cargo tree --locked --offline -p inference --features backend-pytorch --target all -e features
cargo test --locked --offline -p inference --features backend-pytorch --lib chat_seed_maps_alone_without_changing_omitted_options
cargo clippy --locked --offline -p inference --features backend-pytorch --all-targets -- -D warnings
```

Chromium must already be installed (`CHROMIUM_BINARY` may select its executable).
For this PyO3 environment, set `LD_LIBRARY_PATH` to the installed Python library
directory and use a writable isolated test `XDG_CONFIG_HOME`. No package or
browser installation is part of the harness. The complete effective selected
Cargo graph is archived; it has no ORT/Pumas or download-binaries feature.
Workspace manifest/lockfile/dynamic-ORT configuration are unchanged.

Full frontend and Vite build logs use worktree-local workspace package links.
The earlier build link failure is retained separately, as are unsuccessful
Chromium dump-DOM attempts and the initial fixture target correction. The final
harness uses local DevTools and compiles the real Svelte component and helpers.
Routine affected static gates are recorded separately from deciding regressions.

Qualification covers the bounded direct NumberInput seed edge and controlled
JSON persistence/component behavior. It does not qualify a complete Tauri
scheduler run, a new arbitrary-precision graph representation, model loading,
GPU/pretrained/custom models, full worker import or ONNX. Accepted RNG scope and
the seed base's controlled CPU evidence are preserved; full-u64 desktop NumberInput
entry is explicitly refused.
