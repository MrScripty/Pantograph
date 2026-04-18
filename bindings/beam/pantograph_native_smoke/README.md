# bindings/beam/pantograph_native_smoke

## Purpose
Minimal Mix/ExUnit smoke harness for loading the Pantograph Rustler NIF from a
local build artifact and exercising a small public contract surface.

## Covered Surface
- NIF load through `:erlang.load_nif/2`
- `version/0`
- `workflow_new/2`
- `workflow_from_json/1`

## Required Environment
- Elixir and Mix installed locally or in CI
- `PANTOGRAPH_RUSTLER_NIF_PATH` set to the compiled NIF path or base name,
  for example `target/debug/libpantograph_rustler.so`

## Notes
- This harness intentionally avoids Hex dependencies.
- JSON assertions stay string-based here so the harness remains runnable in
  offline environments without adding parser packages.
- Broader workflow-event and error-envelope coverage can be added here as the
  dedicated Rustler contract extraction work lands.
