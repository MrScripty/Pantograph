# Post-resolution diagnostic and harness successor

Parent e4d79b98da15748b81b4f50e23d739ea3bdaeac7 preserves the complete failed
native attempt at fffe5342. No further production decision is repaired here.
Actual-owner logs now distinguish progress-loop entry/exit, accepted seed
storage, readiness proof resolution, admitted task state and dispatch-selection
entry/preparation. Requests/proofs retain canonical saved-snapshot scope; no
prompt bodies or fake success results are added.

The native inspection DTO omits io_artifacts when empty. The harness now reads
workflow_io_artifact_query's canonical artifacts array, recording the actual
query result alongside run/scheduler/inspection observations. Empty artifacts
still fail after the same bounded observation period; a new submission is not
created. Existing CPU vector and metadata assertions remain.

Checks pass: 988 workflow-service tests, strict selected-crate all-target
Clippy, harness ESLint/syntax, formatting and critical gate. Complete effective
graphs precede the Rust test and Clippy builds; ORT_SKIP_DOWNLOAD=1 is set,
dynamic ORT remains configured, forbidden download features are absent and
current Pumas 26a84e32 is retained. Prior 537 embedded and 21 environment tests
cover unchanged repair source. Native post-resolution transitions remain
pending an exact-source run. CPU output, GPU, pretrained and broad loader
qualification remain incomplete.
