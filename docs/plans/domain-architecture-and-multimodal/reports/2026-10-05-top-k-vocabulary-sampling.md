# Top-k vocabulary-bound sampling repair

Independent review of frozen top-k composition
`bc655d3458757766a224531e4a8bbea02953a467` found that a valid graph-authored
`u32::MAX` reached the manual sampler's unbounded `torch.topk`. The original
recording JSON and envelope evidence did not prove sampling. Earlier review
acceptance was provisional; top-k remains a candidate for parent review.

This separate milestone on `fix/workflow-top-k-vocabulary-bound` keeps the
zero-inclusive graph contract and caps positive k at the actual logits
vocabulary width inside `autoregressive._sample_next_token`. It matches the
installed Transformers `TopKLogitsWarper`: k above vocabulary retains all
tokens. The existing sampler serves standard streaming and SDAR cached decode
and continuation; no scheduler or wire variant is changed.

Source inspection also found that non-streaming `_generate_autoregressive`
dropped explicit zero from `model.generate` kwargs, allowing the model's
configured default to survive. Explicit zero is now sent as zero; omission
retains existing model-generation-config resolution and omission behavior.

`python -m unittest discover -s crates/inference/torch/tests -v`: four tests
passed using installed CPU Torch `2.14.1+cpu` and Transformers `4.53.3`:

- Real small batched logits, including ties, compare actual probability tensors
  and actual multinomial draws against Transformers top-k/top-p warpers for
  zero, in-vocabulary values, k above vocabulary and `u32::MAX`.
- Greedy sampling retains argmax with oversized k.
- Streaming through a fixed-logits model fixture matches disabled filtering
  when k exceeds vocabulary, and omission matches the configured k.
- A non-streaming recording model with real tensor inputs verifies exact zero,
  positive/max values, absent defaults and configured defaults in generate kwargs.

The same tests run against the frozen original sampler detect both oversized-k
errors and missing explicit zero. Logs: `/tmp/top-k-real-sampler-tests.log` and
`/tmp/top-k-sampler-baseline-regression.log`. No model weights, pretrained-model
inference, GPU, ONNX download, runtime substitution or new Python dependency
was used. This tests sampling, not model quality or full graph acceptance.

The separately published PR54 fixture successor is
`174c1950312150512e917cab5b0027551fab786b`, tree
`211d02eb62c3285598a3edcdd7437a22f574ed3a`. Parent-hosted exact-head execution
of its two public session scenarios remains required; see the
[published fixture report](https://github.com/MrScripty/Pantograph/blob/174c1950312150512e917cab5b0027551fab786b/docs/plans/domain-architecture-and-multimodal/reports/2026-10-05-pr54-session-output-discovery.md).
Parent owns independent review and PR publication. Frozen feature evidence
and repair histories are retained separately before composition.
