# Desktop text seed precision

Review reproduced `9007199254740993` becoming `9007199254740992` in
NumberInput's generic Number conversion before Rust could validate it. The separate
`fix/text-seed-desktop-precision` successor starts at fetched, frozen seed
`6cf549dd04933bd49a15b4527732d6a5db6e3c71`, tree
`790791fba8cfc904664cc79f65f0672293332cff`. Parent subsequently reported PR58 merged;
normal fetch confirmed main `d61b86fc27a7ecd725bd994b29a7cdcfcef9a6a3`, parents
`763e8d4b`/`d0c9788c`, tree `e714ccb9508797268964a3c42190280d57b7b043`.
Main has exactly the prior reviewed repair tree and is incorporated by normal
merge at the clean repair boundary. Frozen seed/repair reports, branches and the
earlier paused seed worktree remain unchanged.

The desktop contract for NumberInput directly connected to canonical
`llm-inference.seed` is whole numbers from 0 through `9007199254740991`
(`2^53 - 1`). Every fan-out edge is checked. Authored strings are checked for
decimal integer syntax before conversion, including fractional strings which
could otherwise round into the safe range. Invalid input remains its original
string in graph data, survives JSON save/load and displays a clear inline error.
Seed fields use text with a numeric keyboard: browser numeric-field sanitization
cannot turn partial `-`/`1e` input into an omitted seed. Clearing is unset and
omission remains absent. Submit is disabled for invalid seeds and its handler
checks again before creating an execution session. Existing unsafe saved numbers
are also refused, including when connected later or through a second fan-out edge.

Generic floating-point NumberInputs retain their prior parser and numeric field.
Rust/API requests retain full u64, including `2^53`, `2^53 + 1` and `u64::MAX`;
only their existing test was expanded to verify actual JSON text serialization.
No sampler, RNG/replay, worker, EOS/minimum/retry, scope-default or resident code
changes. The direct-edge UI guard does not supply an arbitrary-precision graph
representation or recover authored text already lost by historical rounding.

Fresh checks: 669 frontend tests pass (seven new precision methods), TypeScript,
changed-file ESLint, full Vite build, Rust format, critical anti-pattern,
accessibility, scheduler boundary and traceability gates pass. The expanded Rust
wire method passes; strict all-target inference Clippy passes. Chromium 151 runs
the actual component/helpers in fourteen groups: safe boundaries, rejected
`2^53`/`2^53 + 1`/`u64::MAX`, fractions, partial and nonnumeric text, visible errors,
verbatim persistence and repeated save/load/replay, omission/clearing, existing
unsafe numbers and unaffected generic floats. Its graph store and BaseNode shell
are fixtures; submission tests invoke the actual guard, not a Tauri session.
Independent Astra medium review found no remaining substantive issue after the
numeric-field sanitization correction. Usage metrics are unavailable.

The [hashed evidence archive](../evidence/text-seed-desktop-precision/README.md)
retains the original rounding failure and final component/wire results. An initial
Vite attempt used historical workspace package links; a worktree-local link map
corrected dependency resolution without installing or changing packages. Chromium's
dump-DOM mode supplied no output here; the final reproducible harness uses local
DevTools and runs the real component. Before Cargo execution the complete effective
all-target graph for the exact inference/PyTorch selection was inspected: it has
no ORT/Pumas dependency and no `download-binaries`. Current workspace manifests,
lockfile and dynamic-ORT configuration are unchanged. Cargo uses locked offline
dependencies and `ORT_SKIP_DOWNLOAD=1`; no build binary download or network/auth
change occurred.

This is bounded desktop component/persistence qualification, not full production
desktop execution. Seed candidate CPU qualification remains frozen with its stated
limits; GPU, pretrained/custom models, production loading, full worker import and
ONNX remain unqualified. PR58 source integration is complete; parent publication
and hosted/postmerge CI are handled separately. No main mutation, PR creation or
merge through GitHub is performed by this task.
