# Graph-authored minimum new tokens

The parent authorized the next useful inference/scheduler capability after lifting
the commit hold following verified local Git identity correction. This successor
starts at frozen KV refusal checkpoint `59992b9b32a11ab522f3b0816b99240d65c1d458`.
The existing typed `LengthGenerationOptions.min_new_tokens` was not exposed through
selected text execution. The descriptor now appends an optional integer input,
with zero allowed and no invented default, and projects it through ChatRequest,
the typed gateway, actual host port and PyTorch worker envelope.

An authored floor must fit the explicit positive u32 token budget or PyTorch's
existing 512-token fallback. Shape/range/budget failures precede model load,
formatting, dispatch and KV mutation. Other backends report the existing support
diagnostic; masked block diffusion refuses authored minimum, including zero.
Omission retains the model's generation default. The floor counts new generated
tokens, not prompt, cached context, suffix tokens or decoded characters.

Native Transformers retains its original processor order and uses the official
minimum-new-token EOS processor. A final request-local check refuses active-floor
conflicts from later suppression or forced-EOS processors, preserving their order.
Manual streaming and SDAR use the same processor
after the guarded repetition operation and before sampling. Model EOS sets,
including token zero, are respected. Cached continuation starts counting after
suffix ingestion or replay. The existing empty-output retry preserves a larger
authored/model floor. If EOS suppression leaves no finite eligible token, decoding
explicitly refuses; cached refusal clears the uncommitted live KV snapshot and
does not automatically retry fresh. Full task peaks, resident ownership, uncertainty,
known zero and source/generation fencing are unchanged.

Executed qualification with Python 3.12.3, Torch 2.14.1+cpu and Transformers 4.53.3:
32 CPU test methods pass, exercising real GenerationMixin, fixed logits, native
and manual sampling, model defaults, EOS sets, greedy/stochastic modes, SDAR
suffix/replay, retry, later processor conflicts and real DynamicCache refusal.
Read-only peer review identified the later-suppression counterexample; its actual
native regression passes after adding the final check. The actual public scheduler
connects a Number Input to the new port, preserves companion controls and retains
generated text. Actual host/gateway and both raw worker operations cover omission,
zero, equal/default budgets, u32 bounds and invalid inputs without backend effects.
The full mixed llama.cpp/PyTorch qualification passes 815 inference tests plus one
doctest (three native/doctest cases ignored), 508 embedded-runtime tests, 24
interface-contract tests and 661 frontend tests. Strict all-target Clippy for the
three affected Rust packages, typecheck, format, lint, critical/accessibility gates,
scheduler public boundary, Python compilation and nine ONNX no-build-download
dependency graphs pass. Staged/range traceability is checked with publication.

The first Rust link attempt failed with a bus error when the filesystem filled;
it is not qualification. Reversible generated workspace artifacts in prior private
Cargo targets were cleaned while retaining Git checkpoints, original evidence and
external dependencies. The final suites and Clippy then completed successfully in
`/workspace/pantograph-cache/text-min-new-tokens-target`. Exact source identities,
commands, logs and hashes are recorded in
`/workspace/qualification-evidence/text-min-new-tokens-qualification.json`.

Tests execute controlled models with actual tensors, processors and caches; they
do not qualify pretrained SDAR/custom generation implementations. Full worker
import still lacks soundfile, so worker tests execute actual AST-selected functions.
Native GTK/WebKit, GPU, pretrained-model and ONNX execution remain unqualified.
The external standards directory is unavailable; no fresh Core/Router inspection
is claimed. Parent owns PR/review/merge; no PR54/55 changes are made.
