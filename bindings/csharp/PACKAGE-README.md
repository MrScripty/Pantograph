# Pantograph C# Bindings

This package contains generated C# bindings for the Pantograph headless native
library.

## Contents

| Path | Purpose |
| ---- | ------- |
| `bindings/csharp/pantograph_headless.cs` | Generated C# binding. Add it to your C# project. |
| `examples/csharp/Pantograph.DirectRuntimeQuickstart/` | Minimal direct-runtime console example. |
| `docs/headless-native-bindings.md` | Binding contract, runtime lifecycle, packaging notes, and loader guidance. |
| `manifest.json` | Machine-readable package summary. |

## Required Native Library

If you do not already have the matching Pantograph native shared library,
download `pantograph-headless-native-<platform>.zip` from the same CI run or
release. Put the native library next to your application binary or on the
platform's native-library search path.

If you already ship the matching `libpantograph_headless` /
`pantograph_headless.dll` library for the same build, you only need this
binding package.

Do not mix a generated C# file from one Pantograph build with a native library
from another Pantograph build.

If you used pre-refactor artifacts, replace the old `pantograph_uniffi.cs`
generated file and `using uniffi.pantograph_uniffi;` namespace with
`pantograph_headless.cs` and `using uniffi.pantograph_headless;`.

## Minimal Usage

```csharp
using uniffi.pantograph_headless;

using FfiPantographRuntime runtime = await FfiPantographRuntime.New(
    new FfiEmbeddedRuntimeConfig(
        appDataDir: "/tmp/pantograph/app-data",
        projectRoot: "/tmp/pantograph/project",
        workflowRoots: [],
        maxLoadedSessions: null),
    pumasApi: null);

string createResponse = await runtime.WorkflowCreateSession(
    """{"workflow_id":"my-workflow","keep_alive":true}""");
```

See `examples/csharp/Pantograph.DirectRuntimeQuickstart/` for an end-to-end
save/list/load/edit/session-run example.
