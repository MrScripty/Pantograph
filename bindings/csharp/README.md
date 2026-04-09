# bindings/csharp

## Purpose
Runtime smoke coverage for generated Pantograph C# UniFFI bindings.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Pantograph.NativeSmoke/` | Small C# source harness that loads the native library through generated bindings and runs direct `FfiPantographRuntime` session-create/session-run/session-close smokes. |

## Usage
Run the repository-level smoke script:

```bash
./scripts/check-uniffi-csharp-smoke.sh
```

To run the opt-in diffusion path through generated C#, the embedded Rust
runtime, the process Python adapter, and the real torch/diffusers worker:

```bash
PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH=/path/to/tiny-sd-turbo \
  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID=diffusion/cc-nms/tiny-sd-turbo \
  PANTOGRAPH_PYTHON_EXECUTABLE=.venv/bin/python \
  ./scripts/check-uniffi-csharp-diffusion-smoke.sh
```

The script builds the Rust UniFFI library, generates
`target/uniffi/csharp/pantograph_uniffi.cs` with `uniffi-bindgen-cs`, compiles
the smoke harness against that generated file, and runs the harness with the
native library on the dynamic-linker path.

## Constraints
- Do not hand-edit generated C# bindings.
- Keep generated binding output under `target/` or another ignored build
  artifact directory.
- Keep application/product C# code out of this smoke harness.
- Keep the smoke compile offline: this directory must not need NuGet packages
  to prove that the generated binding names are present.
- Keep the default runtime smoke model-free.
- Keep real-model image acceptance opt-in and explicitly configured with a
  caller-supplied Puma-Lib node selection and Python executable.
- Keep runtime execution smokes session-first: create a workflow session before
  submitting workflow runs.
