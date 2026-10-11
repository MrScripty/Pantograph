# CPU inference workflow qualification

The native scheduler-to-model path and live HTTP command lifecycle **pass** with public Pumas correction
`aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`. The editor-to-scheduler-to-model demo
still needs browser qualification. The native probe uses existing public APIs
without changing Pumas, opening an existing catalog, downloading a model, or
substituting successful validation.

The [CPU embedding browser-host guide](../examples/cpu-embedding-demo/README.md)
contains the integrated PR71 transport, supported editor command bindings,
reproduction commands, HTTP evidence, and exact browser/projection blockers.

## Model and acquisition

The probe uses the repository's [untrained BERT-8 fixture](../crates/inference/tests/fixtures/candle_bert/README.md):
one layer, eight output dimensions, CPU F32, attention-mask mean pooling and L2
normalization. Its 6,464-byte safetensors weights have SHA-256
`549cf713305780cd5af92cf397e6be34ada0e3650f967bb9772fa08ba9cedd44`.
No pretrained model is acquired; additional model download size is zero. These
generated numerical fixtures do not establish useful semantic embedding quality.
They live under the repository license material; the existing Apache-2.0 versus
Cargo `MIT OR Apache-2.0` inconsistency still needs resolution before distribution
(see [Release](release.md)). No separate pretrained-model license is asserted.

## Native service probe

From the repository root, with its pinned Rust toolchain and ordinary build
dependencies available:

```sh
probe_parent="$(mktemp -d)"
probe_root="$probe_parent/run"
PUMAS_REGISTRY_DB_PATH="$probe_root/registry.db" \
HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
cargo run --locked -p pantograph-embedded-runtime \
  --features host-dependency-inventory \
  --example cpu_embedding_workflow_probe -- "$probe_root"
```

The probe requires a new absolute directory and a matching isolated registry
path. It refuses an existing directory, so reruns cannot overwrite evidence or
open a private catalog. Pumas networking and process management are disabled;
the temporary library contains only the copied committed fixture. Do not pass
an existing model library or a local unpublished Pumas override.

The probe uses resource-backed hosted composition and the actual Pumas owner.
It obtains the inference interface from backend validation, authors that returned
interface, requires executable validation, resolves dependencies, publishes the
executable snapshot, creates a session, and submits to the scheduler. Pending
dependency work resumes the same run through the public API. It never inserts a
Ready snapshot, rewrites a load target, or bypasses capability checks.
The owned readiness producer polls every 50 ms for this bounded probe, rather
than the ordinary 60-second background interval. The edit session stays open
while its dependency work is checked.

Evidence in `probe_root` includes the saved workflow, descriptor and executable
validation, dependency Resolve response, published validation, session cleanup,
the diagnostic SQLite ledger, a bounded `diagnostics.json` export, and
`result.json`. A completed native run must return an actual eight-dimensional
vector matching the independent reference within `1e-4`, close its session, and
shut down the runtime. Failure exits nonzero and retains the failing stage.
Even native success leaves `editor_qualified: false`.

The default supported backend profile includes Candle. The reduced
`--no-default-features --features backend-pytorch,standalone` profile currently
does not compile because it lacks `InferenceGateway::new`; it is not a verified
alternative host.

With public Pumas `aa1d79ae`, the real scheduler run
`run_e0267526-7200-4cbe-ad98-61de20937397` completed on Candle `candle.cpu` / CPU
F32. Its inference task took 28 ms, backend run timing was 111 ms, and total
submit-to-response wall time was 158.11 ms. These are single observations,
including workflow/runtime overhead, not an isolated forward benchmark. Session
close and runtime shutdown both succeeded. The actual returned vector was:

```json
[0.13531816005706787, -0.5401950478553772, 0.3139018416404724,
 0.4897967278957367, -0.14633086323738098, 0.257649689912796,
 0.0033145255874842405, -0.5134549736976624]
```

The [full native result and diagnostic events](evidence/cpu-embedding-public-aa1d79ae.json)
preserve this output and its run identity. Both existing real CPU qualification
tests (cold cleanup and resident-owner reuse) now pass and are enabled on the
published dependency pin. Browser qualification remains separate.

## Independent model forward

The separate reference probe reads and verifies the committed fixture hashes,
loads only local safetensors through Transformers, executes a real CPU forward,
and compares it with the committed independent reference. It does not regenerate
weights or golden outputs:

```sh
HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
python scripts/probe-cpu-embedding-reference.py
```

This requires an already provisioned Python environment containing CPU PyTorch,
Transformers, tokenizers and safetensors. Its JSON output records the actual
package versions, model hash, vector, load time and forward time. This is
model-only evidence; neither the scheduler nor the editor is exercised.

The existing native numerical test separately exercises actual Candle forwards:

```sh
cargo test --locked -p inference --features backend-candle \
  real_candle_forward_matches_two_independent_transformers_references -- --nocapture
```

## Remaining integrated-demo gates

1. The Pumas directory-target gate is resolved by the branch's ordinary dependency
   update from `26a84e32` to exact public correction `aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`.
   Direct owner API observations and actual scheduler inference both pass, without
   changing the Pantograph adapter or directory guard. See the
   [pinned diagnosis and corrected responses](pumas-hf-target-diagnosis.md).
2. The actual editor currently uses Tauri commands and subscriptions. The
   `pantograph-frontend-http-adapter` is an outbound host adapter, not a server
   exposing the editor's command/event API. Plain Vite/Chromium is not an
   integrated native host. A native desktop host needs GTK3/WebKitGTK 4.1 build
   prerequisites, WebKitWebDriver, tauri-driver and a display for GUI automation.
3. PR71's verified revision `7fc3584cca8bef647b60ebf303c4b5d6b37d98bd` is now merged
   into this feature branch. The new fixture host connects editor commands and
   Submit to the actual embedded runtime and passes live HTTP tests. Qualify the
   actual browser interaction on a host with a usable Chromium sandbox, resolve
   the runtime node-status owner-event gap described in the host guide, and capture
   one run identity from editor Submit through scheduler dispatch, actual model
   output and visible I/O Inspector content, plus the screenshot. Preserve the
   model revision, backend/device selection, timing and cleanup evidence.

No screenshot of model output in the actual editor exists from this cloud
qualification. A terminal vector, a mocked transport, or a static results page
must not be presented as that evidence.

## Original blocked qualification, 2026-10-11

The cloud run used Pantograph main `256b6b94ece4dbbaa1fb236bc4092ad08aa63525`
plus this probe, published Pumas `26a84e32`, and locked Candle
`88ed7911de9e88196b1f55b199145d22647f415e` (`0.9.2-alpha.2`). No dependency
override or model download was used.

The native probe compiled and reached a real scheduler runtime attempt after
successful executable validation, Resolve and publication. The diagnostic ledger
recorded runtime `candle`, variant `candle.cpu`, device `cpu`, reservation
`runtime-registry.1`, and this failure:

```text
embedded runtime-host embedding gateway execution failed: Backend error:
Configuration error: selected embedding: Pumas executable target must be an existing absolute directory
```

The task became `TerminalFailed`; its recorded duration was 12 ms. This is failed
attempt latency, not inference latency. Session close and runtime shutdown
succeeded. The bounded diagnostic export contained all seven events. The probe
exited nonzero and did not emit a fabricated vector.

The separate actual Candle numerical test passed for widths 8 and 12 with maximum
absolute reference error `5.9604645e-8`. The independent model-forward script ran
on Python 3.12.3, Torch `2.14.1+cpu`, Transformers `4.53.3`, tokenizers `0.21.4`,
and safetensors `0.8.0`. Its one CPU F32 observation was 30.51 ms to load and
9.68 ms to forward, with zero reference error and this actual output:

```json
[0.13531813025474548, -0.5401950478553772, 0.3139018416404724,
 0.4897967278957367, -0.14633087813854218, 0.25764963030815125,
 0.003314525820314884, -0.5134549736976624]
```

These separate component results do not qualify the integrated workflow. The
host also lacks `gtk+-3.0` and `webkit2gtk-4.1` pkg-config entries,
`WebKitWebDriver`, `tauri-driver` and a display. The editor GUI smoke was not run.
