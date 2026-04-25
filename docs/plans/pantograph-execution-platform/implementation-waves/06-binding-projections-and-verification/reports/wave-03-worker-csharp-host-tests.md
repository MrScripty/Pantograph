# Wave 03 Worker Report: csharp-host-tests

## Status

Complete locally.

## Scope

- `bindings/csharp/Pantograph.NativeSmoke`
- `bindings/csharp/Pantograph.DirectRuntimeQuickstart`
- UniFFI C# packaging and packaged quickstart smoke scripts
- Stage `06` implementation notes and coordination ledger

## Work Completed

- Extended the C# native smoke path to exercise generated access to
  backend-owned graph-authoring discovery:
  - `WorkflowGraphListNodeDefinitions`
  - `WorkflowGraphGetNodeDefinition`
  - `WorkflowGraphGetNodeDefinitionsByCategory`
  - `WorkflowGraphGetQueryablePorts`
- Kept C# as a generated binding consumer; no generated C# artifact was
  hand-edited.
- Fixed current quickstart and package README constructor samples for the
  generated `FfiEmbeddedRuntimeConfig` signature by passing
  `maxLoadedSessions: null`.

## Verification

Passed:

```bash
./scripts/check-uniffi-csharp-smoke.sh
PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh
./scripts/check-packaged-csharp-quickstart.sh
```

## Deviations

None. The C# lane loads the real generated/native artifact and remains the only
candidate supported non-Rust host lane for this stage.
