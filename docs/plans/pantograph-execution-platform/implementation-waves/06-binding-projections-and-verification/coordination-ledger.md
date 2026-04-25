# Coordination Ledger: 06 Binding Projections And Verification

## Status

In progress.

## Wave Status

| Wave | Status | Integration Notes |
| ---- | ------ | ----------------- |
| `wave-01` | Complete | Stage-start outcome recorded as `ready_with_recorded_assumptions`; C# is candidate supported, Python is unsupported, and BEAM is experimental until support-tier docs and host smoke coverage align. |
| `wave-02` | Complete locally | UniFFI and Rustler discovery projections are integrated locally; commits are blocked by read-only `.git`. |
| `wave-03` | Complete locally | C# generated-artifact smoke passed; Python remains unsupported; BEAM source smoke coverage is added but host smoke is blocked by missing `mix`. |
| `wave-04` | Pending | Host-owned integration and gate. |

## Worker Reports

| Worker | Report Path | Status |
| ------ | ----------- | ------ |
| uniffi-projections | `reports/wave-02-worker-uniffi-projections.md` | Complete locally, uncommitted because `.git` is read-only. |
| rustler-projections | `reports/wave-02-worker-rustler-projections.md` | Complete locally, BEAM smoke blocked by missing `mix`. |
| csharp-host-tests | `reports/wave-03-worker-csharp-host-tests.md` | Complete locally, uncommitted because `.git` is read-only. |
| python-host-tests | `reports/wave-03-worker-python-host-tests.md` | Complete as unsupported-lane reconciliation. |
| beam-host-tests | `reports/wave-03-worker-beam-host-tests.md` | Source coverage complete locally; host smoke blocked by missing `mix`. |

## Decisions

- 2026-04-25: Implementation proceeds sequentially in this session. The wave
  plan permits parallel UniFFI/Rustler work, but no subagents were authorized,
  and `.git` is currently mounted read-only so atomic worker integration commits
  are unavailable.
- 2026-04-25: C# over the `pantograph_headless` UniFFI runtime is the only
  candidate supported host lane at stage start because it has dotnet-backed
  generated-artifact smoke and packaged quickstart commands. Full
  graph-authoring support still depends on additive backend-owned discovery and
  port-option projections.
- 2026-04-25: Python host bindings are unsupported at stage start because no
  `bindings/python/` package, generated artifact, or language-native import/load
  smoke exists.
- 2026-04-25: Elixir/BEAM remains experimental at stage start. The repository
  has `./scripts/check-rustler-beam-smoke.sh`, but the current host lacks
  `mix`, and the broad Rustler `Supported` row must be narrowed or backed by
  expanded BEAM host verification.
- 2026-04-25: UniFFI graph-authoring discovery projection uses existing
  backend registries and JSON response shapes instead of adding wrapper-local
  node DTO semantics. The method surface is additive on `FfiPantographRuntime`
  so generated host bindings can consume registry facts without hand-edited
  artifacts.
- 2026-04-25: Rustler graph-authoring discovery projection uses
  `pantograph-workflow-service` `NodeRegistry` for node-definition JSON and
  backend `node-engine` queryable port providers for port discovery. The broad
  Rustler support-tier row is downgraded to `Experimental` pending host smoke
  coverage.
- 2026-04-25: Wave `03` keeps Python `unsupported` because no real Python
  host-binding package, generated artifact, or import/load smoke exists.
  C# remains the only candidate supported non-Rust host lane after generated
  artifact and packaged quickstart verification. BEAM remains `experimental`
  until its host smoke can run with `mix`.

## Verification Results

- 2026-04-25 Wave `01` preflight read the Stage `06` plan, stage-start gate,
  binding-platform plan, wrapper READMEs, host harness READMEs, scripts
  inventory, and `docs/headless-native-bindings.md`.
- 2026-04-25 Host command availability: `/usr/bin/dotnet` version `10.0.103`
  exists; `/usr/bin/python3` version `3.12.3` exists; `mix` is not on `PATH`.
- 2026-04-25 UniFFI discovery projection verification passed:
  `cargo test -p pantograph-uniffi direct_runtime_exposes_backend_owned_graph_authoring_discovery`
  and `cargo test -p pantograph-uniffi`.
- 2026-04-25 UniFFI/C# artifact verification passed:
  `./scripts/check-uniffi-embedded-runtime-surface.sh`,
  `./scripts/check-uniffi-csharp-smoke.sh`,
  `PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh`,
  and `./scripts/check-packaged-csharp-quickstart.sh`.
- 2026-04-25 Rustler discovery projection verification passed:
  `cargo check -p pantograph_rustler`.
- 2026-04-25 Rustler host verification gaps: `cargo test -p pantograph_rustler`
  still fails on host-supplied Erlang `enif_*` link symbols, and
  `./scripts/check-rustler-beam-smoke.sh` fails because `mix` is not installed
  on this machine.
- 2026-04-25 Workspace verification passed after Wave `02` and Wave `03`
  changes: `cargo check --workspace --all-features`. The command emitted
  existing `pumas-library` dead-code warnings outside this repo's current
  change set.
- 2026-04-25 Final broad verification passed after Wave `03` report updates:
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` and
  `cargo test --workspace --doc`. Doctests emitted Cargo's expected note that
  doc tests are not supported for the Rustler `cdylib` crate type.
