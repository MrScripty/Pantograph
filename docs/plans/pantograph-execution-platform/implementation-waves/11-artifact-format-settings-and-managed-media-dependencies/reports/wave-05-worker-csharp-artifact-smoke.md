# Wave 05 Worker C# Artifact Smoke Report

## Scope
- Updated the opt-in C# native diffusion smoke to require image outputs as ArtifactStore descriptor objects.
- Replaced inline base64/data URL decoding with a generated UniFFI `WorkflowReadArtifactBody` call.
- Kept text smoke behavior unchanged.
- Added a small C# README note documenting that diffusion image transport uses ArtifactStore descriptors and body reads.

## Changed Files
- `bindings/csharp/Pantograph.NativeSmoke/Program.cs`
- `bindings/csharp/README.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/reports/wave-05-worker-csharp-artifact-smoke.md`

## Verification
- `./scripts/check-uniffi-csharp-smoke.sh`

The script rebuilt `pantograph-uniffi`, regenerated `target/uniffi/csharp/pantograph_headless.cs`, compiled the C# native smoke harness, and passed the default text runtime smoke.

## Residual Risks
- The real diffusion smoke path was not run because this worker did not have a configured `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH` and Python diffusion environment.
- The artifact body is still transported through UniFFI JSON as a byte-number array because that is the current generated binding surface. This smoke now verifies the descriptor/body-read contract, but it does not prove a future non-JSON binary transport.
