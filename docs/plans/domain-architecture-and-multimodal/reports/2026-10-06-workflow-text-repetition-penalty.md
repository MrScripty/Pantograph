# Graph-authored text repetition penalty

The next independent text capability exposes the existing typed
`sampling.repetition_penalty` through the selected PyTorch text owner. Source
inspection confirmed that token limit, system prompt, temperature, top-k, top-p
and the basic image controls are already implemented. This branch starts at
reviewed correctness `13d24e898abd4175eb799ed310d1d275537880e9`; it does not advance
the frozen config correction or combined image-controls review candidate.

The backend-owned text descriptor adds optional F64 `repetition_penalty`, bounded
by the smallest positive f32 and f32 maximum, with no step or default. The host
uses the existing checked scalar conversion and rejects wrong types, zero,
negative values, overflow, underflow and lost authored precision before resolving
or loading model dependencies. Canonical typed request validation also rejects
non-finite and non-positive values before serialization or cold load; this avoids
NaN becoming JSON null and silently selecting a default.

The existing chat adapter and allowlisted worker kwargs preserve the option.
Omission leaves the model generation default in charge; explicit one is neutral.
Values above one discourage repeated tokens and values below one encourage them.
Transformers generation uses its native option. Manual streaming and SDAR fresh
or continued decoding use the official RepetitionPenaltyLogitsProcessor against
the complete prompt, generated and cached token history before greedy selection,
temperature, top-k and top-p. The empty-output retry retains the authored value
or model default. Masked block diffusion rejects authored penalties before cache
mutation or inference because that route does not implement them.

PyTorch diagnostics report the mapped option. Other backends retain a
requires-backend-support diagnostic; their repetition behavior is not qualified.
The obsolete separate scalar-argument worker request helper is replaced by the
existing owned PyTorch request struct. No new scheduler schema, wire version,
dependency pin, resource discount or physical-device assumption is introduced.

## Qualification boundary

The actual public scheduler session exercises omitted and authored penalties
through a connected Selection Input source, the embedded batch host and typed
gateway, preserving the retained text result. It uses the production
descriptor-to-authored-snapshot projection; descriptor-only `options` are not
copied into authored snapshots. Fractional values use the existing selection
source path; scheduler Number Input materialization remains integer-only. Deterministic host and selected-gateway tests verify exact optional values
and existing companion controls, plus malformed refusals with no backend effects.
Worker envelope checks cover both generation operations and invalid values.

Thirteen actual CPU tests pass with Python 3.12.3, Torch 2.14.1+cpu and
Transformers 4.53.3, without pretrained weights or downloads. Real Transformers
generation and manual sampling agree for positive/negative logits, penalties
below/equal/above one, omitted model defaults, greedy and stochastic paths, and
prompt plus generated history. Controlled SDAR cache/replay and retry cases
exercise the actual sampler and worker functions; they do not qualify loading
a native dLLM model. Worker entry tests execute the actual AST-selected functions
with controlled services because full worker import requires missing soundfile.

The private Cargo target is `/workspace/pantograph-cache/text-repetition-target`.
All workspace-package artifacts were removed from that target before compiling;
only hardlinked external dependency artifacts were reused. This avoids the
previously observed cross-worktree workspace binary ambiguity.

The final mixed-backend command
`cargo test --locked --offline -p inference -p pantograph-embedded-runtime -p pantograph-inference-interface-contracts --no-default-features --features backend-llamacpp,backend-pytorch`
passes 811 inference checks (including one doctest; three native/doctest cases
remain ignored), 505 embedded checks and 23 interface-contract checks. Strict
all-target Clippy for the same three packages and feature profile passes with
`-D warnings`. The first full attempt exposed two lifecycle fixtures missing the
new processor import; their import sentinels were updated without treating those
controlled lifecycle tests as native sampling evidence. The public session fixture
was corrected to use the actual snapshot projection and source materialization
contracts before its passing run. No failed fixture run is counted as feature
acceptance.
Frontend checking passes all 661 assertions and TypeScript. Formatting, lint,
critical/accessibility gates with 27 tests, the scheduler-only public boundary
check, staged/committed-range traceability and all nine ONNX no-build-download
feature/target graphs pass. Final identities, raw logs and hashes are recorded in
`/workspace/qualification-evidence/text-repetition-qualification.json`. Native
GTK/WebKit, GPU, pretrained-model quality and native ONNX execution remain
unqualified. The external standards directory is unavailable; no fresh Core or
Router inspection is claimed. Parent owns integration, PR and review scheduling.
