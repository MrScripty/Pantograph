# Inspector snapshot normalization on verified main

The local candidate branch is `integration/inspector-snapshot-main-2026-10-07`,
created from freshly fetched main `a8483e511dcec4f36e269e6e4debf181a318222f`,
tree `e0d26d872866cfeec4244cdbc72bb1475efd47a3`. The qualified source
`2590a5337fe30688c8cd29dc60aa2cb518ad9422`, its branch and all successful and
failed native evidence remain preserved. No applicable AGENTS or local skill
instructions were found in this checkout.

## Production change and dependencies

Rust's existing inspection response omits empty `node_statuses`, `io_artifacts`
and `retention_summary` vectors. The frontend treated these as present arrays,
which caused graph presentation to throw when `node_statuses` was omitted.
`WorkflowProjectionService.queryRunInspection` now describes those wire fields
as optional and normalizes them with `?? []`. It preserves graph, projection and
populated collection identities and does not manufacture node completion status.
This is wire omission normalization, not general runtime schema validation.

Only the qualified service change and its two inspection regression tests are
carried. The regression covers omitted collections reaching the graph presenters,
populated collection preservation, exact request scope and an empty second run
without leaked prior-run state. Main's existing frontend test glob discovers it.

The complete main-to-qualified comparison has 967 changed paths: 904 evidence,
12 qualification, six documentation, and 45 source or existing gate paths.
The full binary patch, inventory and complete source patch are preserved locally
under `/workspace/qualification-evidence/inspector-main-integration`.
Independent read-only dependency audit verified that the Rust inspection contract,
fixture, inspector graph component, presenters, types, IPC/error helpers and service
inheritance are byte-identical between these sources. `resolved_node_io` already
has an optional frontend type and a consumer fallback. No additional dependency
is needed for the normalization fix.

Excluded changes include the inspector's QA run-header marker, SVG pointer locator
and diagnostics, synthetic fixtures, workflow automation, graph editor/session
repairs, runtime enrollment/discovery, Pumas requirement bootstrap and weight-target
loading, dependency planning, scheduler/downstream progression and native diagnostics.
Main's newer chat/seed capture tests and CI gates are retained, rather than reversed
by the older qualification fork. The qualified `startup.rs` difference adds a test;
it does not represent an absent production startup change. Scheduler, rerank and
audio implementation remain owned by the other worker and are untouched here.

## Verification

- Both carried regressions fail against unmodified main, with `undefined !== []`.
  With the repair, all 672 frontend tests pass, including both regressions.
- Full TypeScript check, frontend production build, changed-source ESLint,
  critical anti-pattern and accessibility gates pass; the latter includes 27
  structural role/button regressions. Git whitespace and staged decision
  traceability checks pass.
- A controlled replay of the original failed native inspection response reproduces
  the baseline presenter exception. The candidate service/presenters accept its
  four nodes, three edges, three artifacts and current sequence 12 without
  mutating the captured response or inferring node completion.
- Before the frontend build, the complete locked, offline desktop feature graph
  was inspected. ORT remains dynamic (`load-dynamic`, `disable-linking`), download
  features are absent, and Pumas remains pinned to
  `26a84e323cae566a46a8f76bef48fa1010aed48b`. `ORT_SKIP_DOWNLOAD=1` was set.
  No Cargo build, model/runtime download or sandbox change was performed.

## Native evidence applicability and remaining qualification

[Native run 37582221446](https://github.com/MrScripty/Pantograph/actions/runs/37582221446)
passed on qualified source `2590a5337fe30688c8cd29dc60aa2cb518ad9422`, tree
`61558584d2036bbd13c665822d3cfc297cf0b161`. Artifact `11466235180` has SHA-256
`67bc5a049bfedaf28beec6d816731d0ad19ecb27ec870a6f5c741471eacb3d58`.
It proves that the same normalized inspection service and unchanged inspector
consumers load the graph, accept a native body click, expose the scoped vector
card, use Read and display eight finite values equal to the retained body.
The screenshot shows the vector selection and preview but horizontally clips
some text; complete equality is evidenced by the displayed DOM preview assertion.
Failed runs `37576570678` and `37580177402` remain preserved separately.

That evidence supports this defect repair, but it does not qualify this new
main-based tree end to end. Main differs in other runtime/backend prerequisites
and does not contain the qualification workflow. A fresh qualifier based on the
exact eventual integration tree is required before claiming native CPU graph
acceptance for that tree. Keep its fixtures/automation and any separately reviewed
runtime dependencies in a distinct qualification candidate. No fresh native run,
public write, PR, main change or merge is authorized or performed here.
Controlled replay and frontend checks do not establish mounted native rendering,
real-user model discovery, GPU, pretrained model or full production loader acceptance.
