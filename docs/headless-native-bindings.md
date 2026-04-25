# Headless Native Bindings

Pantograph ships a headless native binding surface through the Pantograph
headless shared library. C# consumers use the generated binding file plus the
native library; they do not call the Pantograph Tauri GUI and do not need a
Pantograph HTTP server.

## Architecture

```text
Host app
  -> generated host-language binding
    -> pantograph_headless native library
      -> pantograph-embedded-runtime
        -> pantograph-workflow-service
```

The boundary accepts and returns JSON strings for workflow service DTOs. Those
DTOs are the shared Pantograph service contract used by the embedded runtime.

## Host Binding Direction

The native Rust API is the canonical headless integration contract. C# and
Python bindings are host-language projections over the `pantograph_headless`
product-native library. Elixir/BEAM uses the Rustler lane, but it must still
project the same backend-owned workflow service contracts rather than defining
alternate workflow semantics.

Required direction:

- host bindings must remain thin adapters over backend-owned contracts
- host bindings must not introduce alternate node catalogs, graph semantics, or
  diagnostics semantics
- generated or wrapper host packages must document their support tier,
  platform support, lifecycle expectations, and native-library pairing rules
- Python host bindings are distinct from Python-backed workflow nodes and the
  process Python sidecar
- Elixir is the product-facing host language for the Rustler/BEAM lane; Rustler
  is the wrapper mechanism, not a separate product contract

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

## Graph Authoring Contract Direction

Graph mutation methods are intentionally generic. Host applications should not
need a hand-maintained node catalog or hardcoded composition table every time
Pantograph adds or refines node types.

Required direction for the headless binding contract:

- bindings must expose backend-owned node-definition discovery projected from
  the registry
- bindings must expose backend-owned queryable-port and port-option discovery
  for dynamic authoring surfaces
- host applications should build palettes, inspectors, and insert flows from
  backend-discovered node metadata rather than out-of-band node knowledge

Current surface:

- the direct headless runtime exposes generic graph mutation helpers,
  registry-backed node-definition discovery, category/grouped discovery,
  queryable-port discovery, and port-option queries
- supported external graph-authoring flows should treat these backend-discovered
  facts as the source of truth instead of maintaining a host-local node catalog

## C# Artifact Layout

The CI C# artifact contains:

```text
bindings/csharp/pantograph_headless.cs
docs/headless-native-bindings.md
examples/csharp/Pantograph.DirectRuntimeQuickstart/
README.md
manifest.json
```

The generated C# file is an artifact, not a checked-in source file. Regenerate
it from the same native library that the host app will load.

## Native Library Artifact Layout

The CI native library artifact contains:

```text
native/<platform>/libpantograph_headless.so
native/<platform>/pantograph_headless.dll
native/<platform>/libpantograph_headless.dylib
docs/headless-native-bindings.md
examples/csharp/Pantograph.DirectRuntimeQuickstart/
README.md
manifest.json
```

Only one native library is present per platform package.

## C# Loading

Generated C# resolves the `pantograph_headless` native library using the .NET
runtime's normal native-library resolution. For development, the simplest
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

The native `pantograph_headless` library is the Pantograph headless runtime
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
release. Binding metadata is part of the native artifact; mismatching generated
code and native binaries can fail at load time or call time.

### Migration From Pre-Refactor Artifacts

Earlier headless binding artifacts used the internal `pantograph_uniffi`
identity in generated namespaces, generated file names, native library names,
and native zip names. Current artifacts use the product-facing
`pantograph_headless` identity instead.

For C# consumers:

1. Replace `using uniffi.pantograph_uniffi;` with
   `using uniffi.pantograph_headless;`.
2. Replace `pantograph_uniffi.cs` with the generated
   `pantograph_headless.cs` file from the same Pantograph build.
3. Replace `libpantograph_uniffi.so`, `pantograph_uniffi.dll`, or
   `libpantograph_uniffi.dylib` with the matching `pantograph_headless` native
   library for the platform.
4. Download `pantograph-headless-native-<platform>.zip` instead of the older
   native runtime zip name.
