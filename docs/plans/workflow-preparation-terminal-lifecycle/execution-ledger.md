# Execution ledger

- 2026-10-20 parent delegated M4 preparation-refusal lifecycle gap. Verified shell,
  clean prior worktrees and fresh public main 4aec0d5c. Reported c25fd49d is not a
  local object. Fetched only public M3/M4 refs, not blocked checkpoint/payload.
  Created isolated worktree/branch from public 2da63ae9; no actual cohort.
- No AGENTS.md exists in workspace/project. Standards revision 0fc92a24 applies:
  Core/Router; implementation, planning, verification/oracles, proportionality,
  commit/docs; Rust API/async/library/tooling; contracts/concurrency/resilience/
  diagnostics/code ownership. Existing scheduler finalization owners inspected.
- Independent text_review identified ambiguous preparation errors after Start,
  attempt-to-release-intent versus native ACK distinction, duplicate responder
  ownership, deferred semantics and atomic guard/finish requirement. Refined
  guarded-driver design admitted; shared terminal-event projection retained.
- Parent subsequently relayed publication approval: PR83 exact 5b4fc6dd is now
  public. Fetched only that public PR ref and verified its 45 added worker lines
  leave the central completion driver/finalizer unchanged. prepare_residency
  refusal reaches that driver before Start/event claim. No upload workaround.
- Controlled public baseline regression failed for the intended reason: expected
  one workflow terminal, observed zero, after expected refusal and zero native/
  reservation port calls. Base production code unchanged in that red run; only
  the regression include/test were added. Raw red log retained externally.
- Implementation shares existing post-finish terminal projection. Registered
  unassigned/no-batch custody and successful event settlement gate the new path;
  matching coherent graph, no Running/attempt/pending cleanup and finish_run
  are checked under one store lock. Genuine deferred responses are untouched.
- Initial new-source compiles identified an unnecessary generic .into() and a
  missed M3 fair-admission argument in test setup. Corrected before qualification;
  neither failed compile is runtime acceptance evidence.
- L1/L2 focused regressions passed 4/4. Full service library passed 1012/1012,
  including the parent's invalid-before-running public-main recipe, cancellation/
  shutdown hold-until-settled and panic fencing. The Waiting readiness fixture
  initially omitted required validation source; added canonical proof/template
  construction so the final observation reaches actual readiness deferral.
- Independent final source/regression review admitted all three current source
  hashes and raw test logs without material findings. The decisive custody test
  uses canonical event claim/dispatch/assignment creation and a moved unique
  ownership token; no cloned/synthetic native ownership proof.
- Staged production/test patch applies cleanly to public PR83 exact 5b4fc6dd via
  temporary-index git apply --cached --check. Its affected owner files equal V2
  source c25fd49d. Structural compatibility is not an M4 native rerun/acceptance.
  Own patch SHA256 a259c5226d666f1070bbcbc746b9e7e1633116f9acf7bd8677a5700bef893daf.
- Default-backend Clippy hit existing inference SelectedTextLoad unused fields
  with warnings denied. No unrelated source suppressed/changed. Rerunning the
  inference/service lib/tests gate with supported inference/backend-pytorch;
  selected-feature gate remains pending here.
- Supported-feature Clippy passed with warnings denied: inference and workflow
  service lib/tests, inference/backend-pytorch enabled. No new dependencies,
  suppressions or unrelated source changes. The default disabled-feature warning
  remains recorded separately; selected gate log clippy-pytorch.log exits 0.
- Before source checkpoint: only the declared six text files are staged; focused
  and full affected suite, source review, affected formatting, staged traceability,
  whitespace and exact PR83 patch-application checks passed. No credentials,
  weights, databases, M4 native/Start policy or unrelated files are staged.
