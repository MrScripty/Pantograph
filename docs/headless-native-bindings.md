# Headless Native Bindings

Pantograph ships a headless native binding surface through the
`pantograph-uniffi` shared library. C# consumers use generated UniFFI C# plus
the native library; they do not call the Pantograph Tauri GUI and do not need a
Pantograph HTTP server.

## Architecture

```text
Host app
  -> generated UniFFI binding
    -> pantograph_uniffi native library
      -> pantograph-embedded-runtime
        -> pantograph-workflow-service
```

The boundary accepts and returns JSON strings for workflow service DTOs. Those
DTOs are the shared Pantograph service contract used by the embedded runtime.

## Required Runtime Flow

Workflow execution clients should use sessions:

1. Create `FfiPantographRuntime`.
2. Inspect available workflow inputs/outputs with `WorkflowGetIo`.
3. Run `WorkflowPreflight`.
4. Create a workflow execution session with `WorkflowCreateSession`.
5. Submit work with `WorkflowRunSession`.
6. Inspect session status or queue state as needed.
7. Close the session with `WorkflowCloseSession`.
8. Call `Shutdown` before process exit.

`WorkflowRun` remains available for compatibility and single-shot host flows,
but interactive applications should treat the session path as the primary
execution path.

## Workflow Authoring Flow

Headless clients can author persisted Pantograph workflows without the GUI:

1. Create or load a graph JSON payload.
2. Save it with `WorkflowGraphSave`.
3. List saved workflows with `WorkflowGraphList`.
4. Load saved workflow files with `WorkflowGraphLoad`.
5. Open an edit session with `WorkflowGraphCreateEditSession`.
6. Mutate nodes/edges or use connection helper methods.
7. Save the returned edited graph when persistence is desired.
8. Close the graph edit session.

Graph edit sessions and workflow execution sessions are separate resources.
Create both deliberately; do not use one ID where the other is expected.

## C# Artifact Layout

The CI C# artifact contains:

```text
bindings/csharp/pantograph_uniffi.cs
docs/headless-native-bindings.md
examples/csharp/Pantograph.DirectRuntimeQuickstart/
README.md
manifest.json
```

The generated C# file is an artifact, not a checked-in source file. Regenerate
it from the same native library that the host app will load.

## Native Runtime Artifact Layout

The CI native runtime artifact contains:

```text
native/<platform>/libpantograph_uniffi.so
native/<platform>/pantograph_uniffi.dll
native/<platform>/libpantograph_uniffi.dylib
docs/headless-native-bindings.md
examples/csharp/Pantograph.DirectRuntimeQuickstart/
README.md
manifest.json
```

Only one native library is present per platform package.

## C# Loading

Generated UniFFI C# resolves the `pantograph_uniffi` native library using the
.NET runtime's normal native-library resolution. For development, the simplest
options are:

```bash
# Linux
export LD_LIBRARY_PATH=/path/to/native/linux-x64:$LD_LIBRARY_PATH

# macOS
export DYLD_LIBRARY_PATH=/path/to/native/osx-arm64:$DYLD_LIBRARY_PATH

# Windows PowerShell
$env:PATH = "C:\path\to\native\win-x64;$env:PATH"
```

Shipping applications can instead copy the platform library to the app output
directory.

## Runtime Dependencies

The native `pantograph_uniffi` library is the Pantograph binding/runtime
facade. It is not a model bundle and it is not the managed llama.cpp/Ollama
runtime bundle.

Before creating sessions for workflows that require model backends, install or
configure the runtime/backend files those workflows require. Use
`WorkflowGetCapabilities` or `WorkflowPreflight` to surface missing backends to
the host application before queueing a session run.

## Python-Backed Nodes

The native Rust library does not embed Python. Python-backed Pantograph nodes
run through the process Python sidecar. Configure a Python executable that has
the required worker dependencies, for example by setting
`PANTOGRAPH_PYTHON_EXECUTABLE`.

Model choices are workflow data. Diffusion workflows should use the `puma-lib`
node to select a Puma-Lib model and connect it to the diffusion node; do not
invent an application-local model path setting.

## Compatibility

Keep the generated C# binding and the native library from the same CI run or
release. UniFFI method metadata is part of the native artifact; mismatching
generated code and native binaries can fail at load time or call time.
