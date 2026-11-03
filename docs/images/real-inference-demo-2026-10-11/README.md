# Real browser qualification — 2026-10-11

These are unedited Chromium screenshots of the actual Pantograph workbench and
loopback native service. Both captures passed at clean source commit
`12fa09a685a4ccc44edf992ba117648f8fa367ef`; the following evidence commit contains
only documentation and these files. Each receipt includes the actual POST command
bodies/responses, source/worktree provenance, browser version, output equality,
latency observations, screenshot SHA-256 values and visible DOM text.

| Evidence | Actual inference | Text Graph regression |
| --- | --- | --- |
| Before Submit | [Graph](inference/graph-before-submit.png) | [Graph](text/graph-before-submit.png) |
| Completed visible output | [Graph](inference/graph-completed.png) | [Graph](text/graph-completed.png) |
| Completed scheduler run | [Scheduler](inference/scheduler.png) | [Scheduler](text/scheduler.png) |
| Local system and run metrics | [Network](inference/network.png) | [Network](text/network.png) |
| Retained bytes read through UI | [I/O Inspector](inference/retained-output.png) | [I/O Inspector](text/retained-output.png) |
| Actual timeline | [Diagnostics](inference/diagnostics.png) | [Diagnostics](text/diagnostics.png) |
| Machine-readable provenance | [Receipt](inference/receipt.json) | [Receipt](text/receipt.json) |

All twelve images were visually inspected. The inference graph has separated
nodes, the explicit label “Untrained BERT-8”, `hello` input and its actual
8-dimensional output. Both graphs show completed output. Scheduler, Diagnostics,
Network execution rows and retained bodies agree with their respective run IDs.
Both captures recorded zero console errors, uncaught exceptions, failed commands
and external application requests. The text Graph is a separate regression run,
not model evidence.

Inference run: `run_92d972df-730e-4412-88d0-b568b5d37c49`; text run:
`run_8e8a5132-55d1-4f08-913a-3f7840c0ee26`. Inference owner duration: 29 ms;
workflow: 121 ms; browser-observed Submit HTTP response: 191.28 ms. These are
single-run observations, not benchmarks. The model is the committed 6,464-byte
untrained BERT-8 fixture, Candle `0.9.2-alpha.2` / CPU F32; download bytes: zero.
The actual model hash, backend commit and public Pumas pin are in the receipt.
No paid inference or private/local catalog is involved.

```json
[0.14047904312610626,-0.5287753939628601,0.09859264642000198,
 0.2963152229785919,0.061696670949459076,0.4760826528072357,
 0.06283976137638092,-0.6072306036949158]
```

The returned vector equals the retained JSON bytes after parsing and differs from
the independent Transformers reference by at most `5.960464477539063e-8`.
[Session close](inference/browser-session-close.json) and
[runtime shutdown](inference/browser-shutdown.json) both succeeded. The isolated
browser and both loopback hosts were stopped after capture.

Chromium `151.0.7922.173` ran headless with explicit user approval for
`--no-sandbox`, a fresh isolated profile/config/cache and no credentials. A
loopback proxy bypass and CDP request interception blocked nonlocal navigation.
The approved mode does not establish native GTK/WebKit support. Reproduction and
model/license scope are in the [host guide](../../../examples/cpu-embedding-demo/README.md).

The Network page shows current placement as “Not scheduled locally” after the
run's session was closed, alongside completed execution rows. Missing backend
key, version, cache and GPU metrics remain unavailable; they are not invented.
The dependency activity panel is transient display state and can reset on Graph
remount. Its proof still belongs to the backend. Settings, the separate NodeEditor,
pretrained semantic quality, mixed-length batch and ONNX inference are unqualified.
