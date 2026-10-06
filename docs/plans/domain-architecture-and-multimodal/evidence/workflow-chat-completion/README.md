# Local chat qualification evidence

Tested implementation: `2189046a6a0a418b54444308028b1af27323bf5a`, tree
`738e25da47f9cf04ba8be0cea2a476aaf44da327`. The separate evidence commit binds this source and its
exact delta from composition `fff03a153ff26b5a38c01be8a3cc65ddb28be07e` in
`source-binding.json` and `source.patch`. `manifest.sha256.json` checks every
packet member. The final candidate may add only these evidence files after that
tested implementation commit.

Deciding logs cover the full affected Rust suite, explicit actual CPU saved-graph
qualification, full Python worker qualification and regression suite, strict
Clippy, formatting, frontend tests/typecheck/build, root gates and scheduler-only
execution. `features.txt` records the complete all-target effective Cargo feature
graph. `commands.sh` records the no-download/offline reproduction route.

Initial failed fixture/harness/Clippy and unspecified-input traceability runs are
retained as diagnostics; they are not deciding passes. Controlled package,
readiness, dispatch and graph-discovery facts are distinct from actual native
local loading, full worker imports and synthetic GPT-2 CPU forwards. This packet
does not establish live Pumas, pretrained/GPU/desktop, post-start native
cancellation or general production-model qualification.

See the [feature report](../../reports/2026-10-06-workflow-chat-completion.md) and
the [fixture instructions](../../../../../crates/inference/tests/fixtures/tiny_chat_gpt2/README.md).
