# Image controls with reviewed runtime correctness

This separate candidate preserves the frozen publication and feature branches.
Normal merge `a894ec69a8743d11aa9467a53d694142be581f72`, tree
`888b3c577446fa6a4b77da14d9154f1b07831fb8`, has exactly these parents:

- Scheduler `af4174a4893c984d26a41ba3935a8555340978c5`.
- Reviewed correctness `13d24e898abd4175eb799ed310d1d275537880e9`.

Guidance `14dfd4a7e1d48a42dab9f08bae54224fea835a28` and image count `3fe88472`
are already scheduler ancestors. They were not cherry-picked or merged twice.
The common ancestor is `ab7a1b4e`. All source merged automatically; only the
plan's next action and appended ledger entries needed reconciliation, retaining
both histories. Parent owns independent review, PR56 and publication decisions.

Test source successor `12728772a8ff81d9356c358a60b8d84c4bc8b3ad`, tree
`6e920d3d63624d2901834d7223543b140a00de29`, parent `a894ec69`, adds no production
behavior. It adds an actual public-session/Pumas host regression for guidance
zero/DDIM and guidance 7.5/Euler, each with three retained image outputs. The
existing worker-envelope regression now supplies all three controls in solo and
batch requests and checks their actual pipeline kwargs. Malformed guidance,
count and scheduler cases include valid values for the other controls, proving
that invalid input is refused before gateway dispatch and media writes. Existing
all-omitted public sessions preserve unset fields and singleton output behavior.

The combined owner also executes the reviewed ordinary/selected/supervised
generation regressions, unknown-pool VRAM-only admission, real atomic config-save
failure and acknowledged child termination tests. Full peak claims, resident
uncertainty, source/instance fences and explicit zero remain conservative.

## Executed qualification

All Rust commands use `--locked --offline`. The isolated target is
`/workspace/pantograph-cache/image-controls-integration-target`. Its copied cache
was cleaned of every workspace package before qualification; only external
artifacts were reused. No scheduler/correctness worktree binary counted as a
combined result. Logs show both branch-specific generation tests and the new
combined-control/public-session tests executed from this worktree.

| Scope | Result |
| --- | --- |
| Full inference package, mixed llama.cpp/PyTorch | 812 passed including a doctest; six optional native/doctest cases ignored in this run. |
| Full embedded runtime package | 512 passed. Public sessions, all-omitted defaults, combined controls, malformed parameter refusal and resident/custody host regressions execute here. |
| Runtime registry | 142 passed. |
| App config | 14 passed; the outer suite reports one ignored subprocess helper, which its parent regression actually executes. |
| Interface/runtime-host contracts | 75 passed. |
| Workflow-service library | 916 passed. |
| Explicit native CPU | Three passed on Torch 2.14.1+cpu/Diffusers 0.37.0, zero pretrained models. Actual scheduler steps, guidance arithmetic and count/seed behavior are qualified by tiny random UNet oracles. |
| Frontend | 661 passed; TypeScript check passed. |
| Strict Clippy | Seven affected packages, all targets, mixed backends, `-D warnings`: passed. |
| Boundaries/gates | Scheduler-only public execution surface, formatting, critical, accessibility (27 tests), source/documentation/range traceability and nine no-build-download target/feature graphs passed. |

The public host/Pumas/backend tests are controlled execution fixtures. Native CPU
oracles are actual Diffusers computations but do not run the production pretrained
loader or qualify generated-image quality. Restricted local loading and worker
protocol tests run in the inference suite. Package pins and the dynamic/no-build-
download ONNX contract are unchanged. No native ONNX-model or GPU claim is added.

The actual desktop prerequisite probe fails: glib-2.0, gtk+-3.0 and webkit2gtk-4.1
are absent. Full Tauri execution/linkage and the desktop image presentation remain
unqualified. Cross-platform execution and physical capacity evidence are separate.
No credentials/permission changes, access-denial workaround, warning suppression,
review CLI, PR action or publication-branch mutation is included.

Evidence and exact final identities are under `/workspace/qualification-evidence/`
with prefix `image-controls-integration-`. The resumption manifest records source,
merge parents and cache preparation; the final qualification manifest records
completed checks, hashes and verified remote refs. No actual service-limit failure
has occurred and no test was interrupted. The documentation-only final successor has identical source to `12728772` and
is published as a separate candidate for independent review after gates. The
frozen PR56 and feature branches remain untouched.
