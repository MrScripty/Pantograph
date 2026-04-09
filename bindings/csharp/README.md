# bindings/csharp

## Purpose
Runtime smoke coverage for generated Pantograph C# UniFFI bindings.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Pantograph.NativeSmoke/` | Small C# source harness that loads the native library through generated bindings and runs a direct `FfiPantographRuntime` workflow/session round trip. |

## Usage
Run the repository-level smoke script:

```bash
./scripts/check-uniffi-csharp-smoke.sh
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
- Keep runtime smoke workflows small and model-free. Real-model image
  acceptance belongs in a separate model/runtime test.
