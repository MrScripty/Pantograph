# Deterministic scheduler demonstration

Run a real Pantograph workflow without model weights or an inference server:

```bash
python3 examples/scheduler-demo/serve.py
# In another terminal, for the actual workbench UI:
npm run dev
```

Open `http://127.0.0.1:8765`. Run the supplied JSON, change its prompt and run
again, then try **Missing path**. Follow **Open the live workbench** and select a
run in Scheduler to inspect it. The workbench URL explicitly selects the host:
`http://127.0.0.1:3001/?workflowHost=http%3A%2F%2F127.0.0.1%3A8765`.
The first build needs the repository's pinned Rust toolchain and Cargo dependencies.
Stop with Ctrl+C. Both servers bind to loopback.

For a saved, editable workflow, open the workbench's **Graph** tab and load
**editor-demo**. Edit Text Input, click **Save**, then **Submit**. This separate
Text Input → Text Output graph uses authored text bindings and the real scheduler;
its returned output and recorded node status are shown when returning to Graph.
The original JSON extraction example remains available through the demo controls.
Selection Input is a metadata-driven selector, not a general JSON editor, and
Text Input does not implicitly parse JSON.

## What actually executes

The Rust example uses the production workflow service and supported scheduler APIs:

1. Validate the canonical Selection Input → JSON Filter → Text Output graph.
2. Publish its executable validation snapshot inside the local service.
3. Create and run a scheduler execution session with the submitted JSON binding.
4. Execute JSON Filter and Text Output as scheduler-owned non-runtime tasks.
5. Record diagnostics in SQLite and retain small scalar artifact bodies using the
   normal bounded artifact store; close the execution session.

The source binding is materialized, not counted as a third executed task. The
legacy host `run_workflow` method deliberately fails if called. Missing input
paths produce real scheduler rejection, not a canned error response.

The Python HTTP bridge keeps one native Rust service alive. Workbench commands
query that live service; there is no browser service replacement, fake Tauri IPC,
or recorded-response injection. Diagnostic refreshes poll real queries. Retained
text is read through the artifact API and shown in the actual I/O Inspector.
The host accepts only allowed loopback origins and an explicit command set;
unsupported commands and transport failures remain errors.

## Model library and scope

The host opens a real, isolated, initially empty Pumas library with its published
repository pin. It never connects to the user's model library. Model search,
download and delete are disabled with an explanation because this demonstration
host does not support them. No model weights are downloaded and no inference is
claimed. Pumas source and unpublished fixes are untouched.

The **Graph** page supports the saved text example through existing graph-edit,
validation, save, and scheduler-session APIs. This bounded demo host accepts only
Text Input, Text Output, Selection Input, and JSON Filter, with at most 32 nodes
and 64 edges. Saves live in its temporary directory; restarting resets them.
The **Scheduler** tab reads real runs, estimates, and artifacts. **Network** reads
actual local system metrics and scheduler load; peer discovery and GPU metrics
are not implemented by that service and remain explicitly unavailable.

The sidebar's separate
**Node Editor** / Node Lab currently has no authoring API, local agent or runtime
publishing implementation. This change does not pretend to supply those features.
The complete **Settings** view is also unqualified for this host: it mounts native
server/model/device configuration, RAG indexing, sandbox configuration and managed
runtime/binary owners, in addition to artifact retention and format services.
Those owners are not connected by this deterministic example. Unsupported commands
remain errors; no healthy Settings screenshot is claimed.

These are real Chromium application captures over HTTP, not native Tauri windows.
The capture environment lacks GTK3/WebKitGTK. A native desktop application and
editor-to-model execution require separate qualification.

The existing PNG gallery was captured at `7fc3584c`, before saved editor submission
was added. Its Graph image still shows the earlier save/submit limitation; it must
not be relabelled as a completed editor run. New editor, Scheduler and Network
captures require the follow-up capture script below and a permitted browser.

## Capture evidence

- [Successful native execution](../../docs/images/scheduler-demo-success.png) and
  [its raw response](../../docs/images/scheduler-demo-success.json).
- [Deliberate missing-path rejection](../../docs/images/scheduler-demo-rejection.png)
  and [its raw response](../../docs/images/scheduler-demo-rejection.json).
- [Workflow Graph](../../docs/images/node-editor.png),
  [Diagnostics](../../docs/images/execution-ledger.png),
  [timeline](../../docs/images/execution-ledger-timeline.png),
  [completed node statuses](../../docs/images/execution-node-status.png),
  [I/O Inspector](../../docs/images/workflow-run-inspector.png), and
  [isolated Model Library](../../docs/images/model-library.png).

The static presentation reads these recordings; the runnable workbench queries
Rust. Recorded durations are observations, not benchmarks. Generic scheduler
estimate reasons may mention `inference_gateway`; that does not establish model
inference. Missing bodies and genuine failures remain visible when they occur.

## Reproduce screenshots

Start the two servers above, then a temporary Chromium DevTools session:

```bash
chromium --headless --remote-debugging-port=9222 \
  --remote-debugging-address=127.0.0.1 --user-data-dir=/tmp/pantograph-capture about:blank
node examples/scheduler-demo/capture.mjs
```

Use the browser sandbox configuration required by your environment. The script
executes successful and rejected runs, loads the actual application, queries its
live host, checks completed node statuses and reads the retained output bytes.
It checks the healthy gallery for transport failures before saving genuine PNGs.
No errors are hidden or cropped to simulate success. Stop the processes you start.

`node examples/scheduler-demo/capture-editor.mjs` drives the actual saved editor
load, text edit, Save and Submit actions, then checks returned output and captures
Graph, Scheduler and Network. This follow-up script has not yet been browser-
qualified: this environment's Chromium SUID sandbox is misconfigured. No new PNGs
are supplied until a supported sandboxed browser or explicit approval is available.

For the original one-shot JSON interface:

```bash
printf '%s' '{"items":[{"prompt":"Inspect the fixture."}]}' | \
  cargo run --quiet --locked -p pantograph-workflow-service --example scheduler_demo
```

The JSON `execution.ok` flag distinguishes scheduler success and failure;
malformed JSON or setup failures instead exit nonzero with an error on stderr.
