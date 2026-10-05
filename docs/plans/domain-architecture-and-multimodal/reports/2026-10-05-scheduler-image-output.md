# Scheduler-owned image output references

Status: implemented for coordinated hosted execution and review. No main merge,
PR creation or external review request. Cloud ONNX qualification stays deferred
by the owner and does not block source delivery.

## Gap and bounded implementation

The runtime host already returns typed media artifact references for image
generation, and core node-engine already implements `image-output` passthrough.
The scheduler still classified that registered sink as unsupported. It can now
execute an image sink after its materialized producer and retain the same
artifact ID and optional media metadata in the requested workflow output.

The new non-runtime template requires exactly one `image` binding. Graph lowering
retains the producer dependency; readiness waits for its completed typed media
reference and preserves blocked, invalid and unavailable outcomes. Missing or
stream-only/mixed streaming bindings remain diagnosed because streaming is not
part of this materialized-result contract.

The existing adapter serializes the typed reference for core passthrough, then
decodes the existing reference DTO and validates the resulting task result.
Strings, artifact URIs, raw JSON objects and model references cannot masquerade
as typed media input. Artifact contents are not fetched or decoded here. No raw
path, base64 representation, media store, new result schema, image generation
backend, GUI behavior or runtime admission/lifecycle policy is introduced.
Media metadata remains opaque producer-owned metadata rather than a new MIME
inference or format-validation policy.

Acceptance: preserve IDs and present/absent media metadata, retain the runtime
dependency and wire round-trip, reject missing and untyped inputs/invalid output
shapes, and exercise public session inference→image-output in the existing
concurrent-run recording-host fixture. This is separately admitted under the
coordinator's continuing scheduler/inference-plan task. It follows current
workflow-service graph/readiness ownership, node-engine mechanics and ADR-011;
it does not revise the older domain plan's admission authority or close DA-03.

- Branch: `feat/scheduler-image-output`.
- Base/single parent: `655a83231ba068b27393e1c6f70c9b5e1a1a7d76`.
- Base tree: `da07b4c9a5be8a8c0b29963400ff361f8c8ecd9e`.
- Preserves the frozen JSON-filter/text-merge and PR50–52 source stack. PR49
  repair and `integration/workflow-controls-pr50-52` remain independent.
- Pumas pin remains `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`.
- Standards inspected at `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
- Write set: classification/template/lowering, existing adapter/readiness,
  focused graph/adapter/core/session tests and this report.

## Qualification and actual limits

| Check | Evidence |
| --- | --- |
| `cargo test --locked -p node-engine --lib` | 261 passed, no failures; one existing benchmark-like harness ignored. New test executes the real core image passthrough and verifies the complete artifact reference is unchanged. No image model runs. |
| Native classifier/graph/wire, adapter acceptance/rejection and concurrent public session fixture | Updated/added and compiler-checked; not linked or executed locally. The session fixture now requests the sink output rather than the inference node directly, while retaining run identity, lifecycle and isolation assertions. Its host is a controlled recording fixture. |
| `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-workflow-service -p pantograph-embedded-runtime --features backend-pytorch --lib --tests` | Passed without warnings; compiler-only qualification. |
| Critical/accessibility gates; traceability checker tests | Passed; accessibility checker 27 and traceability checker 28 tests. |
| Scheduler-only public execution surface gate | Passed. |
| `npm audit --omit=dev --audit-level=high` | Zero production vulnerabilities. |
| Formatting, whitespace and staged/committed-range traceability | Applied before upload. |

Logs: `/workspace/pantograph-cache/scheduler-image-output-*.log`. Existing
toolchain, regular speed, Rust 1.92.0 and one Cargo build job. No frontend source
changed; no frontend suite rerun is attributed to this candidate.

No GUI, desktop IPC, image decoding, GPU, real text/image model inference or
native workflow session was executed locally. No ONNX retry, runtime substitution,
dependency workaround, credential/permission expansion or network change was
attempted. Parent owns native hosted tests, final review, PRs and integration.

The concrete graph mechanics completed in this stack are text fan-in, JSON
selection/extraction and typed image sinks. Next required model acceptance needs
the recorded real text/image fixtures and desktop/runtime execution environment.
The research's learned ranking and hybrid residency proposals additionally need
production capability/observation facts and non-mutating candidate evaluation
with selected reservation commit; simulated scores do not supply those contracts.
This handoff claims neither that acceptance nor approval for a scheduler redesign.
