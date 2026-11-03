# Multi-branch actual-app qualification — 2026-10-12

Six unedited Chromium screenshots show one integrated Pantograph run, not separate
components or substitute mockups. Capture source is clean commit
`7b30b9b0b25697e9216ceb21f6cdb43e41037f34`. The evidence commit only adds these
files and documentation. All images were visually inspected; screenshot hashes,
source/worktree provenance, browser version, visible DOM text, actual request and
response bodies, timings and numerical references are in [receipt.json](receipt.json).
The [previous captures](../real-inference-demo-2026-10-11/README.md) are preserved.

| Actual app view | Evidence |
| --- | --- |
| Authored graph before Submit | [Graph before](graph-before-submit.png) |
| Both completed inference branches and visible report | [Graph completed](graph-completed.png) |
| The completed run in Scheduler | [Scheduler](scheduler.png) |
| Both model identities and CPU host metrics | [Network](network.png) |
| Retained 8-dimensional output read through UI | [I/O Inspector](retained-output.png) |
| Owner status events, task attempts and terminal event | [Diagnostics](diagnostics.png) |

Run `run_b7478981-5ac7-4a93-8546-2d34a1456caf` contains 13 nodes and 13 edges.
Real DOM interactions edited Prompt to `hello`, resolved both dependencies, saved,
submitted, returned to Graph and opened each inspection view. Context is
`different world`. Merge composes `different world\nhello`; BERT-8 embeds that
text and BERT-12 independently embeds Context. A JSON filter reads actual runtime
metadata, then another Merge combines the text, capability limitations and
`candle.cpu` in a visible report. No model responses, owner statuses, command
responses or UI stores were injected.

Both models are **untrained numerical fixtures**, not useful pretrained semantic
models. Actual backend: Candle `0.9.2-alpha.2`, commit
`88ed7911de9e88196b1f55b199145d22647f415e`; device CPU, F32. Public Pumas pin:
`aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`. BERT-8 weights are 6,464 bytes and
BERT-12 weights 10,992 bytes; both are already committed. Their SHA-256 hashes are
in `independent_reference.models` in the receipt. Model download size: **0 bytes**;
paid inference: none. Repository-generated fixture license scope and the existing
repository license inconsistency are documented in the [host guide](../../../examples/cpu-embedding-demo/README.md).

BERT-8 owner duration: **27 ms**; BERT-12: **33 ms**; workflow: **189 ms**;
browser-observed Submit response: **305.05 ms**. These are single-run observations,
not benchmarks. Both vectors differ from independent actual Transformers CPU
forwards by at most **5.960464477539063e-8**. The UI readback exactly matches the
returned 8-dimensional vector. Separate real HTTP tests verify retained readback
for both vectors and the report.

The accepted capture has zero console errors, uncaught exceptions, failed app
commands, visible alerts or external application requests. Browser session close
and [native shutdown](browser-shutdown.json) succeeded. The isolated browser and
both loopback servers were stopped after capture. Chromium ran with specific user
approval for this isolated local `--no-sandbox` capture, a new profile/config/cache,
no credentials, loopback proxy bypass and CDP request restrictions. This does not
qualify native GTK/WebKit or authorize future unsandboxed browsing.

## Limits and publication provenance

**Text generation is unqualified. Image generation and image transport are
unavailable in this qualified host.** The graph's report explicitly displays
those limits. [Capability investigation](capability-limitations.json) preserves
the failed development attempts: missing Python dependency payload handoff,
unsupported external image source, bounded strings and required typed media
artifacts. No GPT-2 output, generated image or image-passthrough success is claimed.
No private catalog, local user machine or Pumas source was modified.

For the portfolio publisher: these six PNGs are actual application UI. Use
`graph-completed.png` as the primary richer-workflow image, with Scheduler,
Network, I/O Inspector or Diagnostics as supporting views. Preserve the
untrained-model and unavailable-capability captions. Do not substitute diagrams,
code screenshots or rejected development captures. No portfolio deployment is
performed by this workstream.

Some inspected fields truthfully remain unavailable: post-session Network
placement is no longer active; scalar sinks have no inference backend attribution;
Scheduler estimate/cache fields are not measurements; dependency badges say
`requirements unknown` even though the resolved runtime proof was consumed.
Owner events and final model outputs provide the execution evidence. These are
remaining presentation/observability limitations, not successful extra features.

Validation: 724 frontend tests, four real HTTP tests (including rejected topology
and model changes), typecheck, full and critical lint, production build, Rust
example build, formatting and decision traceability passed. Reproduction:
[Multi-branch CPU workflow](../../../examples/cpu-embedding-demo/README.md#multi-branch-cpu-workflow).
