# Public CPU text workflow observations

The public workflow now has a reusable probe and CI runtime-effects observation.
T1/T2 remain verified on current main plus the reused PR75 adapter. Actual
untrained CPU text generation passes. T3 and T4 remain blocked: trained assets
are unavailable, and the public terminal result loses failure/cancellation meaning.
This draft is not qualified for a ready merge.

| Observation/check | Result |
| --- | --- |
| Pumas catalog -> descriptor -> graph Resolve/Check -> published validation -> scheduler -> CPU PyTorch -> text-output | nonempty completion, `:::eeeeeeeeeeeeeeeeeeeee` |
| Corrupt disposable weights after resolution | actual `pytorch_worker_model_load_failed`; header deserialization: header too large |
| Active task cancellation | public Running/ack identity matches cancelled node and terminal attempt; early cancellation before generation |
| Cleanup in all three observations | session closes; shutdown returns; queue empty; no instance/models/reservations or uncertain residency |
| Public failure/cancellation result | **unqualified**: both become generic `InvalidRequest` / `TerminalFailed` |
| Focused adapter regressions | 7 passed |
| Embedded-runtime library | 619 passed; 3 ignored |
| Inference library | 853 passed |
| Clippy, both owners/lib/tests and new example, warnings denied | exit 0 |
| Affected rustfmt, Bash syntax, YAML parse, staged traceability, diff whitespace | pass |

`probe_passed` means the selected runtime effect and cleanup were observed.
`integration_passed` and `public_result_projection_qualified` remain false for
the two negative cases. These are intentionally distinct from observation/CI
success. Neither negative API failure is silently classified as a good public
cancellation result.

## Exact material and reproduction

Baseline main: `4aec0d5c4927a6af2832d40e224be5c2136fc9ef`. PR75 adapter source
`2f1e38fd846ae089b47c5a2ba4dccf1fcf892cc1` was selectively reused as
`7a78625bb9cb18dd01462b4d5295583ef1b787cf` without rewriting the original branch.
The source was reverified on current main; existing PR75 remains untouched.
Source hashes, outcomes and evidence are in
[source-and-results-2026-10-19.json](source-and-results-2026-10-19.json) and
[evidence/2026-10-19](evidence/2026-10-19/). Evidence copies contain only isolated
synthetic-probe JSON/logs, never operator catalogs or trained weights.

Use the [runbook](../../../cpu-text-generation-workflow-probe.md).
The script requires a clean checkout at its explicit expected SHA, creates new
per-scenario XDG/catalog roots, builds the example and runs all three observations.
It never installs packages, downloads weights or fabricates readiness. The new
CI workflow prepares CPU prerequisites explicitly and retains evidence even on
failure. A green runtime-effects job cannot satisfy the missing acceptance gates.

Local checks used the saved Rust 1.92.0/Python 3.12.3 toolchain, two Cargo jobs,
no incremental compilation, CPU-only Torch, explicit linked-Python library and
venv site-packages, and dedicated writable XDG roots. The full embedded binary
was the executable built by the focused Cargo test command, then run without a
filter; see the machine record for its exact filename. Reproduction commands
for the affected suites remain in the previous handoff report.

## Blockers and scope

The official exact DistilGPT2 config endpoint returned proxy CONNECT HTTP 403
normally and under an approved read-only escalation. No weights were acquired.
This was a network proxy denial, not automatic approval-review rejection or a
license refusal. The missing input is a verified local safetensors/config/tokenizer
asset set plus authoritative upstream revision/license evidence. An equivalent
trained licensed standard local causal LM within the same CPU/no-custom-code/
600 MiB contract could satisfy T3 with its own identity. The committed randomly
initialized model cannot substitute for trained-model evidence.

The normal setup issue was resolved using official PyPI: SoundFile 0.13.1,
cffi 2.1.1 and pycparser 3.11 installed. All seven worker prerequisites import;
`uv pip check` passes; Torch reports CUDA absent. No product dependency or lockfile
changed (the Cargo edit only registers the example).

Actual negative runs exposed a separate shared-owner defect:
`workflow/runtime_branch_run_finalization.rs` replaces non-completed task causes
with a generic final-state error. Related batch finalization shares this semantic
family. The canonical runtime ledger still records the exact loader failure and
cancellation acknowledgment. This slice records the issue and leaves shared
production finalization/fairness files unchanged for parent/M3 coordination.

Initial probe runs exposed an excessive ledger query limit and invalid direct
inference-node external bindings. Those probe defects were corrected: use the
owner's 500-event bound and route controls through public source nodes. Review
also corrected negative-result identification, dirty-source admission, terminal
failed-history cleanup and report-write cleanup ordering. Final observations use
the corrected source; initial failed attempts are not acceptance evidence.

Independent GPT-6.1 Sol / High `text_review` admitted the design, reviewed source
and actual negative evidence, and required distinct runtime-observation versus
public-result qualification. Final evidence review is recorded in the ledger.

Full text-to-image qualification still needs verified licensed text and image
models, the public composed text -> image execution and actual generated image
artifacts/output presentation, plus failure/cancellation/cleanup at those real
boundaries. Existing CPU embedding/image presentation/composition work is not
repeated. No GPU, native desktop UI, model quality or performance qualification
is claimed.
