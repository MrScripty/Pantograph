# Text repetition numeric correction

This bounded successor starts at frozen feature
`2a6f7cf56bd7e6230f8feab1472efb237eec7431`. That feature and its original
qualification evidence remain unchanged. Parent owns PR, review, integration and
merge; the independent image/main candidate is not part of this branch.

Actual CPU probes confirmed three review findings. Minimum-positive-f32 penalty
on float32 logits `[2,4,1,0]`, history `[0,1]`, produces positive infinities and NaN
softmax. Penalty `0.0001` on fp16 `[10,20,2,1]` similarly overflows, while float32
promotion produces finite `[100000,200000,2,1]`. Ordinary fp16 `[1,1.2,0,0]`,
history `[1]`, penalty `1.2`, rounds the adjusted repeated score to one and selects
token zero; promoting the stored fp16 score produces `1.0001627` and selects token
one, as native Transformers 4.53.3 does.

Manual streaming and SDAR fresh/suffix/replay decoding now promote logits to
float32 before the official repetition processor. A checked wrapper rejects
non-finite inputs except existing negative-infinity masks, any newly non-finite
adjusted score, a mask becoming anything other than negative infinity, and rows
without a finite selectable score. Existing masks from earlier native processors
remain supported. No scalar clamp, invented bound, float64 fallback or repaired
logit is introduced. Cached SDAR numeric refusal propagates instead of retrying
with different history; the empty-output native retry uses the same guard.

Native generation uses a request-local shallow model copy with shared parameters.
Only that copy's processor builder is wrapped. The resolved official repetition
processor is replaced at its original position, preserving earlier sequence bias,
later sanitizers/suppression/warpers, and omitted model defaults. Passing a public
custom processor would run too late and allow sanitization to conceal overflow.
Generation that bypasses the expected Transformers builder fails closed. Tests
exercise an actual Torch module and verify shared parameters plus unchanged
resident method/configuration on success and refusal.

The public authored scalar domain remains positive finite f32, including its
minimum and maximum. Both extremes still work where the actual logits yield
defined computation. The behavioral contract is stricter: a valid scalar may
produce a numeric refusal after model logits become available. Batch worker
envelopes report `invalid_request`; streaming raises before emitting an affected
token. Scalar validation still precedes inference, but operational validation
necessarily follows the model forward pass. No claim is made that every positive
penalty can be applied to every finite logit tensor. Explicit one remains neutral.

## Qualification

Twenty-three actual CPU tests pass with Python 3.12.3, Torch 2.14.1+cpu and
Transformers 4.53.3, using fixed logits without pretrained models or downloads.
They cover fp16/bfloat16/f32 native/manual parity, greedy and stochastic routes,
extreme positive and negative overflow, defined scalar extremes, mask integrity,
native processor order despite invalid-value removal, unsupported native builder
bypass, actual worker envelopes, and SDAR fresh/cache/suffix/replay/retry paths.
The new parity regression also fails against the original frozen Python source.
An initial test incorrectly expected the fp16 winner in the separate f32 tie case;
its dtype-specific expectation was corrected before acceptance. Read-only review
found the tiny-model-default mask-to-NaN case, which is fixed and covered.

The full inference package passes 811 checks with three ignored native/doctest
cases using the mixed llama.cpp/PyTorch portable profile. Strict all-target
inference Clippy, format, critical/accessibility and scheduler-boundary checks,
staged/range traceability and nine ONNX no-build-download graphs pass. Existing
worker import fixtures received only the two new import sentinels; they do not
qualify native sampling. No frontend or embedded-runtime source changed, and
their frozen-parent suite results are not counted as reruns here.

Cargo uses private `/workspace/pantograph-cache/text-repetition-numerics-target`;
all workspace artifacts were removed before compilation, retaining only
hardlinked external dependency artifacts. Frozen qualification targets remain
unchanged. Exact identities, commands and hashes are recorded separately in
`/workspace/qualification-evidence/text-repetition-numerics-qualification.json`.
Native GTK/WebKit, GPU, pretrained-model quality and native ONNX execution remain
unqualified. Full worker import still lacks soundfile; worker entry tests execute
the actual AST-selected functions with controlled services. Native SDAR model
loading is unqualified. No external Core/Router reinspection is claimed because
the referenced standards directory remains unavailable.
