# Accepted observations and graph composition

Branch `integration/scheduler-observations-graph` combines independently accepted
observations `2d861ba7527764a8c796ac9775ef2b9ea9d69024` (first parent) and graph
composition `defc8c5fcafbea07533847b40fbe0cdd253c9909` (second parent).
The clean automatic merge tree is
`a007754ea5945d9475012da1047562c1b0d0bb6f`, matching the coordinator's independent
result. No conflicts or source resolution edits. This report is the only addition
to that merge. The distinct per-run image artifact fixture correction is retained.

Ancestry includes merged main `7555f193dcd342f18fbd65613a33d9deb073e2b6`, PR44
`ca9edde6850cd58ade0b7e534bb4f58e704c2c64`, PR49 repair, ordered PR50–52 and text,
JSON and image feature sources, registry evaluation `1f6401b7282cb88404c5174115a5b2cb5f655f57`
and selected reservation `b19961bac2c86e8511a2c809a4823bac04db8c14`.
Pumas remains pinned to `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`.
Pooling `495efea606620153fe2f0363b104e56518436a06` is excluded. Frozen refs remain
unchanged. Ongoing custody fixes are excluded and progress on a separate branch.

## Actual checks on this candidate

- `cargo test --locked -p pantograph-runtime-registry -p pantograph-diagnostics-ledger
  -p pantograph-runtime-host-contracts -p pantograph-scheduler`: 82 registry,
  116 ledger, 49 runtime-host (42 unit plus seven compatibility) and 133 scheduler
  tests passed; no failures or ignored tests.
- `cargo test --locked -p node-engine --lib`: 261 passed, one existing ignored harness.
- `cargo test --locked -p inference --features backend-candle,backend-pytorch --lib`:
  725 passed, no failures or ignored tests. Real frozen CPU BERT checks and controlled
  process/IPC fixtures ran; process fixtures are not real model inference.
- `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-workflow-service
  -p pantograph-embedded-runtime --features backend-candle,backend-pytorch --lib --tests`:
  passed without warnings. Changed workflow/observer fixtures compiled only.
- Frontend: 659 tests passed, zero skips/failures. Critical, accessibility (27 tests),
  traceability checker (28 tests), scheduler-only surface and formatting/whitespace
  gates passed. Production audit: zero vulnerabilities.
- Staged and committed-range traceability checked before upload; semantic review
  remains necessary. Logs: `/workspace/pantograph-cache/observations-graph-*.log`.

Regular speed, existing Rust 1.92.0 and Node 24.12 toolchains, one Cargo build job.
No ORT retry, GUI/desktop IPC, GPU or real text/image-model run. Native workflow
execution remains owner-deferred for hosted qualification. Source acceptance and
portable qualification do not claim native execution, calibrated phase evidence,
learned ranking or DA-03 completion. Parent owns review/PR publication/integration;
no PR, external review request or main merge performed here.

Continue the coordinator-admitted preparation/commit/binding custody failure and
cancellation fixes independently after freezing this coherent composition.
