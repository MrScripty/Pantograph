# Resident publication and failed-owner reclaim correctness

Successor `fix/runtime-producer-resident-correctness` preserves frozen producer
source `7c57a744381aae3d2710456b166570fd7d5b93c3`. It carries the unpublished
correctness commits `2e67e083`, `abb717ab` and `b9e83208` without rewriting them,
and adds direct ordinary-reclaim coverage. Parent owns PR, review and merge.
PR54/55 and unrelated features are untouched.

Terminal host-port cleanup now samples and publishes owner residency before
removing a task lease, then reconciles again for both Retain and Evict outcomes.
When full peak claims initially reject resident publication, the existing ordered
owner observation installs unknown coverage before custody release. That coverage
blocks competing admission while the later sample retries the declaration.
The 100-byte shared pool / 40-byte resident estimate / two 50-byte lease regression
completes one lease through `EmbeddedReservationLifecyclePort`, checks the awaited
post-release publication boundary for a rejected competing 20-byte claim, and
requires 40 resident + 50 task bytes, leaving 10 available. The existing selected-text
workflow regression now executes the actual host batch port and gateway load,
retains its generated output, and forwards its completion event to the actual
reservation port with two controlled leases. Its gateway residency remains
unpublished until that terminal reconciliation. Backend inference is controlled;
this does not claim real model allocation. A last-lease regression
requires owner stop acknowledgment before capacity becomes free.

Failed runtimes with uncertain allocations are ordinarily evictable after their
leases and pins permit eviction. Reclaim resolves the matching active or embedding
lifecycle owner even when readiness is false. An absent owner returns an error;
a stop error or an unacknowledged allocation retains unavailable shared capacity.
Only ordered Released evidence from acknowledged shutdown clears uncertainty.
Host regressions cover failed terminal cleanup, failed shutdown, successful calls
without release acknowledgment, ordinary reclaim without terminal cleanup, and
later acknowledgment. Full task peaks, source/generation validation and explicit
known-zero handling remain owned by their existing code.

## Executed qualification

On the final source, 275 portable registry/scheduler/AppConfig tests, 441 portable
inference tests and one controlled PyTorch worker-owner test pass (717 total).
The new registry failure regression was also run against frozen `7c57a744` in a
separate detached worktree: it fails with zero rather than one eviction reservation
candidate. The successor passes it. These are logical accounting and controlled
worker tests, without model weights or hardware execution.

The full PyTorch-enabled inference invocation ran 705 tests: 704 passed and the
existing `fresh_backend_switch_admits_first_load_and_switch_away_confirms_release`
fixture failed attempting to import AutoModelForCausalLM from its controlled
Transformers module. This failure is retained; no broad-suite pass is claimed.
The default portable suite and named production worker-owner test are separate
successful invocations, not substitutes presented as that failing invocation.

Host-test execution is blocked before linking by the pinned ort-sys 2.0.0-rc.12
prebuilt download returning HTTP 403. The new host regressions are compiler-checked
with the dependency's no-download setting (`ORT_SKIP_DOWNLOAD=1`), which provides
no ONNX runtime or execution qualification. GTK 3 and WebKitGTK 4.1 pkg-config
prerequisites are absent. No dependency pin, permission, credential or download
source was changed. Native ONNX, GTK, GPU and real-model behavior remain unqualified.

Commands and complete outcomes are retained in the local qualification manifest
`/workspace/qualification-evidence/resident-correctness-qualification.json` and
its referenced logs. Repository instruction/skill files were absent in the selected
worktree and `.agents`/`.codex`; the active domain plan was read. Its external
Coding-Standards baseline directory is unavailable, so fresh Core/Router review
is not claimed.
