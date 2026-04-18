# bindings/beam/pantograph_native_smoke

## Purpose
Minimal Mix/ExUnit smoke harness for loading the Pantograph Rustler NIF from a
local build artifact and exercising a small public contract surface.

## Covered Surface
- NIF load through `:erlang.load_nif/2`
- `version/0`
- `workflow_new/2`
- `workflow_from_json/1`
- `workflow_add_edge/5`
- `workflow_validate/1`

## Required Environment
- Elixir and Mix installed locally or in CI
- `PANTOGRAPH_RUSTLER_NIF_PATH` set to the compiled NIF path or base name,
  for example `target/debug/libpantograph_rustler.so`

## Notes
- This harness intentionally avoids Hex dependencies.
- JSON assertions stay string-based here so the harness remains runnable in
  offline environments without adding parser packages.
- The local `Pantograph.Native` shim defines generated NIF stubs for the full
  default Rustler export surface so `:erlang.load_nif/2` can load the compiled
  library before individual smoke tests call the narrower contract under test.
- This harness now also pins a backend-owned validation error path by
  asserting that unknown edge endpoints round-trip back to BEAM as validation
  strings through `workflow_validate/1`.
- This harness also pins a wrapper-visible parse failure path by asserting
  that malformed workflow JSON returns an `{:error, message}` tuple through
  `workflow_from_json/1`.
- Broader workflow-event and error-envelope coverage can be added here as the
  dedicated Rustler contract extraction work lands.
