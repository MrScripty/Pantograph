using System;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;
using uniffi.pantograph_uniffi;

string projectRoot = RequireArgument(args, "--project-root");
string appDataDir = RequireArgument(args, "--app-data-dir");

Directory.CreateDirectory(projectRoot);
Directory.CreateDirectory(appDataDir);

using FfiPantographRuntime runtime = await FfiPantographRuntime.FfiPantographRuntimeAsync(
    new FfiEmbeddedRuntimeConfig(
        appDataDir: appDataDir,
        projectRoot: projectRoot,
        workflowRoots: []),
    pumasApi: null);

try
{
    const string workflowName = "CSharp Native Quickstart";
    const string workflowId = "CSharp Native Quickstart";

    string savedPath = SaveDemoWorkflow(runtime, workflowName);
    InspectSavedWorkflows(runtime);
    await EditWorkflowText(runtime, savedPath);

    if (args.Contains("--run-session", StringComparer.Ordinal))
    {
        await RunWorkflowSession(runtime, workflowId);
    }
    else
    {
        Console.WriteLine(
            "Skipped workflow execution session. Pass --run-session after installing required Pantograph runtimes.");
    }
}
finally
{
    await runtime.Shutdown();
}

static string SaveDemoWorkflow(FfiPantographRuntime runtime, string workflowName)
{
    string response = runtime.WorkflowGraphSave(
        $$"""
        {
          "name": {{JsonSerializer.Serialize(workflowName)}},
          "graph": {
            "nodes": [
              {
                "id": "text-input-1",
                "node_type": "text-input",
                "position": { "x": 0.0, "y": 0.0 },
                "data": {
                  "text": "saved default",
                  "definition": {
                    "category": "input",
                    "io_binding_origin": "client_session",
                    "label": "Text Input",
                    "description": "Provides text input",
                    "inputs": [{
                      "id": "text",
                      "label": "Text",
                      "data_type": "string",
                      "required": false,
                      "multiple": false
                    }],
                    "outputs": [{
                      "id": "legacy-out",
                      "label": "Legacy Out",
                      "data_type": "string",
                      "required": false,
                      "multiple": false
                    }]
                  }
                }
              },
              {
                "id": "text-output-1",
                "node_type": "text-output",
                "position": { "x": 260.0, "y": 0.0 },
                "data": {
                  "definition": {
                    "category": "output",
                    "io_binding_origin": "client_session",
                    "label": "Text Output",
                    "description": "Displays text output",
                    "inputs": [{
                      "id": "text",
                      "label": "Text",
                      "data_type": "string",
                      "required": false,
                      "multiple": false
                    }],
                    "outputs": [{
                      "id": "text",
                      "label": "Text",
                      "data_type": "string",
                      "required": false,
                      "multiple": false
                    }]
                  }
                }
              }
            ],
            "edges": [{
              "id": "edge-text",
              "source": "text-input-1",
              "source_handle": "text",
              "target": "text-output-1",
              "target_handle": "text"
            }]
          }
        }
        """);

    using JsonDocument document = JsonDocument.Parse(response);
    string path = document.RootElement.GetProperty("path").GetString()
        ?? throw new InvalidOperationException($"Missing save path: {response}");

    Console.WriteLine($"Saved workflow file: {path}");
    return path;
}

static void InspectSavedWorkflows(FfiPantographRuntime runtime)
{
    string response = runtime.WorkflowGraphList();
    using JsonDocument document = JsonDocument.Parse(response);
    int count = document.RootElement.GetProperty("workflows").GetArrayLength();
    Console.WriteLine($"Persisted workflow count: {count}");
}

static async Task EditWorkflowText(FfiPantographRuntime runtime, string savedPath)
{
    string workflowFileJson = runtime.WorkflowGraphLoad(
        $$"""{"path":{{JsonSerializer.Serialize(savedPath)}}}""");
    using JsonDocument workflowFile = JsonDocument.Parse(workflowFileJson);
    string graphJson = workflowFile.RootElement.GetProperty("graph").GetRawText();

    string createSessionResponse = await runtime.WorkflowGraphCreateEditSession(
        $$"""{"graph":{{graphJson}}}""");
    string editSessionId = ReadString(createSessionResponse, "session_id");

    string updatedGraphResponse = await runtime.WorkflowGraphUpdateNodeData(
        $$"""
        {
          "session_id": {{JsonSerializer.Serialize(editSessionId)}},
          "node_id": "text-input-1",
          "data": {
            "text": "edited before session run"
          }
        }
        """);

    string updatedGraphJson;
    using (JsonDocument updatedGraph = JsonDocument.Parse(updatedGraphResponse))
    {
        updatedGraphJson = updatedGraph.RootElement.GetProperty("graph").GetRawText();
    }

    runtime.WorkflowGraphSave(
        $$"""
        {
          "name": "CSharp Native Quickstart",
          "graph": {{updatedGraphJson}}
        }
        """);

    await runtime.WorkflowGraphCloseEditSession(
        $$"""{"session_id":{{JsonSerializer.Serialize(editSessionId)}}}""");
}

static async Task RunWorkflowSession(FfiPantographRuntime runtime, string workflowId)
{
    string createResponse = await runtime.WorkflowCreateSession(
        $$"""{"workflow_id":{{JsonSerializer.Serialize(workflowId)}},"keep_alive":true}""");
    string workflowSessionId = ReadString(createResponse, "session_id");

    try
    {
        string runResponse = await runtime.WorkflowRunSession(
            $$"""
            {
              "session_id": {{JsonSerializer.Serialize(workflowSessionId)}},
              "inputs": [{
                "node_id": "text-input-1",
                "port_id": "text",
                "value": "hello from C#"
              }],
              "output_targets": [{
                "node_id": "text-output-1",
                "port_id": "text"
              }]
            }
            """);

        Console.WriteLine($"Workflow run response: {runResponse}");
    }
    finally
    {
        await runtime.WorkflowCloseSession(
            $$"""{"session_id":{{JsonSerializer.Serialize(workflowSessionId)}}}""");
    }
}

static string RequireArgument(string[] args, string name)
{
    int index = Array.IndexOf(args, name);
    if (index < 0 || index + 1 >= args.Length || string.IsNullOrWhiteSpace(args[index + 1]))
    {
        throw new ArgumentException($"Required argument: {name} <path>");
    }

    return Path.GetFullPath(args[index + 1]);
}

static string ReadString(string responseJson, string propertyName)
{
    using JsonDocument document = JsonDocument.Parse(responseJson);
    return document.RootElement.GetProperty(propertyName).GetString()
        ?? throw new InvalidOperationException($"Missing string property '{propertyName}': {responseJson}");
}
