# Frontend HTTP Adapter Transport Contracts

## Diagnosis and owner

PR #9 Headless run 37099224225 passed workflow-service tests and then remained
in the frontend HTTP adapter step. Completed job logs are unavailable while it
runs. Source inspection identifies an unbounded wait: five tests use a test-only
session wrapper around a vector-output graph, which the current scheduler does
not execute. The service returns before the HTTP host hook is called, and each
test then joins a server blocked in `TcpListener::accept`. Its existing socket
read timeout only applies after acceptance. PR #10 inherits the same test path.

The four HTTP error-envelope tests belong to the transport implementation in
`FrontendHttpWorkflowHost::run_workflow`. Invoke that port directly, retain the
original exact error kinds/messages, and require a real POST to `/v1/workflow/run`.
Remove the test-only session wrapper and disk graph fixtures, which obscured
that boundary. This does not reintroduce a public direct-run service API or make
session scheduling dispatch through the compatibility hook.

The old missing-output fixture mixed two owners: it returned HTTP 200 with an
empty output list but expected the local service to validate a requested target.
The adapter only parses that response. Preserve its empty-output behavior in a
transport test and assert remote `output_not_produced` envelope mapping separately.
Canonical scheduler missing-result rejection remains covered by
`task_result_output_projection::tests::rejects_missing_requested_output`; this
slice makes no new claim about the scheduler's public error code.

## Bounded server and evidence

The test server now uses a five-second accept deadline and retains a two-second
read timeout. Transport test requests have a two-second timeout. Read the complete
request line, headers and declared body before replying, avoiding partial-read
assumptions; assert the actual HTTP method/path. Missing dispatch becomes a clear
finite server failure rather than a CI job waiting indefinitely.

Changes are confined to the adapter's test module and this report. Rustfmt and
whitespace pass. Local Cargo execution is unavailable under the shared disk
constraint; existing binaries do not qualify changed source. Independent review
and fresh hosted adapter tests are required. Running earlier jobs cannot consume
this repair automatically and need cancellation or their existing timeout before
their final logs become available. Broader lint, Clippy, audit and native binding
qualification remain separately open.

## Terminal evidence and review

Both earlier stalled runs were cancelled through the authenticated GitHub UI.
Completed PR #9 logs now confirm 878 library tests and all 31 integration contracts
passed; the adapter compiled, passed nine of fourteen tests, then blocked in its
remaining HTTP fixtures until cancellation at 05:31:39 UTC. Native/C# stages did
not run. This corroborates the source-grounded hang diagnosis.

Independent integrator source review accepted staged tree
`85d9372bd91786187719042bc778446485d553fd`, including transport/target-validation
ownership and bounded socket lifetimes. At the reviewer's suggestion, the empty
response transport test additionally checks the exact received JSON workflow ID,
inputs and output targets. This is source review only; hosted compilation and
execution remain required. Production module bytes remain unchanged.
