# bindings/csharp

## Purpose
Compile-time smoke coverage for generated Pantograph C# UniFFI bindings.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Pantograph.NativeSmoke/` | Small C# source harness that compiles against generated bindings and references the direct `FfiPantographRuntime` surface. |

## Usage
Run the repository-level smoke script:

```bash
./scripts/check-uniffi-csharp-smoke.sh
```

The script builds the Rust UniFFI library, generates
`target/uniffi/csharp/pantograph_uniffi.cs` with `uniffi-bindgen-cs`, and
compiles the smoke harness against that generated file.

## Constraints
- Do not hand-edit generated C# bindings.
- Keep generated binding output under `target/` or another ignored build
  artifact directory.
- Keep application/product C# code out of this smoke harness.
- Keep the smoke compile offline: this directory must not need NuGet packages
  to prove that the generated binding names are present.
