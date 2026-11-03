# Behavior-preserving inference lint corrections

Fresh PR #24 Clippy job 111191777174 reached the inference crate and reported 54 findings. This bounded slice addresses 36 annotation/style findings: 29 redundant method must_use attributes whose return types already carry that obligation; equivalent ordered filter/map and contains checks; Option early return via ?; derived SupportTier default with Unknown retained; and three borrowed Path comparisons instead of temporary PathBuf allocations. Public signatures, type-level must_use, conversion/error policy, and runtime identity semantics remain unchanged.

The option regression now checks exact ordered unsupported diagnostics while a supported streaming option emits no issue. New tests retain Unknown's default serialized value and Path-component equality across inference/embedding/reranking matchers, including mismatched model rejection. Hosted checks explicitly run compatibility/model-contract suites and the matcher regression. Existing lifecycle builder/telemetry behavior is unchanged by removing redundant annotations.

Formatting and whitespace pass locally. No local inference compilation or executed new Rust tests are claimed. Independent review and fresh hosted qualification remain pending. Nine high-arity functions, the public image-planning enum, and eight large-error returns are intentionally separate structural/API proposals, not suppressed or silently modified.

Independent source review accepted tree cfc7168c7afd6904d00ea9818e67b8f3f17424cc, including diagnostic order/content, Unknown default, borrowed Path equivalence and retained type/Result must-use obligations. Fresh hosted tests and Clippy remain required.
