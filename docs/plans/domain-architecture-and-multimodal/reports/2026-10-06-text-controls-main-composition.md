# Text controls composed with current main

The minimum-token source `64b81b0b773064dc7cb6df70789e8f44fa91430b` was four commits
ahead and thirteen behind main `763e8d4b13ba311ff245c75a370e56dafb417d8e`, with merge
base `13d24e898abd4175eb799ed310d1d275537880e9`. Parent reports scoped independent
acceptance of repetition/KV source `59992b9b` and minimum-token source `64b81b0b`.
This separate normal merge starts at current main and merges the frozen text head,
preserving both histories. Only plan/ledger conflicts required resolution; no
production or test source needed manual changes. Original branches/evidence and
PR54/55 remain unchanged. Parent owns PR/review/merge and hosted CI.

Read-only composition review compared both parents' shared gateway, worker,
descriptors, snapshots, import sentinels and fixtures. Both text and image controls
remain present with original port indices. Ten deciding source/fixture/config
files exactly match their owning parent, including both samplers, actual text
host projection, image planner/host route and reviewed startup-config fixture.
The mixed public-session text-to-image regression forwards actual generated text
and retains both outputs; combined image controls and connected text controls also
pass through the production host boundaries. Controlled backend fixtures prove
composition and attribution, not real models or hardware.

Fresh qualification on the combined tree:

| Executed scope | Result |
| --- | --- |
| Full mixed llama.cpp/PyTorch inference | 821 tests plus one doctest pass; six optional native/doctest cases ignored in this ordinary run. |
| Embedded runtime | 517 pass. |
| Runtime registry / app config | 142 / 14 pass; one app-config subprocess helper ignored by the outer harness is executed by its parent regression. |
| Inference-interface / runtime-host contracts | 25 / 52 pass. |
| Workflow-service library | 916 pass. |
| Actual text CPU sampler | 32 test methods pass on Torch 2.14.1+cpu / Transformers 4.53.3. |
| Actual CPU Diffusers | All three explicit tests pass serially on installed Diffusers 0.39.0; tiny random UNet, supplied embeddings, zero pretrained models. |
| Frontend | 662 pass; TypeScript check passes. |
| Strict Clippy | Seven affected packages, all targets, mixed backends, `-D warnings`: pass. |
| Other gates | Rust format, lint, critical/accessibility, scheduler public boundary, Python compilation, staged/range traceability and nine ONNX no-build-download dependency graphs pass. |

The first explicit Diffusers run used the default parallel harness: two tests
passed and the scheduler oracle assertion failed. The same suite then passed with
`--test-threads=1`. Tests temporarily patch shared Diffusers scheduler class
methods; interference is a source-supported explanation, not a newly qualified
parallel-test claim. Both logs are retained. No production/test change, increased
timeout or new ignored case was introduced to obtain the serial pass.

Private target `/workspace/pantograph-cache/text-main-composition-target` retains
only external dependency hardlinks from the earlier cache before compilation;
every workspace package was cleaned in both targets to recover bounded disk
space. Source and original evidence were preserved. Exact merge identities,
executed commands, source checks, logs and hashes are in
`/workspace/qualification-evidence/text-main-composition-qualification.json`.

Full Python worker import remains unqualified: soundfile is absent, and actual
worker-function tests use AST-selected source rather than a full dependency import.
GTK/WebKit pkg-config prerequisites are missing. Native desktop, GPU, pretrained
and custom model execution, and ONNX execution remain unqualified. The external
standards directory remains unavailable. Text seed is separately traceable as a
clean worktree at frozen `64b81b0b`; only design inspection occurred before parent
prioritized this composition, and no seed source was included.
