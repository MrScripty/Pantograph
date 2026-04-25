# Wave 02 Worker Report: rustler-projections

## Status

Complete locally. Commit is blocked because `.git` is mounted read-only.

## Scope

- `crates/pantograph-rustler/src/lib.rs`
- `crates/pantograph-rustler/src/registry_nifs.rs`
- `crates/pantograph-rustler/README.md`
- `bindings/beam/README.md`
- `bindings/beam/pantograph_native_smoke/`
- Stage `06` implementation notes and coordination ledger

## Changes

- Added BEAM-visible NIFs for backend-owned graph authoring discovery:
  - `node_registry_list_definitions`
  - `node_registry_get_definition`
  - `node_registry_definitions_by_category`
  - `node_registry_queryable_ports`
- Kept node-definition projection sourced from `pantograph-workflow-service`
  `NodeRegistry`, not Rustler-local task metadata.
- Kept queryable-port discovery sourced from the backend `node-engine`
  registry after built-in provider registration.
- Extended the BEAM smoke harness stubs and tests to cover node definitions,
  grouped definitions, and queryable ports through the real NIF.
- Downgraded the broad Rustler support-tier documentation to `Experimental`
  until host smoke coverage justifies a supported BEAM surface.

## Verification

```bash
cargo check -p pantograph_rustler
```

Attempted but not passing in this environment:

```bash
cargo test -p pantograph_rustler
./scripts/check-rustler-beam-smoke.sh
```

`cargo test -p pantograph_rustler` still fails at link time on host-supplied
Erlang `enif_*` symbols, matching the existing crate README limitation.
`./scripts/check-rustler-beam-smoke.sh` fails before build because `mix` is not
available on `PATH` in this environment.

## Notes

- No generated artifacts were edited.
- BEAM remains experimental until the smoke harness is run in an environment
  with Mix/Elixir/Erlang and the supported surface is either narrowed further
  or covered end to end.
