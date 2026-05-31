# crates/node-engine/src

## Purpose
This directory contains Pantograph's workflow execution and descriptor core. It
turns execution descriptors into runnable behavior, validates execution/runtime
inputs, and keeps execution dispatch aligned with descriptors published by
`workflow-nodes`. Canonical graph-authoring contracts are owned by
`pantograph-node-contracts` and projected through workflow-service.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `builder.rs` | Engine construction helpers and composition wiring. |
| `composite_executor.rs` | Executor composition for multi-stage task execution. |
| `core_executor.rs` | Main node-type dispatch, dependency-aware execution, and payload normalization. |
| `core_executor/` | Focused core-executor helper and test modules behind the stable executor facade. |
| `descriptor.rs` | Node descriptor contracts consumed by the graph and runtime layers. |
| `engine.rs` | Workflow engine entry points and orchestration helpers. |
| `engine_tests.rs` | Shared workflow engine facade test fixtures and behavior-module index. |
| `engine_tests/` | Focused workflow engine facade tests for cache state, demand execution, workflow events, multi-demand behavior, human input, and snapshot projection. |
| `engine/` | Focused graph-event and multi-demand helpers behind the stable engine facade. |
| `error.rs` | Shared engine and execution error types. |
| `events.rs` | Stable facade for workflow event contracts and sink implementations. |
| `events/` | Focused event contract, sink, and test modules behind the stable facade. |
| `extensions.rs` | Extension points used to add engine behavior without mutating the core API. |
| `groups.rs` | Group/node graph helpers. |
| `model_dependencies.rs` | Model dependency typing used by execution preflight and runtime selection. |
| `orchestration/` | Orchestration-specific execution and state modules. |
| `path_validation.rs` | Validation helpers for file and model-path inputs. |
| `port_options.rs` | Port metadata helpers used by graph editing and execution, including validated provider query context and typed disabled/unavailable rows for fact-aware option lists. |
| `registry.rs` | Built-in node registration, descriptor inventory, and callback-executor type boundaries. |
| `single_task.rs` | One-task core execution API that owns `graph_flow::Context` and empty extension setup for scheduler-owned non-runtime adapters. |
| `tasks/` | Task metadata and task-oriented helpers. |
| `types.rs` | Shared workflow graph and runtime DTOs. |
| `undo.rs` | Undo/redo support for workflow graph editing. |
| `validation.rs` | Graph validation and invariants. |

## Problem
Pantograph needs one execution layer that understands workflow node contracts
without hard-coding frontend assumptions. As node types expand from generation
and embeddings into reranking, execution dispatch must preserve semantic
boundaries instead of forcing new workloads through incompatible legacy paths.

## Constraints
- Node descriptors feed execution and canonical contract projection, so runtime
  assumptions must stay append-only.
- Execution helpers must tolerate heterogeneous port payloads while still
  normalizing them into typed backend requests.
- Task-type inference drives dependency/runtime selection, so incorrect
  classification can start the wrong engine mode.
- Built-in dispatch must fail explicitly for disabled node behavior instead of
  synthesizing successful placeholder outputs.
- Image-generation runtime execution is scheduler-owned. Node-engine may keep
  request-shaping helpers for validation, but it must not launch image
  generation through a planned-inference host or host-installed executor
  extension.

## Decision
Keep `core_executor.rs` as the single dispatch boundary for built-in node types
and normalize node inputs there before handing them to downstream runtimes.
Reranking and embeddings therefore enter execution through canonical
`llm-inference` task kinds with dedicated document parsing, embedding checks,
and task classification instead of preserving backend-specific graph node
types. Retired backend-specific inference node names are only accepted as
stale-node guards that return migration errors. `engine.rs` also remains the
backend-owned source for
graph-mutation and incremental-demand workflow events, so adapters only
translate emitted execution facts instead of inferring graph-change semantics
locally.
Graph-authoring, GUI, and binding contract semantics are not owned here; they
are resolved through `pantograph-node-contracts` projections.
`DependencyEnvironmentSidecar` is an exact-only association marker for
workflow-service subject resolution. It is not executable runtime data and must
not be coerced through `any`, `json`, or `component` ports.

For scheduler-owned workflow execution, `single_task.rs` is the only supported
one-task core execution plumbing. It requires an explicit node type from the
immutable task definition, injects that type into the core input shape, creates
local graph-flow context plus empty executor extensions, and executes one
host-independent core task. It is not a scheduler allowlist, demand executor,
runtime inference launcher, Pumas selector, or workflow-session API. Callers
must reject runtime inference, `puma-lib`, file I/O, and unsupported node kinds
before constructing a single-task request.

## Alternatives Rejected
- Reusing the generic llama.cpp inference node for reranking.
  Rejected because reranking expects query-plus-documents semantics and ordered
  scored output, not prompt completion.
- Letting the frontend classify reranker models independently.
  Rejected because runtime mode selection must stay backend-owned.

## Invariants
- Built-in node dispatch in `core_executor.rs` must match descriptor inventory
  published by `workflow-nodes`.
- Core executor behavior tests stay in `core_executor/tests.rs` so production
  dispatch and helper extraction can proceed without growing the facade file.
- Synchronous built-in node handlers stay in `core_executor/pure_nodes.rs` so
  payload normalization for pure nodes remains separate from runtime-backed
  adapters and dispatch wiring.
- Core executor pure processing handlers stay in
  `core_executor/processing_nodes.rs` so validation and JSON path extraction do
  not grow the input/output passthrough module.
- Core executor model payload handlers stay in `core_executor/model_nodes.rs`
  so Puma library contract projection remains separate from generic pure nodes.
- Core executor file I/O handlers stay in `core_executor/file_io.rs` so host
  path validation remains separate from pure node payload normalization and
  runtime-backed adapters.
- Core executor settings expansion and optional-input readers stay in
  `core_executor/settings.rs` so runtime-backed adapters share one schema
  default and port-override normalization path.
- Core executor dependency preflight stays in
  `core_executor/dependency_preflight.rs` so model-reference construction,
  backend-key normalization, and resolver readiness checks remain separate from
  dispatch and runtime request execution. Explicit workflow/backend inputs are
  the only graph-owned backend signal; resolved Pumas package facts may supply
  factual compatibility context, but node-engine must not treat retired package
  facts or path fields as an alternate graph model-selection contract.
- Retired backend-specific inference node types in `core_executor.rs` must
  remain outside the live executor path. Saved graph upgrades are owned by
  workflow-service canonicalization; new runtime-backed behavior must enter via
  canonical `llm-inference` task/runtime evidence.
- Gateway-backed inference execution stays in `core_executor/inference_nodes.rs`
  so canonical task request/projection handlers and unload-model behavior
  remain separate from Python-worker adapters. The retired direct
  `vision-analysis` HTTP path must not bypass canonical `llm-inference`
  image-understanding task contracts.
- Scheduler-owned non-runtime execution enters node-engine through
  `single_task.rs`, not `DemandEngine`, workflow sessions, planned-inference
  host extensions, or caller-constructed graph-flow context. The single-task API
  must fail closed if explicit node-type authority is missing or contradicted
  by core resolution.
- Llama.cpp completion execution stays in `core_executor/llamacpp_nodes.rs` so
  completion streaming and KV-cache capture are isolated from the remaining
  gateway-backed inference adapters.
- Retrieval inference execution stays in `core_executor/retrieval_nodes.rs` so
  reranking document parsing and embedding compatibility checks remain separate
  from text/chat adapters.
- Stable Audio Python-worker execution stays in
  `core_executor/audio_nodes.rs`. Node-engine PyTorch launch is retired:
  successful PyTorch execution must come from scheduler task state/results and
  runtime-host responses, not a node-engine Python-worker adapter.
- Task-type inference must reflect execution semantics, not UI naming.
- Input normalization may be permissive for additive compatibility, but output
  shapes must stay stable once published.
- `PortDataType` remains execution descriptor input. Graph-authoring
  compatibility is projected through `pantograph-node-contracts::PortValueType`
  so GUI and binding consumers do not duplicate node-engine rules.
- Boolean setting readers in `core_executor/settings.rs` remain an
  inference-node helper contract; audio-only builds may compile that module,
  but they must not force those inference-only readers to count as live
  production paths.
- Graph mutation and incremental execution events must be emitted from executor
  state transitions, not synthesized by frontend or transport adapters.
- Workflow engine execution, graph mutation, event emission, cancellation, and
  human-input tests stay indexed by `engine_tests.rs`, with behavior coverage
  split under `engine_tests/` so `engine.rs` remains focused on production
  orchestration helpers and facade methods.
- Multi-demand planning, dispatch-window, bounded-parallel execution, failure
  attribution, and result aggregation tests stay in
  `engine/multi_demand_tests.rs` so `engine/multi_demand.rs` remains focused on
  production multi-target demand coordination.
- Execution events may carry additive `occurred_at_ms` timestamps, and adapter
  layers must preserve those backend-owned producer times when projecting trace
  or diagnostics state instead of restamping them locally.
- Registry callback executors should keep async and sync callback signatures
  behind local type aliases so FFI-facing registration stays reviewable without
  growing complex inline function types.
- `tool-executor` dispatch is disabled until backend-owned tool execution
  contracts exist.

## Revisit Triggers
- A second reranker family requires materially different request normalization.
- Node execution dispatch becomes too large to keep maintainable in one file and
  needs an extracted per-capability executor split.
- Saved workflow migrations become necessary for structured document inputs.

## Dependencies
**Internal:** `workflow-nodes`, `inference`, `pantograph-workflow-service`,
graph/task modules in this crate.

**External:** `serde_json`, async runtime support, and dependencies declared in
the crate manifest.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-006-canonical-node-contract-ownership.md`

## Usage Examples
```rust
use node_engine::core_executor::CoreNodeExecutor;
```

## API Consumer Contract
- Hosts call into the engine/executor surface with workflow graphs whose node
  types and port IDs have already been validated against backend-owned node
  contracts where graph-authoring validation is required.
- Execution errors distinguish invalid workflow input from backend/runtime
  failures where possible.
- Disabled node behavior, including `tool-executor`, must surface as execution
  errors rather than successful placeholder outputs.
- Additive node inputs may be accepted for compatibility, but callers should
  prefer the canonical `pantograph-node-contracts` projections when
  constructing new graph-authoring workflows.
- Canonical `llm-inference` text/chat, embedding, rerank, audio transcription,
  and image-generation execution should pass through
  `inference::InferenceExecutionRequest` rather than constructing
  backend-native request JSON or calling backend-specific gateway helpers in
  node-engine. Streaming remains responsible for graph `TaskStream` event
  shaping, but the request itself uses typed gateway stream methods instead of
  direct node-engine HTTP transport.
- When canonical inference inputs include resolved Pumas package facts,
  node-engine forwards them on the typed request so inference can emit
  backend/model compatibility summaries. Malformed package-facts payloads are
  execution input errors, not silently omitted optional facts. Node-engine must
  not derive those summaries locally.
- Canonical `llm-inference` dependency preflight may use an explicit graph
  `backend_key` input, but it must not interpret resolved Pumas backend hints
  or `recommended_backend` metadata as executable backend selection. Legacy
  `runtime_hint` fields are not backend preference inputs. Runtime scheduling
  and admission policy remain outside node-engine and inference.
- Canonical dependency preflight requires explicit Pumas identity from
  `pumas_model_ref` or `model_id`; `model_path` is not a successful
  graph-facing dependency-preflight identity. The host resolver owns executable
  load-target resolution from that identity.
- Workflow dependency input resolution carries package-facts context from
  `puma-lib` model-reference edges into canonical inference inputs, so existing
  Pumas model-ref connections can benefit from package-facts diagnostics without
  requiring a separate saved-workflow edge.
- Hosts that need durable typed inference diagnostics may provide
  `extension_keys::INFERENCE_LIFECYCLE_SINK`; node-engine only forwards bounded
  lifecycle facts and does not import or write the diagnostics ledger.
- Dependency-preflight lifecycle failure details redact local path-shaped tokens
  before they reach the lifecycle sink. The returned execution error may still
  contain the full operational detail for the caller.
- Contract-only canonical inference validation may forward bounded stable
  artifact refs to lifecycle diagnostics for task-validation failures. It must
  filter local path-shaped refs and keep started/cleanup lifecycle events free
  of artifact refs.
- Canonical `llm-inference` text/chat `usage` output is bounded token-count
  metadata from typed gateway results or terminal stream chunks. It must not
  contain prompt text, generated text, token arrays, logits, tensors, backend
  kwargs, or local paths.
- Canonical `llm-inference` image-generation request construction consumes the
  graph/API field `denoising_scheduler` for optional sampling intent. The
  overloaded graph/API key `scheduler` is intentionally not read as an alias.
- Backend-owned port options may expose disabled rows with typed availability
  state, stable reason code, and bounded display reason fields. Providers must
  keep primitive option values separate from labels and must not hide
  unavailable state in metadata or presentation strings.

## Structured Producer Contract
- Built-in node descriptors, canonical contract projection, and execution
  dispatch must evolve together.
- Task metadata such as `taskTypePrimary` is machine-consumed by dependency
  selection and must remain stable once introduced.
- Canonical text/chat generation inputs may provide grouped
  `generation_options`; malformed grouped options are execution input errors,
  not backend/runtime failures.
- Reranker outputs are published as ordered result lists plus convenience fields
  such as top score/document; consumers should not infer ranking from raw input
  order.
- Image-generation outputs publish one graph-visible image body for artifact
  conversion plus compact generated-image summaries, metadata, and diagnostics.
  `results` and diagnostics must not duplicate generated image bytes, prompt
  text, tensors, backend kwargs, or backend-local command flags.
- Text/chat outputs may include the existing graph `usage` port when the
  backend reports bounded prompt/completion/total token counts. Missing usage
  remains a valid backend-default case and must not be synthesized by
  node-engine from prompt or response content.
