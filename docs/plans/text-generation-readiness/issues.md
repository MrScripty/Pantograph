# Issues

- T-01 (bounded support): distribution presence does not certify versions,
  imports or model compatibility. Owner: dependency/readiness adapters. All
  Pumas-declared model constraints are refused by this narrow profile. Revisit
  when a model needing those declarations is selected; do not drop their pins.
- T-02 (blocking T3): trained-model access is unavailable after proxy HTTP 403;
  the development venv also lacks SoundFile. Owner: external M2 qualification
  worker. No package installs or weight downloads occurred in this repair.
  Resume only with sanctioned model access, verified license/revision/resource
  bounds and satisfied actual worker prerequisites. No GPU/desktop substitution.
- T-03 (resolved): foreign process Python could supply readiness observations
  for linked PyO3 execution. The exact text profile now observes the linked
  interpreter; malformed/foreign profiles cannot fall through to process probes.
  Seven focused regressions and independent review cover the repair.
- T-04 (resolved): a default Check initially projected embedded observations as
  host Python. Full snapshot tests now prove the same embedded environment is
  retained for both default and explicit checks.
