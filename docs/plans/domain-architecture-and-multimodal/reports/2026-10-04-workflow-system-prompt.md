# Workflow text system prompt

Status: implemented for coordinated review. Native consumer execution remains
owner-deferred under the Codex cloud ONNX limitation; source delivery and useful
independent qualification continue. No merge or external review request.

## Feature and source

Text-generation workflow nodes now advertise optional `system_prompt` as a
string input. The host preserves its exact content in the typed inference
request, which the existing selected-text gateway sends before the user prompt
as a system message. The existing PyTorch adapter extracts that message into
its worker's `system_prompt` field. Omission remains `None`; explicitly empty
strings remain supplied strings. Prompt and system prompt each retain the
existing 1024-byte string bound, and `max_new_tokens` remains independent.

The concrete missing behavior was a descriptor without this input and a host
that rejected it and always constructed `system_prompt: None`, despite existing
gateway/worker support. This lets a graph author supply reusable generation
instructions separately from task-specific text, including instructions for
the generated prompt consumed by the next image node.

- Branch: `feat/workflow-system-prompt`.
- Base/single parent: `a66b63a3205aada16a4b8074930c8733b31f817d`.
- Base tree: `3adc67b530899499953d74d087291f80d0ae5227`.
- Pumas pin: `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`, unchanged.
- Standards inspected: `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
- Write set: text descriptor/projection, shared text-input fixture and contract
  test, existing gateway recording tests, and this report.
- PR49 typed-name repair `8da19e851802f5d0ca74b323eb301791fa6a72a9` remains
  independent. PR50 `d8c636f9239b46d24ed0cafb584329841cc826fa` and image candidate
  `a66b63a3205aada16a4b8074930c8733b31f817d` are preserved in stacked ancestry.
- Pooling candidate `495efea606620153fe2f0363b104e56518436a06` remains excluded.

The existing domain plan's DA-03 requires actual dependent text-to-image output;
that required-real acceptance remains open. The scheduler thesis/evaluation and
compatible-inference research inspected in earlier milestones do not establish
current acceptance. Reconciliation with source identifies this small missing
task input through current descriptor, string-source, host and worker owners;
no scheduler-ranking policy, speculative model family, new persistence/wire
variant, or additional lifecycle framework is needed. This feature is admitted
by the coordinator's next scheduler/inference-interface task and does not revise
the older plan's source-remediation authority or claim DA-03 closure.

## Actual qualification

| Claim / command | Evidence and boundary |
| --- | --- |
| `cargo test --locked -p pantograph-inference-interface-contracts` | 19 integration tests passed. New shared text descriptor fixture verifies optional string system prompt, no defaults, retained token-limit type, validation and wire round-trip. |
| `cargo test --locked -p inference --features backend-pytorch --lib` | 683 passed, none ignored. New selected-gateway test executes omitted, empty and whitespace-preserving system messages with correct system/user order and completed cleanup against a recording backend. Existing adapter/envelope system-prompt checks also run. No model inference. |
| `npm run test:frontend` | 659 passed, none skipped. |
| Native descriptor producer matches shared fixture; bounded strings preserve exact host projection; wrong types/oversize inputs reject; token limit and system prompt reach the host's recording backend together | New/updated focused native tests added and compiler-checked; not executed locally. Existing omitted-options projection and unsupported-input checks remain. |
| `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-embedded-runtime --features backend-pytorch --lib --tests` | Passed compiler check; no native linkage/execution claimed. |
| `cargo fmt --all -- --check`, `git diff --check` | Passed. |
| Critical/accessibility gates; traceability checker tests | Passed, including 27 accessibility checker tests and 28 traceability checker tests. Staged and committed-range gates are applied for upload. |
| `npm audit --omit=dev --audit-level=high` | Zero production vulnerabilities. |

Executed using the existing toolchain activation script, regular speed, Rust
1.92.0, one Cargo build job and the installed Python 3.12 library directory in
`LD_LIBRARY_PATH`. Logs: `/workspace/pantograph-cache/system-prompt-*.log`.
No ONNX download/link retry, runtime substitution, network-policy change,
dependency workaround or permissions expansion was attempted.

No GUI, desktop IPC, real text/image model inference, GPU or mixed workflow was
executed. The parent owns hosted/native follow-through for the changed embedded
tests and model acceptance. Controlled gateway/worker fixtures are compatibility
evidence, not inference evidence; actual use of system instructions depends on
the selected model's existing worker/template behavior.

Retain this branch/worktree as a protected candidate. The parent owns PRs,
reviews, integration and retirement. This upload does not advance reviewed
PR49/50/51 refs or earlier candidate branches.
