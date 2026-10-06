# Graph-authored text stop string

This user-authorized bounded M2 extension connects the existing typed
`stopping.stop_strings` option to the canonical text graph and PyTorch AR worker.
The plan's descriptor/validation task is its anchor; the plan did not explicitly
promise a numbered post-seed stop-string milestone. Inspection found the typed
option/default resolver and llama.cpp mapping already present, while the graph
port, host/chat forwarding and PyTorch worker behavior were missing. No broader
scheduler, resident-accounting or GPU-admission work is included.

The separate `feat/workflow-text-stop-string` worktree starts from normally
fetched, accepted seed/precision `fae339bdb1eb681a01ea983f1abe0f2f48916349`, tree
`6c8c4bb1cbed0527b58c1048ebf49fa0e243d56b`. It includes fetched main
`d61b86fc27a7ecd725bd994b29a7cdcfcef9a6a3`, tree
`e714ccb9508797268964a3c42190280d57b7b043`, through the existing normal merge.
The seed candidate, paused seed patch, reviewed composition/harness/repair and
precision histories remain unchanged. No main mutation, PR creation or existing
PR change is performed. No AGENTS.md or local skill instructions were found in
the selected checkout or workspace instruction directories. The external
standards path recorded by the plan remains unavailable; no fresh inspection of
that external source is claimed.

The appended optional scalar-string `stop` port has no default and carries one
literal marker. Graph markers must be nonempty and fit the existing 1024 UTF-8
byte input bound; whitespace, case and Unicode are preserved exactly. Empty or
wrong-type values reject before backend effects, and oversized values reject at
the generic materialized-input boundary. Existing JSON graph/request save/load
uses its existing string representation. Chat and worker transport use lists;
typed callers retain the existing multiple-marker option. No new graph wire type
or generic text-input parser is introduced.

Omission remains absent through host, typed gateway, chat and both worker
operations. The existing caller-owned resolver retains model < workflow < runtime
preset < request precedence and source diagnostics. Empty typed lists remain
unrequested; explicit empty worker lists and empty markers refuse. Native and
manual routes accept inherited generation-config string/list defaults. Legacy
native defaults refresh only under the installed Transformers resolver's original
hash/nondefault-owner conditions. Editing a generation config's stop to `None`
owns omission. Resolution and matching do not mutate the resident configuration.

Matching searches cumulative decoding of newly generated IDs, excluding prompt,
replayed context and suffix bytes. The earliest complete marker ends generation;
the marker and trailing text in its token are withheld. Manual append streaming
also withholds the longest possible marker prefix across token/chunk boundaries,
then flushes an unmatched prefix at EOS or the token budget. A tokenizer that
rewrites already emitted text explicitly refuses because append output cannot
retract it. No-marker requests retain their prior decoding path.

| Stop/minimum interaction | Behavior |
| --- | --- |
| Authored `min_new_tokens` and early marker | Refuse immediately; no short success or continued generation. |
| Satisfied authored floor | Stop; count generated token IDs including withheld marker text. |
| Omitted native minimum | Preserve inherited Transformers minimum/forced-EOS precedence; stop criteria can end generation independently. |
| Manual inherited minimum | Resolve/cap the existing floor; an early marker refuses. |
| Explicit minimum zero | Override the inherited floor. |

Native support is conditional on canonical Transformers generation/config/sample/
stopping-criteria functions and single-sequence greedy or sampling tensor output.
Beam, multiple-output, dictionary-output and custom native routes refuse before
forward execution. SDAR, live-KV continuation and masked block diffusion refuse
authored or inherited stops before prompt formatting or KV mutation. Existing KV
state remains untouched on this refusal. PyTorch diagnostics report `Mapped`
forwarding with worker route validation; other gateway backends retain
`RequiresBackendSupport`, without a blanket honored claim.

The [hashed deciding evidence](../evidence/workflow-text-stop-string/README.md)
records final source hashes and reproducible commands:

| Executed scope | Result |
| --- | --- |
| Actual installed Python CPU decoding | 73 methods pass: retained 59 plus 14 stop regressions. |
| Mixed llama.cpp/PyTorch/Candle/std-process inference | 800 library + 74 integration checks and one doctest pass. |
| Embedded runtime/public scheduler-host graph execution | 528 pass, including exact connected stop after JSON roundtrip. |
| Interface contracts / runtime-host contracts | 27 / 52 pass. |
| Workflow service | 976 pass. |
| Existing frontend / TypeScript / production assets | 669 tests, typecheck and Vite build pass. |
| Strict all-target Clippy | Five selected packages pass with `-D warnings`. |
| Other affected gates | Rust format, Python compilation, ESLint, critical anti-patterns, accessibility, scheduler boundary, staged traceability and no-build-ONNX checks pass. |

Six existing optional native/doctest cases remain ignored. CPU regressions cover
cross-token/chunk markers, exact Unicode/whitespace, overlapping markers, prompt
boundaries, EOS/budget flushing, minimum conflicts, defaults/omission, seeded
native/manual draw counts and unchanged ambient RNG, route refusal and actual
worker entry forwarding. Rust tests execute the public graph/host/gateway route
with controlled backend output; they do not run a production model loader.

Independent Astra medium review found a legacy-default detection mismatch:
an inherited empty list could miss a refreshed marker, while an edited `None`
could resurrect the owner marker. Both transitions now pass native/manual/SDAR
regressions and the finding is independently closed. Initial CPU fixture/config
failures and the oversized-host test assumption were corrected without relaxing
production contracts. Strict Clippy required `rfind` in the new scope test.
Implementation/review usage metrics are unavailable.

Before any build, the complete effective all-target graph for the exact five
selected packages/features was inspected. It uses current Pumas
`26a84e323cae566a46a8f76bef48fa1010aed48b`, ORT `load-dynamic` and ort-sys
`disable-linking`; `download-binaries` is absent. Builds used locked offline Cargo
and `ORT_SKIP_DOWNLOAD=1`. Manifests, lockfile and ORT configuration are unchanged;
no ONNX/ORT binary or pretrained-model download, denied-download retry or
network/authentication change occurred. The broader metadata-only ONNX guard also
passes for Linux, Windows and macOS feature selections; it fetched four missing
Cargo crate archives, not build binaries. Existing npm dependencies are linked to
this worktree's workspace package without installation.

Qualification is controlled CPU generation with fixed forwards and installed
Torch 2.14.1+cpu / Transformers 4.53.3. Actual worker functions are selected through
AST because full audio dependencies are unavailable. Production loading, pretrained
or custom models, GPU, full worker import, ONNX execution and complete Tauri/desktop
execution remain unqualified. Parent candidate review/publication and hosted CI
remain separate; this slice does not close DA-03 or DA-07.
