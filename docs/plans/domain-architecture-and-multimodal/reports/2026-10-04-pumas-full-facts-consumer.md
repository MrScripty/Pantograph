# Pumas full-facts consumer: native qualification handoff

This candidate completes the source integration on
`feature/pumas-full-facts-consumer`, preserving checkpoint
`a02b652e6957b3d01834820d0b22558b52837ccf` and accepted owner/client base
`1c0d86dc6275ac2be466c3f918d2fcd35be99cb6`. It is ready for native compilation
and qualification, **not yet runtime-qualified or accepted**.

The current dependency is published Pumas
`2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`, verified tree
`6d27a72603a7571724c6d17179cab64368734a38`, version 0.7.0. Its ordered parents are
`4c850426b528df115db636b0f4798511946710fc` (fixture API timeout repair) and
`58f320a31216872d1a3a389c66bb0c7bb3a30daa` (semantic hash no-op repair). The parent
reports independent acceptance and non-force publication of this exact source.
Its tree matches reviewed composition `9d569b14801220e94b03464a2d9fcc6615e95456`;
the published commit mapping changes metadata only. Both accepted fixes and the
full stacked producer ancestry remain present.

`Cargo.toml` and `Cargo.lock` pin this exact source. Full/default features,
including ONNX Runtime, remain intact. This repin preserves accepted Pantograph
`92e38b85d735ad5efc8c3e4bdd7d11cfab0fa060` and all preceding embedding/custody
repairs. Strict cache freshness assertions and source gates are unchanged. The
lock delta changes only the producer source identity. Earlier receipts below
remain historical evidence from pin
`5be6d967dbd5c0ff7449f342e77f05f9cd645a8e`, tree
`8e693fe410dac9142e6213430638e7180e5643c7`; they do not qualify the current pin.

## Six consumer boundaries

| File | Resulting behavior |
| --- | --- |
| `crates/workflow-nodes/src/setup.rs` | Authenticated local clients obtain the full producer DTO v3. Both facts and target calls translate only the explicit `pumas://models/` URI prefix to the existing relative library ID. Case, Unicode, revision, and artifact selectors remain unchanged. Clients do not register owner API authority. |
| `crates/pantograph-embedded-runtime/src/pumas_dispatch_package_facts.rs` | Owner and local-client access use full facts, not summaries. Existing read-only, contract-version, artifact-ID, size, and path-free dispatch gates remain; returned model and requested revision must agree. |
| `crates/pantograph-embedded-runtime/src/runtime_dispatch_load_target_facts.rs` | Owner and authenticated-client access resolve owner-fresh targets. Identity mismatches fail before projection; physical paths remain stripped from scheduler facts. |
| `crates/pantograph-embedded-runtime/src/runtime_host_package_facts.rs` | Host resolution uses the same access facade, checks returned identity before privacy adaptation, and preserves concrete producer-relative entries and supported Transformers/Diffusers evidence. |
| `crates/pantograph-embedded-runtime/src/runtime_host_load_target.rs` | Host resolution uses owner/client access and verifies model, requested revision, artifact, and requested path. Only the host target retains the physical executable path. |
| `crates/pantograph-embedded-runtime/src/workflow_service_composition.rs` | Hosted construction accepts owner or local-client access through the existing resolvers. Read-only access remains rejected. No new gateway or residency owner is introduced. |

The source audit completed two necessary download-progress binding changes in
`pantograph-uniffi/src/lib.rs` and `pantograph-rustler/src/pumas_nifs.rs`:
producer progress now returns `Result<Option<_>>`; existing binding errors are
propagated before serializing successful progress. Two existing test call sites
now wrap owner API handles in the access facade. Positive live-owner fixtures
use the producer's unversioned identity instead of inventing revision `main`.

## Identity and supported scope

The access facade returns the complete producer DTO, including GGUF evidence,
inspection manifest, diagnostics, and exact nested reference metadata. Its new
real-owner/client test compares complete serialized facts and load-target
state/payload/diagnostic codes, respects existing IPC diagnostic sanitization,
rejects a wrong token, and checks owner survival after client drop.

Dispatch intentionally projects scheduling facts; it does not expose executable
paths. The existing host decoder adapts producer model-reference version
metadata to the inference DTO. That DTO models Transformers and Diffusers
evidence but does not model the producer's GGUF or inspection-manifest fields;
this candidate does not invent new inference contracts for those fields.
The existing logical privacy projection for owner-local absolute package entries
remains; physical load paths come only from the separately resolved host target.

The current producer's full-facts resolver reports no revision. A requested
revision therefore fails closed when absent or mismatched; it is never inferred
from scheduler data. Default selected artifacts must match the explicit selected
artifact ID. Read-only access gains no full-facts or owner-fresh authority.

The accepted Candle branch is separately published at `8ed84956b72b383f78ae5a0121cff94f589db915`,
tree `df8b8e4945c67a5967337846566c943bf7f78bbb`, preserving `05ea1f2c` ancestry.
This consumer can supply its existing typed embedding request/facts/target
contracts without broadening CPU F32 BERT, recipe, architecture, dtype, or device
support. Candle source and host embedding graph/artifact routing are not changed
or merged here. End-to-end embedding execution remains a separate integration.

## Minimal native acceptance

Run in the native environment with its existing verified ORT runtime; retain the
normal full feature profile and locked dependency source:

```sh
cargo test --locked -p workflow-nodes --features model-library --lib configured_local_client_full_facts_and_guarded_target_match_real_owner
cargo test --locked -p workflow-nodes --features model-library --lib
cargo test --locked -p pantograph-embedded-runtime --lib runtime_host_package_facts
cargo test --locked -p pantograph-embedded-runtime --lib runtime_dispatch_load_target_facts
cargo test --locked -p pantograph-embedded-runtime --lib
cargo check --locked -p pantograph-uniffi -p pantograph_rustler --lib
cargo clippy --locked -p workflow-nodes --features model-library --all-targets
cargo clippy --locked -p pantograph-embedded-runtime --all-targets
```

The new owner/client fixture uses a local public-API imported minimal GGUF header,
actual owner discovery, and real authenticated framed IPC. This ordinary local
import has no explicit selected artifact ID: its owner-fresh target response
must preserve the producer's non-ready admission guard, not invent an artifact.
It executes no GGUF inference. Additional fixtures test ready-target identity rejection, missing
revision rejection, and preserved embedding tokenizer/config/task evidence.
Existing hosted read-only, cleanup, and resource-backed tests remain included.

## Executed checks and limits

Cloud executed locked/offline metadata resolution, default workflow-node tests
(168 passed), runtime-host contract tests (41 passed), dependency-planning
contract integration tests (39 passed), formatting, critical anti-pattern, diff, and
staged/range traceability checks. These checks do **not** qualify Pumas-dependent
compilation or the newly prepared consumer IPC/host tests.

The earlier model-library build stopped in `ort-sys` at the existing CDN `403`
before consumer code compilation. That denial was not retried during this audit.
No feature reduction, alternate download path, credential, permission, or network
setting change was attempted. Native compiler, binding-platform prerequisites,
and runtime results remain to be recorded by the qualifier. GUI, real embedding
handoff, and pretrained semantic inference were not executed here.

No known source-only TODO remains after this bounded audit; successful native
compilation and the listed tests are still required before acceptance. The known
pre-existing Pumas load-target fixture decode failure is not newly attributed to
this change. Gates and external review coordination remain with the parent.

## Published composition repin qualification

This three-file repin starts from accepted Pantograph `92e38b85` and changes no
Rust implementation, fixture, freshness assertion, embedding contract, custody
boundary, feature selection, or gate. The current manifest/lock source is the
published composition above; all other lockfile bytes remain unchanged.

The all-target compiler check for `workflow-nodes` with `model-library` and
`pantograph-embedded-runtime` passed with the default feature graph intact using
`ORT_SKIP_DOWNLOAD=1`. This is **compiler evidence only**: it skips obtaining ORT
binaries, does not link or execute the runtime, and proves no inference. It
reported one warning for unchanged inference fields `package`, `target`, and
`device`. Formatting, critical anti-pattern, diff, and staged/range traceability
checks are recorded in the handoff receipts.

The ordinary locked native check still stops at the pinned ORT 1.24.2 CDN CONNECT
proxy `403`. No credential, permission, network setting, runtime substitution, or
feature reduction was used to claim runtime qualification. The full native
owner/client freshness test, GUI, and model-dependent inference were not executed
at this consumer pin. Earlier real IPC tests from the accepted producer source
remain separate evidence. Exact consumer hosted qualification must run after
independent candidate review and the parent's coordinated PR49 advancement; no
previous-head producer or consumer build substitutes for that gate.
