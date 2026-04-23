# bindings/beam

## Purpose
Host-side BEAM smoke and acceptance harnesses for Pantograph Rustler bindings.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `pantograph_native_smoke/` | Minimal Mix/ExUnit harness that loads the compiled `pantograph_rustler` NIF and exercises a small BEAM-visible contract surface. |

## Usage
Build the Rustler NIF first:

```bash
cargo build -p pantograph_rustler
```

Then point the smoke harness at the built NIF base path and run the host-side
tests:

```bash
PANTOGRAPH_RUSTLER_NIF_PATH="$(pwd)/target/debug/libpantograph_rustler.so" \
  mix test
```

The repository also provides a canonical runner that performs the build,
toolchain checks, and harness invocation together:

```bash
./scripts/check-rustler-beam-smoke.sh
```

The harness strips any platform extension before calling
`:erlang.load_nif/2`, so callers may provide either the full library filename
or the extensionless base path.

## Constraints
- Keep this directory host-harness-only; do not move canonical workflow
  semantics into BEAM code.
- Keep the smoke project dependency-light and offline-friendly.
- Keep host-side tests focused on NIF loading and BEAM-visible contract
  behavior, not on duplicating backend Rust tests.
- Keep environment and artifact paths explicit so repeated runs do not depend
  on hidden global state.
