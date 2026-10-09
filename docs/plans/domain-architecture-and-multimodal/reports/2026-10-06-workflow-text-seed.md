# Graph-authored text sampling seed

The documented next inference slice was the existing typed text sampling seed.
Inspection confirmed that the canonical option/default resolver already carried
`sampling.seed`, while the text descriptor, actual host/chat projection and
PyTorch worker token sampler lacked its end-to-end graph route. This candidate
adds that missing route without changing scheduler ownership or default layering.

`feat/workflow-text-seed-on-pr58-repair` started at independently reviewed repair
`796d84cd94b7b45e54e14d36d8d1da7a069294b4`, tree
`8d704866a03a0c122b0b0bb27ff1af6dbf5917ac`. Its documentation-only successor
`d0c9788c5d5ba11ebdfe548f360f1c270e41acab`, tree
`e714ccb9508797268964a3c42190280d57b7b043`, was incorporated by normal fast-forward
before committing seed work. Both commits remain ancestors. The earlier paused
seed worktree and patch remain unchanged; reviewed composition/harness branches
are preserved. Main was independently recorded at `763e8d4b`; no main change,
PR creation, merge, resident-accounting work or old Pumas pin is included.
PR58's main merge and parent publication/hosted CI remain release dependencies.

The optional `seed` input is appended after existing text controls, retaining port
indices. It uses scalar u64 with no invented default or inexact floating-point
maximum. The host accepts u64 and nonnegative i64 inputs; invalid graph values
fail before backend effects. The actual host, typed gateway, chat edge and both
worker operations preserve omission, zero and full u64 boundaries. Scope-aware
controlled dispatch verifies caller-resolved model/workflow/runtime-preset defaults,
source attribution and explicit zero overriding inherited values. Connected graph
regressions execute the public scheduler/host route with seeds 0, 42 and u64::MAX.
Other backends retain `RequiresBackendSupport`; PyTorch reports forwarding with
worker route/device validation rather than a broad honored-generation claim.

Each accepted seeded worker request owns one generator on the actual token
sampling device. Manual streaming and SDAR loops use that generator; empty-output
native retry shares it within the same request. KV snapshots contain context/cache,
and each later request begins its own RNG from its authored seed. Replaying an
identical starting context and seed replays token draws on the qualified CPU path.
Omission retains ambient sampling; greedy generation consumes no seed draws.
No global reseeding, global state swap, resident sampler mutation or module-wide
Torch patch is introduced.

Native sampling retains installed canonical Transformers generation and processor
ordering. A request-local model copy resolves its exact generation configuration,
including legacy model-config defaults, and binds the canonical sample code with
a private multinomial receiver. Unsupported custom generate/sample/config routes,
beam modes, masked block diffusion and unsupported sampling devices explicitly
refuse seeded execution. Seed checking preserves PR58's tokenizer EOS stopping,
union minimum suppression, inherited native minimum/forced-EOS precedence and
strict authored-floor handling. Refused continuation/retry drops uncommitted KV.
Three additional seeded regressions exercise these repaired control interactions.

Fresh qualification results are preserved in the [hashed archive](../evidence/workflow-text-seed/README.md):

| Executed scope | Result |
| --- | --- |
| Actual Python CPU sampling/worker functions | 59 methods pass: original 32, eight PR58 regressions and 19 seed methods. |
| Inference with llama.cpp/PyTorch/Candle/std-process feature selection | 795 library and 74 integration tests pass, plus one doctest; four optional native cases and two doctests retain their existing ignored status. |
| Embedded runtime | 524 pass, including connected seed and actual host projection. |
| Inference-interface contracts | 26 pass. |
| Runtime-host contracts | 45 library and seven compatibility tests pass. |
| Workflow service | 916 library and 60 integration tests pass. |
| Frontend / TypeScript | 662 tests and typecheck pass. |
| Strict all-target Clippy | Five selected packages pass with `-D warnings`. |
| Other gates | Rust format, critical anti-pattern, scheduler public boundary, Python compilation, diff whitespace and staged/range traceability pass. |

These current inference counts are distinct from repair-only 747 library checks
and the earlier 821-check package qualification. The earlier paused seed suite's
two controlled lifecycle import failures required import placeholders for the new
Transformers symbols; both tests now pass. Those fixtures prove drain/order and
selection retention under stub dependencies, and do not establish model loading.
Independent Astra medium reviewed the complete integration against `d0c9788c`
and found no substantive issue. Implementation/review usage metrics are unavailable.

Before any build, the complete effective all-target feature graph for the exact
five-package selection was inspected. It uses Pumas
`26a84e323cae566a46a8f76bef48fa1010aed48b`, ORT `load-dynamic` and ort-sys
`disable-linking`; `download-binaries` is absent. Builds used `ORT_SKIP_DOWNLOAD=1`
and locked offline Cargo with existing dependencies. No manifest/lockfile change,
network/authentication change or build-time binary download occurred.

Qualification is limited to token selection with fixed CPU forwards and actual
installed Torch 2.14.1+cpu / Transformers 4.53.3 processors. Randomness inside
arbitrary model forwards/custom processors, cross-device/version equivalence,
production loaders, CUDA, pretrained/custom models, full worker dependency import,
desktop and ONNX execution remain unqualified. Worker tests select actual entry
functions through AST because full audio dependencies are unavailable. SDAR replay
uses controlled cache/forward fixtures rather than pretrained cache restoration.
Rust/JSON carries full u64; desktop NumberInput uses JavaScript Number and may round
values above `2^53 - 1`. This slice does not qualify full-u64 desktop entry.
