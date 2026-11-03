# bindings/csharp

## Purpose
Runtime smoke coverage for generated Pantograph C# bindings.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Pantograph.NativeSmoke/` | Small C# source harness that loads the native library through generated bindings and runs direct `FfiPantographRuntime` backend discovery plus session-create/session-run/session-close smokes. |
| `Pantograph.DirectRuntimeQuickstart/` | Artifact-ready quickstart showing native save/list/load/edit-session/workflow-session usage from C#. |
| `PACKAGE-README.md` | README copied to the generated C# binding artifact. |

## Generator prerequisite

The maintained workflow uses UniFFI 0.28.3 and the official
[uniffi-bindgen-cs v0.9.0+v0.28.3 source](https://github.com/NordSecurity/uniffi-bindgen-cs/tree/2f4880f03ed08d960ad0ee14d11cf95444eee540).
The generator is not published as crates.io version 0.9.0. Install its exact
reviewed Git revision with the upstream lockfile:

```bash
cargo install uniffi-bindgen-cs \
  --git https://github.com/NordSecurity/uniffi-bindgen-cs \
  --rev 2f4880f03ed08d960ad0ee14d11cf95444eee540 --locked
```

This provisions a development/CI tool; it does not change the application's
UniFFI dependency or hand-edit generated bindings. The headless workflow owns
its pin and generated-artifact qualification. Keep the upstream MPL-2.0 notices
with any redistributed generator source/binary; this repository does not vendor
or package the generator itself. Existing generated-binding/artifact licensing
obligations remain unchanged.

## Shutdown result

Await `runtime.Shutdown()` and handle the generated `FfiException` if the
owned backend cannot stop. Its message contains the standard JSON error envelope
with `internal_error` and the original shutdown cause. A failed stop does not
mean the runtime released its residency; callers may retry after resolving the
cause. Existing C# await expressions remain valid, but regenerate bindings and
ship them with the matching native library when adopting this fallible API.
Rust callers now handle `Result<(), FfiError>` explicitly.

## Usage
Run the repository-level smoke script:

```bash
./scripts/check-uniffi-csharp-smoke.sh
```

To run the opt-in diffusion path through generated C#, the embedded Rust
runtime, the process Python adapter, and the real torch/diffusers worker:

```bash
PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID=diffusion/cc-nms/tiny-sd-turbo \
  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID=diffusers \
  PANTOGRAPH_PYTHON_EXECUTABLE=.venv/bin/python \
  ./scripts/check-uniffi-csharp-diffusion-smoke.sh
```

The script builds the Pantograph headless native library, generates
`target/uniffi/csharp/pantograph_headless.cs` with `uniffi-bindgen-cs`, compiles
the smoke harness against that generated file, and runs the harness with the
native library on the dynamic-linker path.

The diffusion smoke expects image outputs to be ArtifactStore descriptors and
loads image bytes through the generated `WorkflowReadArtifactBody` UniFFI API.
It does not accept inline image base64 or data URLs.

To create local zip artifacts matching CI:

```bash
./scripts/package-uniffi-csharp-artifacts.sh
```

The packaging script writes:

- `target/bindings-package/artifacts/pantograph-csharp-bindings.zip`
- `target/bindings-package/artifacts/pantograph-headless-native-<platform>.zip`
- `target/bindings-package/artifacts/checksums-sha256.txt`

To compile the artifact-ready quickstart against the packaged generated C# and
run it against the packaged native library without NuGet/network restore:

```bash
./scripts/check-packaged-csharp-quickstart.sh
```

## Constraints
- Do not hand-edit generated C# bindings.
- Keep generated binding output under `target/` or another ignored build
  artifact directory.
- Keep application/product C# code out of this smoke harness.
- Keep the smoke compile offline: this directory must not need NuGet packages
  to prove that the generated binding names are present.
- Keep the default runtime smoke model-free.
- Keep real-model image acceptance opt-in and explicitly configured with a
  caller-supplied Puma-Lib model reference and Python executable.
- Keep runtime execution smokes session-first: create a workflow session before
  submitting workflow runs.
