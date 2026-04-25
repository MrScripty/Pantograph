# Wave 03 Worker Report: beam-host-tests

## Status

Source coverage committed; host smoke blocked by missing `mix`.

## Scope

- `bindings/beam/pantograph_native_smoke`
- `crates/pantograph-rustler`
- BEAM support-tier documentation
- Stage `06` implementation notes and coordination ledger

## Work Completed

- Added BEAM smoke stubs for the Rustler graph-authoring discovery NIFs:
  - `node_registry_list_definitions`
  - `node_registry_get_definition`
  - `node_registry_definitions_by_category`
  - `node_registry_queryable_ports`
- Added BEAM smoke assertions for backend-owned node definitions, grouped
  definitions, and queryable-port discovery.
- Downgraded the broad BEAM/Rustler support-tier language from `Supported` to
  `Experimental` until host smoke coverage can be run on a machine with the
  BEAM toolchain.

## Verification

Passed:

```bash
cargo check -p pantograph_rustler
```

Blocked on this host:

```bash
./scripts/check-rustler-beam-smoke.sh
```

The command fails immediately because `mix` is not installed on `PATH`.

Known Rust unit-test limitation:

```bash
cargo test -p pantograph_rustler
```

The command still fails at link time because Erlang `enif_*` symbols are
provided by the BEAM host runtime.

## Deviations

The BEAM lane remains `experimental` for Stage `06` on this host. The source
smoke fixture was expanded, but the language-native host command could not be
executed here.
