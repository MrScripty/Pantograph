# Issues

- T-01 (bounded support): distribution presence does not certify versions,
  imports or model compatibility. Owner: dependency/readiness adapters. All
  Pumas-declared model constraints are refused by this narrow profile. Revisit
  when a model needing those declarations is selected; do not drop their pins.
- T-02 (blocking T3): trained-model access is unavailable after proxy HTTP 403;
  verified trained weights/config/tokenizer/license inputs are missing. Owner:
  CPU text integration delegate; parent coordinates model access. SoundFile was
  installed from official PyPI on 2026-10-19 and actual CPU imports pass. No
  weights were downloaded. Historical no-install scope applied to the adapter
  repair; the current delegation authorizes normal setup.
  Resume only with sanctioned model access, verified license/revision/resource
  bounds and satisfied actual worker prerequisites. No GPU/desktop substitution.
- T-03 (resolved): foreign process Python could supply readiness observations
  for linked PyO3 execution. The exact text profile now observes the linked
  interpreter; malformed/foreign profiles cannot fall through to process probes.
  Seven focused regressions and independent review cover the repair.
- T-04 (resolved): a default Check initially projected embedded observations as
  host Python. Full snapshot tests now prove the same embedded environment is
  retained for both default and explicit checks.

- T-05 (blocking T4): actual public CPU probe runtime cancellation and corrupt
  model loading both release resources, but the public workflow call loses the
  cause and returns `InvalidRequest` / `TerminalFailed`. Canonical diagnostics
  retain runtime cancellation and `pytorch_worker_model_load_failed` header
  deserialization failure. Owner: workflow-service runtime finalization; parent
  coordinates with scheduler M3 before a separate source change. Relevant owners:
  `workflow/runtime_branch_run_finalization.rs` and related batch finalization.
  No fairness policy or shared finalization source is changed in this slice.
  Runtime-effects observation may pass while public projection remains false.
