# Repaired controls and scheduler graph composition

Status: locally qualified candidate for parent-owned hosted execution and review.
No main merge, PR creation or external review request.

## Exact source and coverage change

- Branch: `integration/scheduler-graph-functionality`.
- First parent: accepted repaired controls composition
  `b8232ccd4db04a568646bd77b4b8a9463c28c93e`, tree
  `708c8c80d3c7dcc1527b41e6ce3b7e2fce19855f`.
- Second parent: accepted image sink `3fd46ff4a21d881b618fe84a64961036d1476d9f`,
  preserving JSON filter `655a83231ba068b27393e1c6f70c9b5e1a1a7d76` and text merge
  `0b8209c43517314f55b2a22c0f9c121cada6e6c3` in their ordered source ancestry.
- Clean automatic merge tree before the requested coverage improvement/report:
  `f64e5c5562121e4f312439a389780ab1f17a5a84`, matching the parent's independent
  merge-tree result. No conflicts or resolution edits.
- Includes repaired main `fa89621eedd3b716477b4f6ee6b883547bc08cda`, PR49 repair
  `8da19e851802f5d0ca74b323eb301791fa6a72a9` and ordered PR50–52 sources.
- Pumas pin remains `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`.
- Pooling candidate `495efea606620153fe2f0363b104e56518436a06` is excluded.
- Ongoing reservation/observation work, including
  `1f6401b7282cb88404c5174115a5b2cb5f655f57`, is excluded and remains separate.

The parent requested one fixture improvement before freeze: concurrent image
runs previously returned identical `runtime-output-image` IDs, so that equality
could not detect swapped artifacts. The existing batch-host fixture now supports
distinct IDs derived from each member's workflow run ID. The concurrency test
uses this mode, checks outputs differ and asserts each response's artifact ID
matches its own run. Existing request/session/attempt/lifecycle assertions remain.
Other fixture uses retain their prior default IDs. This repairs a coverage gap;
no production runtime regression or successful native test execution is claimed.

The only additional source change beyond the clean merge is this test fixture
and assertion. Frozen source refs are preserved. Standards inspected at
`366c1d90a24bbfb50973f62b155a5f3396c0f107`; audit/traceability gates remain intact.

## Combined available qualification

| Command / check | Actual result |
| --- | --- |
| `cargo test --locked -p pantograph-runtime-registry -p pantograph-inference-interface-contracts -p pantograph-runtime-host-contracts -p pantograph-scheduler` | 75 registry tests, 19 interface contract tests, 42 runtime-host unit plus 7 compatibility tests, and 130 scheduler tests passed. No failures or ignored tests. |
| `cargo test --locked -p node-engine --lib` | 261 passed; one existing benchmark-like harness ignored. Includes real core merge, JSON extraction and typed-reference passthrough mechanics. |
| `cargo test --locked -p inference --features backend-candle,backend-pytorch --lib` | 725 passed, none failed or ignored. Real frozen CPU BERT forward/replacement/typed-name checks and controlled PyTorch process/IPC compatibility fixtures; these fixtures are not model inference. |
| `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-workflow-service -p pantograph-embedded-runtime --features backend-candle,backend-pytorch --lib --tests` | Passed without warnings. Changed workflow/session code compiled; not linked or executed locally. |
| `npm run test:frontend` | 659 passed, no failures or skips. |
| Critical/accessibility gates and traceability checker tests | Passed; accessibility checker 27 tests and traceability checker 28 tests. |
| Scheduler-only public surface gate; production audit | Passed; zero production vulnerabilities. |
| Formatting, whitespace and staged/committed-range traceability | Applied before publication. |

Logs: `/workspace/pantograph-cache/graph-composition-*.log`. Existing toolchain,
regular speed, Rust 1.92.0, Node 24.12 and one Cargo build job.

Hosted execution of the changed workflow tests remains pending. No GUI, desktop
IPC, GPU, real text/image models or dependent mixed-model workflow ran locally.
No ORT download/link retry, runtime substitution, dependency workaround, credential,
network or permissions change was attempted. The owner-deferred cloud limitation
does not block source delivery. Parent owns hosted CI, final review and integration;
independent source acceptance does not substitute for those checks or DA-03.

Freeze this composition and continue the separate production candidate
evaluation/selected-reservation and observation work under the coordinator's
existing instruction; do not wait for this milestone's review to progress.
