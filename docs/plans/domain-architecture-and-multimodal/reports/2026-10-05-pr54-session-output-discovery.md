# PR54 public-session output discovery fixtures

This small successor starts from published PR54 repair head
`78bc71931772d891a6b5555076a072a63fe969a7`, tree
`141a0eae52ec7aa587536da297dd14a652836431`, on
`fix/pr54-session-output-discovery`. The separate top-k composition remains
frozen at `bc655d3458757766a224531e4a8bbea02953a467`.

The parent reports 910 passing and two failing headless tests, and 27 passing
and two failing focused tests. Both failures reject requested `out.text` as
non-discoverable after the earlier attribution fixture repair.

Public host I/O discovery deliberately reads persisted `data.definition`:
only input/output categories with `io_binding_origin: client_session` are
exposed, using the appropriate directional ports. These two fixture graphs
saved empty node data instead. The canonical `NodeRegistry` already declares
`text-output.text` as a client-session output. The fixture now serializes the
real built-in definitions for every node before saving and publishing that
same graph. No production discovery, registration or authorization rule changes.

Both scenarios call public `workflow_get_io` and assert their exact source
input ports and sole `out.text` output. The final exact output assertions remain:
JSON extraction preserves `"  red cube\n"`; fan-in preserves deterministic
source-ID order and content `"  first \nsecond"`, and performs zero runtime
loads. Stream inputs and processing-node outputs are not exposed as targets.

The failure is in `completed_scheduler_run_response`, after every scheduler
task has completed. Inspection covers the later task-result lookup, completed
status/identity/port validation, string projection, host binding validation and
requested-output production check. The original result assertions exercise
these gates after corrected discovery; source inspection is not execution.

Local qualification: 11 node-engine single-task tests passed, including real
core JSON-filter, merge and text-output behavior. Workflow-service library/test
compiler-Clippy checking passes with ONNX downloading disabled; the unchanged
inference dependency reports its existing unused selected-text-load fields.
Formatting, whitespace, critical and scheduler-only surface gates pass.
No native linking/execution or ONNX download retry was attempted.

Parent-hosted exact-head execution is required after publication:

```sh
cargo test --locked -p pantograph-workflow-service --lib workflow::tests::session_execution::scheduler_session_extracts_json_prompt_into_downstream_text_output -- --exact
cargo test --locked -p pantograph-workflow-service --lib workflow::tests::session_execution::scheduler_session_runs_text_fan_in_and_downstream_output_without_a_runtime -- --exact
cargo test --locked -p pantograph-workflow-service --lib workflow::tests::session_execution::
```

Retain the ordinary PR54 headless suite. Local logs are
`/tmp/pr54-output-discovery-node-tests.log` and
`/tmp/pr54-output-discovery-final-clippy.log`. Parent owns PR publication and
independent review; the original top-k milestone/branch is not advanced here.
