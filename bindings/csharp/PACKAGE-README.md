# Pantograph C# Bindings

This package contains generated C# UniFFI bindings for the Pantograph headless
native runtime.

## Contents

| Path | Purpose |
| ---- | ------- |
| `bindings/csharp/pantograph_uniffi.cs` | Generated C# binding. Add it to your C# project. |
| `examples/csharp/Pantograph.DirectRuntimeQuickstart/` | Minimal direct-runtime console example. |
| `docs/headless-native-bindings.md` | Binding contract, runtime lifecycle, packaging notes, and loader guidance. |
| `manifest.json` | Machine-readable package summary. |

## Required Native Library

Download the matching `pantograph-native-runtime-<platform>.zip` artifact from
the same CI run or release. Put the native library next to your application
binary or on the platform's native-library search path.

Do not mix a generated C# file from one Pantograph build with a native library
from another Pantograph build.

## Minimal Usage

```csharp
using uniffi.pantograph_uniffi;

using FfiPantographRuntime runtime = await FfiPantographRuntime.New(
    new FfiEmbeddedRuntimeConfig(
        appDataDir: "/tmp/pantograph/app-data",
        projectRoot: "/tmp/pantograph/project",
        workflowRoots: []),
    pumasApi: null);

string createResponse = await runtime.WorkflowCreateSession(
    """{"workflow_id":"my-workflow","keep_alive":true}""");
```

See `examples/csharp/Pantograph.DirectRuntimeQuickstart/` for an end-to-end
save/list/load/edit/session-run example.
