# UniFFI Owner-Derived Validation Publication

## Observed gap and accepted decision

PR #12 hosted UniFFI tests compiled and reached execution, but saved text and
interactive fixtures failed on missing executable validation snapshots. The FFI
constructor configures ephemeral attribution; the scheduler requires a matching
saved validation snapshot. The API exposed editing but no path to obtain/publish
that validation. This is an embedding API gap, not permission to fabricate a test
snapshot or weaken admission.

The integrator accepted a thin bridge before implementation; the canonical plan
records it. EmbeddedRuntime delegates typed requests to the existing service
refresh/publication methods. UniFFI accepts their strict JSON DTOs, maps errors
through the existing workflow envelope, and serializes the validated snapshot
record returned by the owner. Publication derives graph and proof provenance from
server-owned session state. Revision, descriptor and dependency-proof freshness
checks remain unchanged. No parallel validator or snapshot constructor is added.

The public client flow loads a saved graph, creates an edit session, refreshes the
returned revision, checks the submit gate, publishes the returned validation
session for the workflow/version, then runs an execution session. The documented
attribution lifecycle is ephemeral; reopening the runtime requires republishing.
No inference readiness or interactive task execution support is introduced.

## Evidence and remaining qualification

The existing text execution test now obtains its snapshot through that public
flow. New tests cover execution after publication and closing the edit session,
missing publication, loss of publication on runtime reopen followed by successful
republishing, stale graph validation, and rejection of caller-supplied proof
fields. The interactive graph retains an explicit rejection regression at the
current unsupported scheduler-task boundary rather than asserting the obsolete
legacy executor's waiting-input message. UniFFI metadata and generated C# checks
require both new methods.

Rustfmt, shell syntax and staged whitespace are supporting checks only. No local
Rust compilation has been attempted under the shared disk constraint. Independent
read-only source review is required before publication, then exact-head hosted
Rust and generated-binding execution must qualify the bridge. Selector fixtures
are a separate preceding milestone. The managed-runtime readiness fixture still
requires current evidence; aggregate lint, Clippy and audit remain unresolved.

## Independent review correction

Historical checkpoint `0e5496ce362002d11379d3771b952beb02d0eaa6` remains frozen
in its original checkout. Review found two blocking gaps: inference-only records
unconditionally required a node, so pure text could not be published; and actual
C# callers still attempted execution without publication. No wrapper ownership,
strict DTO or newly introduced race bypass was found.

The integrator approved a domain correction: structural validation permits zero
inference nodes, while publication and scheduler projection require exact equality
with the canonical graph's inference-node set. The inference request owner exports
a shared iterator for its existing llm-inference category. Other inference-bearing
node types remain Unsupported in the scheduler classifier, with a regression for
that fail-closed behavior. Duplicate snapshot and graph inference IDs are rejected;
missing/extra projection IDs are reported. Runtime-node proof validation and all
revision/fingerprint/version/dependency freshness checks stay in force.

The public scheduler projection method now requires a graph argument and enforces
coverage itself, including for imported/deserialized records. The execution owner
passes its actual loaded graph; publication passes the server-owned edit graph.
No new caller flag, alternate snapshot constructor or schema escape is added.
Regressions cover empty owner publication, valid runtime coverage, missing/extra/
duplicate projections and deserialized empty records presented for runtime graphs.
The stored-attribution fixture now actually includes its claimed inference node.

NativeSmoke and DirectRuntimeQuickstart now load/edit/refresh/publish through the
new generated methods, check submit permission and publication identity, and close
the edit session before execution. Symbol checks remain supporting gates, not a
claim that the C# workflow ran. Both real programs require hosted generation,
compilation and execution. This correction is based on the separately published
selector/readiness milestones; their tests have passed in hosted runs. Independent
re-review and all fresh Rust/C# qualification remain pending.

Independent narrow re-review accepted correction tree
`80666d31216c3286eec923bd4ee96e8a58bab05e`: exact graph coverage at owner
construction and graph-required public projection resolves the zero-inference
blocker, and both actual C# callers use publication before execution. Missing,
extra, duplicate and deserialized-empty regressions were inspected; unsupported
inference-bearing kinds remain fail-closed. No new source blocker was found.
This report records bounded source acceptance, not hosted execution evidence.

## Hosted attribution-fixture follow-up

Initial PR #15 head `8a8dcbcc0a58aafc0f8e09d7853078856ad78834` compiled and
ran the workflow library suite: 880 passed, two failed. Exact coverage rejected
pre-existing synthetic `infer` records attached to pure text graphs in the
snapshot-ordering and client/bucket attribution fixtures. Their behavioral
assertions remain unchanged. Both fixtures now create a graph session, refresh
owner validation and publish through the real service API, then close the edit
session; they no longer invent an inference node to satisfy the old nonempty
snapshot rule. Runtime-inference fixtures retain their actual runtime proofs.
Rustfmt/whitespace pass; narrow review and fresh hosted qualification are pending.
Independent narrow source review accepted follow-up tree
`255079840fae85689cc6ff5b508fe3bc66be0e8b`: both fixtures now use the public
owner validation flow, with all behavioral assertions unchanged and no production
change. Fresh hosted full-library and downstream C# qualification remain required.
