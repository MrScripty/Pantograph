# Scheduler-selected rerank preservation candidate

This local successor adds CPU llama.cpp GGUF rerank execution through the
scheduler/runtime-host boundary while preserving both working direct core rerank
and audio-transcription routes. The removal experiment `63f7b4e4e97afe5fb22e9368265ac3013c7a5de9`
remains a separate unpublished candidate. This branch starts from main
`a8483e511dcec4f36e269e6e4debf181a318222f`; it does not compose that experiment.

## Selected slice and contracts

Rerank is the smaller slice: it already has a typed query/document request and
five outputs (`results`, `scores`, `top_document`, `top_score`, `diagnostics`).
The existing llama.cpp adapter already implements `/v1/rerank`. Audio additionally
needs owned media materialization and audio-specific worker/load contracts.

The production descriptor provider now exposes required string query and JSON
documents, optional positive `top_n`, return-documents flag and JSON task/backend
options, and the five retained outputs. No authored default or streaming port is
introduced. Connected first-class controls override nested task options. Document
strings and text/content/document objects retain parent parsing and blank filtering;
the direct host additionally recognizes the existing documents_json and control
aliases. Invalid controls fail explicitly instead of silently selecting defaults.

The additive runtime-host JSON input variant has a 64 KiB serialized bound.
Workflow materialization admits structured JSON only for rerank documents,
task_options and extra_options; existing scalar/numeric/media mappings and other
task restrictions remain. Existing 1,024-byte scalar text and 64 KiB structured
output limits still apply. Oversized output fails visibly and is not truncated.

Selected execution validates requested/selected/package/target model, artifact
and explicit revision constraints, current package contracts, valid local GGUF
file/storage facts, canonical rerank task evidence, and exact CPU runtime/device
agreement before effects. Custom model code, missing targets, unsupported owners,
GPU variants, nonempty runtime traits, streaming, generation controls, backend
options that replace canonical identity/inputs, and malformed inputs fail
explicitly. A ready generic backend cannot substitute for selected loading.

The loader maps canonical CPU to llama.cpp's local `none` selector and zero GPU
layers. It uses the separately resolved executable file and an existing process
spawner. There is no prompt/chat fallback, implicit model resolution or runtime
acquisition. Runtime reuse requires the same known complete load target identity;
a matching path with changed model/revision/artifact/content facts requires
replacement. Generic starts cannot establish selected rerank ownership.

The gateway moves backend custody into the owned operation. Host cancellation
suppresses completed output after actual backend completion; dropping the caller
signals cancellation but cannot release custody early. This is completion-observed
cancellation, with no promise of a bounded interrupt for a nonreturning HTTP owner.
Producer/start failures remain failures. Returned scores must be finite and
indices in-range and unique. Sequential scheduler envelopes preserve each member's
execution/assignment/workflow/run/node/task identity; native batching is not advertised.

## Acceptance and limits

Tests compare the retained, unchanged `CoreTaskExecutor` route with the selected
host using the same synthetic backend, query/document parsing, controls, nullable
outputs and option diagnostics. They exercise exact target/model/revision/runtime/
device capture, wrong identities and requested revisions, unsupported selected
loading despite generic readiness, invalid scores/indices, streaming refusal,
in-flight cancellation and caller abort while the actual owner is pending.

The public session test saves and reloads graph documents and authored descriptor
snapshots, then executes reopened graphs through the scheduler and production
selected host/gateway for two model/revision identities. It compares all five
outputs against actual retained-parent execution. Alternate host runtime-load/run
hooks remain unused. Descriptor tests compare the saved fixture to production
port generation, not merely hand-authored fixture acceptance. A production
llama.cpp loader test records the exact target, CPU and rerank command arguments
through a refusing synthetic spawner without launching or fetching a runtime.

This is synthetic structural and behavior qualification. It does not establish
native GGUF numerical/semantic quality, live Pumas discovery, pretrained/custom
models, GPU, desktop display, throughput or prompt-bounded cancellation. The
separate native inspector source/public run and its unqualified display result
are untouched. No model or ORT/runtime payload downloads, public writes or CI
operations are included.

## Preserved audio route and smallest next replacement

Audio retains the parent typed `AudioTranscriptionRequest` and eight outputs:
response, null stream, text, nullable language/duration_seconds, segments,
metadata and diagnostics. Its smallest coherent scheduler replacement would
select a supported PyTorch CPU HF audio package, resolve a path-free media artifact
through an owned bounded audio resolver, and use a selected audio-load method
before the existing typed transcription worker. The resolver must validate media
ownership/type/bytes without introducing arbitrary client paths. Preserve language,
timestamps, options and nullable results; validate finite duration and segment
bounds. Streaming, unsupported audio/package/device combinations and custom code
must reject explicitly. Compare the retained route and scheduler outputs with an
existing synthetic audio owner, cancellation/caller-loss custody and reopened
graphs before removing that direct handler. No audio implementation or retirement
is included in this rerank candidate.

## Reproduction

Use the pinned toolchain and existing cache, `ORT_SKIP_DOWNLOAD=1`,
`HF_HUB_OFFLINE=1`, `TRANSFORMERS_OFFLINE=1`, locked offline Cargo, an isolated
writable XDG configuration root, and the installed Python library/site-package
paths. The relevant package set is inference, node-engine, embedded-runtime,
runtime-host contracts, interface contracts and workflow-service. Enable existing
PyTorch/Candle/llama.cpp/std-process and embedded audio features to retain the
parent routes in regression coverage. Logs and the exact local commit/tree/source
patch are recorded under `/workspace/pantograph-scheduler-rerank-evidence-20261007`.

The final combined offline suite passes 2,842 Rust test cases and one doctest
with zero failures. Six opt-in integration/native cases and sixteen doctests
remain ignored. This includes both retained parent rerank successes, both retained
parent audio successes, the saved/reopened rerank graph, per-member envelope
identity, cancellation/caller custody, the production CPU launch argument check,
known-target reuse and reserved-option fences. ONNX policy passes all nine
supported target/feature selections; public scheduler-surface, formatting and
whitespace checks pass. No node-engine source, manifest or lockfile is changed.

Strict all-target Clippy passes for the six relevant packages with warning denial
and the same feature set. Two introduced lint findings were repaired: a nested
else-if and a test-only standard mutex guard lifetime. No lint allowance was added.
Independent review and any publication remain separate owner decisions.
