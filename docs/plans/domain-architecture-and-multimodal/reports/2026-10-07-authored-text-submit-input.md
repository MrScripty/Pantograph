# Authored text at the desktop Submit input boundary

Native qualification attempt 2 at `1e127925` accepted and enqueued the saved
four-node, three-edge CPU embedding graph, then failed with `runtime scheduler
graph has no ready task and is not complete`. Its validation was current and
executable. The prior Invalid control-task error was absent. Native per-task
states were not exported, so the following state explanation is a deterministic
source replay rather than a claim about an exported native state snapshot.

Projection retains prompt, inference and vector tasks; every dependency names a
retained task and the canonical control association contributes no input binding.
The desktop Toolbar nevertheless submitted an empty request input array despite
`prompt.data.text` containing `hello world`. Source tasks start AwaitingInputs;
the scheduler materializes only explicit typed request inputs. With no prompt
result, all three tasks remain AwaitingInputs and the existing guard correctly
rejects preparation. Removing further dependency edges would not fix that cause.

Submit now captures unbound authored text-input strings as typed `text` port
bindings before asynchronous session creation. Empty and whitespace strings are
preserved exactly. Nodes with any incoming edge, missing/non-string text, and
other node types contribute no binding. This narrow change leaves bound inputs,
backend input authority, Invalid states, dependency proofs and admission guards
intact. Other primitive input types and source nodes with incoming bindings are
outside this fix; no fallback or coercion is introduced for them.

The real parsed Toolbar handler regression uses controlled services and changes
the graph during session creation to verify the captured request. With the old
Toolbar and the same new helper/test harness in a separate baseline checkout,
it fails on actual empty inputs. This is handler integration evidence, not a
mounted browser test. The real scheduler-boundary regression reproduces the
exact guard, rejects a numeric text binding, then verifies typed text produces a
run-scoped Completed prompt result while inference remains
WaitingDependencyReadiness and vector output remains AwaitingInputs.

All 683 frontend tests and 932 workflow-service tests pass, along with TypeScript
checking, frontend build, formatting, critical checks and scheduler-only public
surface checks. Complete effective all-target normal/build/dev Cargo features
are checked before compilation: ORT load-dynamic, ort-sys disable-linking, no
download-binaries/fetch-models/copy-dylibs/tls-native, current Pumas `26a84e32`.
ORT_SKIP_DOWNLOAD=1, HF_HUB_OFFLINE=1 and TRANSFORMERS_OFFLINE=1 are defensive
build guards. Existing inference dead-code warnings are retained.

Both prior native attempts and their original artifacts remain outside source
Git. A separate reviewed qualification candidate will retain the existing native
workflow assertions, synthetic fixture and oracle. Actual CPU execution,
artifactRead and inspector acceptance depend on that bounded native outcome.
Controlled local tests do not qualify GPU, pretrained models, real-user model
discovery, full production loaders, or Pumas IPC repair.
