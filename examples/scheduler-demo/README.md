# Deterministic scheduler demonstration

Run a real three-node Pantograph graph without model weights, an inference
server, or a desktop display:

```bash
# From the repository root, with its pinned Rust toolchain and Python 3:
python3 examples/scheduler-demo/serve.py
```

Open `http://127.0.0.1:8765`. Run the supplied JSON, edit its prompt and run again,
then choose **Missing path** and run. The first build needs Cargo dependencies;
subsequent runs use the compiled example. The server binds only to loopback.
Stop with Ctrl+C. It creates an isolated temporary workflow and SQLite stores
for each request; it does not change saved user workflows or Pumas data.

## What actually executes

The [Rust example](../../crates/pantograph-workflow-service/examples/scheduler_demo.rs)
uses the same JSON-extraction path covered by the service's
`scheduler_session_extracts_json_prompt_into_downstream_text_output` test:

1. Load canonical built-in definitions and validate the three-node graph.
2. Publish an executable validation snapshot to the local service's store.
3. Create a workflow execution session and submit through the scheduler API.
4. Materialize `source.value`; execute `json-filter` and `text-output` as
   scheduler-owned non-runtime tasks.
5. Close the session and read the actual SQLite diagnostic event records.

“Publish” in step 2 is an internal snapshot operation, not network publication.
The host's legacy `run_workflow` method deliberately errors if called. There is
no substitute executor returning canned outputs. The browser draws the returned
graph and displays the Rust result; **Download execution JSON** preserves it.
The three-node graph has two executed processing/output tasks: the input binding
is materialized, rather than counted as an extra task attempt.

The missing-path case is a real failure: the filter has no value suitable for
the downstream text task, and the service reports that `out` ended `Invalid`.
The generic scheduler estimate may still mention `inference_gateway`; the
recording preserves that reason as returned. It is not proof of inference use.

## Evidence and limits

- [Successful execution screenshot](../../docs/images/scheduler-demo-success.png)
  and [matching raw JSON](../../docs/images/scheduler-demo-success.json).
- [Rejected execution screenshot](../../docs/images/scheduler-demo-rejection.png)
  and [matching raw JSON](../../docs/images/scheduler-demo-rejection.json).
- [Actual node editor screenshot](../../docs/images/node-editor.png): Chromium
  renders the backend-produced graph and definitions in Pantograph's existing
  Svelte editor. The capture loads the graph through `loadWorkflow`, adjusts
  only node positions, and does not fake IPC, execution states, or output.
  The screenshot visibly retains **Submit unavailable** and **No output yet**.
  Desktop save/edit/submit behavior is not qualified by this capture.

These are genuine Chromium screenshots, not generated images. The native demo
and editor rendering are separately verified surfaces; they are not a claim of
end-to-end Tauri execution. No model inference, GPU behavior, cache reuse,
learning-policy performance, or unpublished Pumas fixes are demonstrated.
Recorded elapsed times are local observations, not benchmark results.

Captured against main `256b6b94ece4dbbaa1fb236bc4092ad08aa63525` plus this branch's
example, JSON Filter renderer registration, and aligned-edge visibility fix, on 11 October 2026. The runtime
uses the repository's existing published Pumas pin; Pumas source was not edited.

## Reproduce screenshots

Start the demo above and `npm run dev`. Start Chromium with a temporary profile
and the DevTools endpoint bound to loopback:

```bash
chromium --headless --remote-debugging-port=9222 \
  --remote-debugging-address=127.0.0.1 --user-data-dir=/tmp/pantograph-capture about:blank
node examples/scheduler-demo/capture.mjs
```

Use the browser sandbox configuration required by your execution environment.
The capture script drives actual UI actions, checks successful and rejected
native responses, and checks that the editor renders three nodes and two edges.
It refreshes the PNGs and their matching JSON records in `docs/images/`.
Stop the processes you started after capture.

For a JSON-only run:

```bash
printf '%s' '{"items":[{"prompt":"Check the fixture."}]}' | \
  cargo run --quiet --locked -p pantograph-workflow-service --example scheduler_demo
```

The executable returns a JSON `execution.ok` flag for scheduler success/failure.
Malformed JSON or setup failures instead exit nonzero with an error on stderr.

## Workbench gallery provenance

The capture script also renders the actual Diagnostics, I/O Inspector, Library,
and Node Editor workbench pages. It assigns the native example's recorded query
responses to the browser service's read methods; it does not fabricate ledger
records or install fake Tauri IPC. Empty-vector normalization matches the normal
frontend service adapter. The example explicitly rebuilds projections from its
SQLite event log before querying them. The same raw response is saved beside the
screenshots, including the run graph, resolved I/O, statuses and projection cursors.

Native event subscriptions remain unavailable in Chromium. The Library's Pumas
calls also remain unavailable, so its error/empty view is not evidence of a
working catalog. The sidebar's Node Editor is the current Node Lab unavailable
page; Graph is the separate working workflow canvas. GTK3 and WebKitGTK 4.1 were
not available in the capture environment. No native Tauri window was run, no
model weights were downloaded, and no private user library was accessed.

The gallery captures source commit `7c9cc6d94da0f4612d5240967bcc5e0aaafa6a9b`
plus the projection export and capture/docs changes in this gallery commit.
No application components or runtime validation were changed for these captures.

The recorded node-status projection is empty for this non-runtime run, although
the ledger contains two task-attempt start/completion pairs. The gallery preserves
that distinction. I/O artifacts retain metadata only; previews are unavailable.
