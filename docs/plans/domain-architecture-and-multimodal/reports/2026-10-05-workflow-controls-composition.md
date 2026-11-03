# PR50–52 composition with repaired PR49

Status: qualified locally for parent-coordinated hosted checks and review. This
candidate does not merge main, advance reviewed feature refs, or request review.

## Exact composition

- Branch: `integration/workflow-controls-pr50-52`.
- First parent: verified remote main `fa89621eedd3b716477b4f6ee6b883547bc08cda`.
- Main tree: `e3d9d65c34f18265638efa99444e2ee02c73e4be`.
- Main parents: `11b046fa23da5715d69e33e98c41cb9aad792f9e` and PR49 repair
  `8da19e851802f5d0ca74b323eb301791fa6a72a9`.
- Second parent: PR52 `762e2b7e4afca0f14781412015a0a6e946bc5ca0`, preserving
  PR51 `a66b63a3205aada16a4b8074930c8733b31f817d` and PR50
  `d8c636f9239b46d24ed0cafb584329841cc826fa` in their ordered source ancestry.
- Automatic merge tree before this evidence report:
  `f6e1dc17a284d656128ec5763d897340aee9ff3b`.
- Merge was clean: no conflicts, resolution edits, dependency changes or new
  feature code. It combines the repaired selected-model identity/lifecycle with
  integer sources, workflow text token limits, image controls and system prompts.
- Pumas pin remains `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`. Coding standards
  inspected at `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
- Scheduler merge candidate `0b8209c43517314f55b2a22c0f9c121cada6e6c3`, ongoing
  JSON-filter work and pooling candidate `495efea606620153fe2f0363b104e56518436a06`
  are excluded from this composition.

## Executed qualification

| Command / check | Actual result |
| --- | --- |
| `cargo test --locked -p pantograph-inference-interface-contracts -p pantograph-runtime-host-contracts -p pantograph-scheduler` | 19 inference-interface contract, 42 runtime-host contract and 130 scheduler tests passed; none failed or ignored. |
| `cargo test --locked -p inference --features backend-candle,backend-pytorch --lib` | 725 passed; none failed or ignored. Includes real frozen CPU BERT forward/replacement/typed-name tests and controlled PyTorch worker/IPC tests. Worker fixtures do not demonstrate model inference. |
| `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-workflow-service -p pantograph-embedded-runtime --features backend-candle,backend-pytorch --lib --tests` | Passed without warnings. Native workflow and embedded test code compiled; these tests were not linked or executed. |
| `npm run test:frontend` | 659 passed, no failures or skips. |
| Critical anti-pattern gate; accessibility gate; traceability checker tests | Passed; accessibility checker 27 tests, traceability checker 28 tests. |
| `npm run test:scheduler-only-workflow` | Passed. |
| `npm audit --omit=dev --audit-level=high` | Zero production vulnerabilities. |
| Formatting, whitespace, staged traceability and committed-range traceability | Applied to the final composition before publication. |

Toolchain: existing activation script, Rust 1.92.0, Node 24.12, one Cargo build
job, installed Python 3.12 library directory. Logs are retained under
`/workspace/pantograph-cache/workflow-composition-*.log`.

The owner deferred the Codex cloud ONNX Runtime download/link limitation. No
download retry, alternative runtime, credential, network-policy or permissions
change was attempted. Native workflow/embedded execution, desktop GUI IPC, GPU
and dependent real text-to-image model workflow were not executed locally. The
real CPU BERT tests are distinct from these outstanding checks.

The parent reported individual PR50–52 hosted tests passed. That report is not
evidence of final combined hosted CI or review; the parent owns those checks and
integration. Reviewed source refs remain frozen. This report and its upload do
not claim the domain plan's DA-03 real-model acceptance is complete.
