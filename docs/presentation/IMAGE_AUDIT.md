# Presentation image and view audit — 11 October 2026

The audited public capture revision is `7fc3584cca8bef647b60ebf303c4b5d6b37d98bd`.
The source image bytes match both deployed branches: portfolio Pages commit
`5c5be2f53129f91fe89843caf0e53559f88655c7` and Pantograph Pages commit
`6630f0777e06ff0411085304131be78352dc5e5a`. Eidetic is present in the portfolio
source revision `48a06f9`, not that deployed portfolio snapshot.

## Exact visible messages and interpretation

| Asset | Visible state | Where it is used in the audited version |
| --- | --- | --- |
| `node-editor.png` | `Submit unavailable: Save the workflow before submitting`; Selection Input: `Target input does not expose selectable options`; JSON Filter: `Idle`; Text Output: `No output yet`. | Both portfolio and Pantograph; repeated in Pantograph's Graph perspective. This is the Graph canvas, not Node Editor authoring and not editor-executed output. |
| `scheduler-demo-success.png` | Successful native extraction; no transport-error banner. | Both sites. This is the separate runnable browser demo host, not the Svelte workbench or native Tauri. |
| `scheduler-demo-rejection.png` | `Rejected · inspect diagnostics`; `Execution Failed`; `invalid_request: scheduler task 'out' did not complete; final state was Invalid`. | Portfolio; intentionally invalid `{items: []}`. Must be labeled expected rejection, never a normal-state hero. Pantograph also exposes the matching JSON through its recording selector. |
| `execution-ledger.png` | Run completed; no transport error. Optional Client/Client Session metadata may show `Unavailable`. | Pantograph gallery. |
| `execution-ledger-timeline.png` | Completed native event timeline; no transport error. | Pantograph gallery. |
| `execution-node-status.png` | Both executed tasks completed; three retained payloads; no transport error. | Linked from Pantograph timeline caption. |
| `workflow-run-inspector.png` | Retained text is visible; no transport error. | Pantograph gallery. |
| `model-library.png` | Blue notice: `Local demo host: real scheduler and diagnostics; isolated model library. Search, download and deletion require a host that enables these actions.` Empty table: `No Pumas models found`. | Pantograph gallery. This is an honest isolated empty catalog; management actions are disabled. |
| `node-editor-status.png` | `Node authoring is unavailable in this build`; Authoring API, Local agent, Runtime publishing: `Unavailable`. | Stored in the published artifact but omitted from the audited main gallery. It is the separate Node Editor / Node Lab tab. |

None of those current images shows `__TAURI_INTERNALS__`, `Transport error`, or
`Cannot read properties`. Those strings would identify another or older capture.
The current files themselves still contain the genuine Graph capability warnings.
No error pixels are cropped, hidden, retouched or replaced with fabricated output.

The source and deployed-branch hashes agree, so the branch artifacts are not
stale. Direct HTTP verification and cache headers for `mrscripty.github.io` are
unavailable in this environment because its outbound proxy rejects the host.
A browser cache/CDN mismatch cannot be confirmed or excluded from branch hashes.
The source revision adds content-derived asset query strings for future updates;
that is cache hygiene, not a claim that caching caused the reported messages.

## What the three perspectives actually are

- **Graph screenshot:** a static PNG of the real Svelte Graph canvas.
- **Recorded run:** a real demo-host screenshot plus an interactive HTML viewer
  of previously saved native execution JSON. Selecting events does not execute
  a workflow in the website.
- **Architecture diagram:** an authored HTML diagram of responsibility boundaries.
  It is neither an app screenshot nor evidence of model inference.

## All eight workbench views

Graph, Diagnostics, I/O Inspector and Library have genuine reviewed captures.
Scheduler is a functional read view awaiting a matching capture. Network lacks
the relevant host command; Settings needs real service support. Node Editor
is a separate, unqualified authoring surface. Runtime fixes and new captures require separate qualification; presentation
changes must not pretend to implement them.

The portfolio's Rheon image separately retains: `Accepted times differ; these
are not equal physical horizons.` Its capture is an exploratory fixed-box solver
comparison, not qualified full 3D fluid physics. Kenoma's current schematic poser
has no application-error banner; visible joint creasing is preserved. Eidetic's
native authored-demo screenshot has no model-inference qualification.
