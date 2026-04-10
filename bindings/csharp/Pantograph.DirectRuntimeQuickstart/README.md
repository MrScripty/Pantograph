# Pantograph.DirectRuntimeQuickstart

This example is copied into the C# binding artifact. It demonstrates the
direct native path:

```text
C# application -> generated C# binding -> Pantograph headless native library -> embedded Rust runtime
```

It does not call HTTP or Tauri.

## Run From An Artifact

1. Extract the C# binding artifact.
2. Extract the Pantograph headless native artifact for your platform.
3. Copy the native library next to your application executable, or put the
   native library directory on the dynamic-linker path.
4. Run:

```bash
dotnet run --project examples/csharp/Pantograph.DirectRuntimeQuickstart \
  -- \
  --project-root /tmp/pantograph-quickstart/project \
  --app-data-dir /tmp/pantograph-quickstart/app-data
```

This default command exercises native workflow authoring. To also execute the
saved workflow, install/configure the Pantograph managed runtimes needed by
your workflow and append `--run-session`.

## What It Exercises

- Instantiates `FfiPantographRuntime`.
- Saves a text workflow through `WorkflowGraphSave`.
- Lists persisted workflows through `WorkflowGraphList`.
- Loads the saved workflow through `WorkflowGraphLoad`.
- Opens a graph edit session, mutates node data, saves the edited graph, and
  closes the graph edit session.
- When `--run-session` is passed: opens a workflow execution session, submits a
  session run, prints outputs, and closes the workflow execution session.
