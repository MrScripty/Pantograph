using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Threading.Tasks;
using uniffi.pantograph_uniffi;

namespace Pantograph.NativeSmoke;

public static class Program
{
    private const string WorkflowId = "csharp-runtime-text";

    public static async Task Main()
    {
        string smokeRoot = RequireEnv("PANTOGRAPH_CSHARP_SMOKE_ROOT");
        string appDataDir = Path.Combine(smokeRoot, "app-data");
        string projectRoot = Path.Combine(smokeRoot, "project");

        WriteTextWorkflow(projectRoot, WorkflowId);
        InstallFakeDefaultRuntime(appDataDir);

        using FfiPantographRuntime runtime =
            await DirectRuntimeSmoke.CreateRuntime(projectRoot, appDataDir);

        string runResponse = await runtime.WorkflowRun(TextRunRequest("direct run", "csharp-run-1"));
        AssertFirstOutputValue(runResponse, "direct run");

        string createResponse = await runtime.WorkflowCreateSession(
            $$"""{"workflow_id":"{{WorkflowId}}","keep_alive":true}""");
        string sessionId = ReadString(createResponse, "session_id");

        string sessionRunResponse =
            await runtime.WorkflowRunSession(TextSessionRunRequest(sessionId, "session run"));
        AssertFirstOutputValue(sessionRunResponse, "session run");

        string statusResponse =
            await runtime.WorkflowGetSessionStatus($$"""{"session_id":"{{sessionId}}"}""");
        if (ReadString(statusResponse, "session", "workflow_id") != WorkflowId)
        {
            throw new InvalidOperationException($"Unexpected status response: {statusResponse}");
        }

        string queueResponse =
            await runtime.WorkflowListSessionQueue($$"""{"session_id":"{{sessionId}}"}""");
        AssertEmptyItems(queueResponse);

        string keepAliveResponse = await runtime.WorkflowSetSessionKeepAlive(
            $$"""{"session_id":"{{sessionId}}","keep_alive":false}""");
        if (ReadBool(keepAliveResponse, "keep_alive"))
        {
            throw new InvalidOperationException($"Expected keep_alive=false: {keepAliveResponse}");
        }

        string closeResponse = await runtime.WorkflowCloseSession(
            $$"""{"session_id":"{{sessionId}}"}""");
        if (!ReadBool(closeResponse, "ok"))
        {
            throw new InvalidOperationException($"Expected close ok=true: {closeResponse}");
        }

        await runtime.Shutdown();
        Console.WriteLine("Pantograph C# UniFFI runtime smoke passed.");
    }

    private static string TextRunRequest(string value, string runId) =>
        $$"""
        {
          "workflow_id": "{{WorkflowId}}",
          "inputs": [{
            "node_id": "text-input-1",
            "port_id": "text",
            "value": {{JsonSerializer.Serialize(value)}}
          }],
          "output_targets": [{
            "node_id": "text-output-1",
            "port_id": "text"
          }],
          "run_id": "{{runId}}"
        }
        """;

    private static string TextSessionRunRequest(string sessionId, string value) =>
        $$"""
        {
          "session_id": "{{sessionId}}",
          "inputs": [{
            "node_id": "text-input-1",
            "port_id": "text",
            "value": {{JsonSerializer.Serialize(value)}}
          }],
          "output_targets": [{
            "node_id": "text-output-1",
            "port_id": "text"
          }],
          "run_id": "csharp-session-run-1"
        }
        """;

    private static string RequireEnv(string variableName)
    {
        string? value = Environment.GetEnvironmentVariable(variableName);
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new InvalidOperationException($"Missing required environment variable: {variableName}");
        }

        return value;
    }

    private static void AssertFirstOutputValue(string responseJson, string expected)
    {
        using JsonDocument document = JsonDocument.Parse(responseJson);
        string actual = document
            .RootElement
            .GetProperty("outputs")[0]
            .GetProperty("value")
            .GetString() ?? "";

        if (actual != expected)
        {
            throw new InvalidOperationException(
                $"Expected first output value '{expected}', got '{actual}'. Response: {responseJson}");
        }
    }

    private static void AssertEmptyItems(string responseJson)
    {
        using JsonDocument document = JsonDocument.Parse(responseJson);
        int itemCount = document.RootElement.GetProperty("items").GetArrayLength();
        if (itemCount != 0)
        {
            throw new InvalidOperationException($"Expected empty queue response: {responseJson}");
        }
    }

    private static string ReadString(string responseJson, params string[] propertyPath)
    {
        using JsonDocument document = JsonDocument.Parse(responseJson);
        JsonElement element = document.RootElement;
        foreach (string property in propertyPath)
        {
            element = element.GetProperty(property);
        }

        return element.GetString()
            ?? throw new InvalidOperationException($"Expected string at {string.Join(".", propertyPath)}");
    }

    private static bool ReadBool(string responseJson, string propertyName)
    {
        using JsonDocument document = JsonDocument.Parse(responseJson);
        return document.RootElement.GetProperty(propertyName).GetBoolean();
    }

    private static void WriteTextWorkflow(string projectRoot, string workflowId)
    {
        string workflowsDir = Path.Combine(projectRoot, ".pantograph", "workflows");
        Directory.CreateDirectory(workflowsDir);
        File.WriteAllText(
            Path.Combine(workflowsDir, $"{workflowId}.json"),
            """
            {
              "version": "1.0",
              "metadata": {
                "name": "C# Runtime Smoke",
                "created": "2026-01-01T00:00:00Z",
                "modified": "2026-01-01T00:00:00Z"
              },
              "graph": {
                "nodes": [
                  {
                    "id": "text-input-1",
                    "node_type": "text-input",
                    "data": {
                      "name": "Prompt",
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
                      },
                      "text": "hello"
                    },
                    "position": { "x": 0.0, "y": 0.0 }
                  },
                  {
                    "id": "text-output-1",
                    "node_type": "text-output",
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
                    },
                    "position": { "x": 200.0, "y": 0.0 }
                  }
                ],
                "edges": [{
                  "id": "e-text",
                  "source": "text-input-1",
                  "source_handle": "text",
                  "target": "text-output-1",
                  "target_handle": "text"
                }]
              }
            }
            """);
    }

    private static void InstallFakeDefaultRuntime(string appDataDir)
    {
        string runtimeDir = Path.Combine(appDataDir, "runtimes", "llama-cpp");
        Directory.CreateDirectory(runtimeDir);

        string[] fileNames = OperatingSystem.IsLinux()
            ? new[]
            {
                "llama-server-x86_64-unknown-linux-gnu",
                "libllama.so",
                "libggml.so"
            }
            : OperatingSystem.IsMacOS()
                ? new[]
                {
                    RuntimeInformation.ProcessArchitecture == Architecture.Arm64
                        ? "llama-server-aarch64-apple-darwin"
                        : "llama-server-x86_64-apple-darwin",
                    "libllama.dylib"
                }
                : OperatingSystem.IsWindows()
                    ? new[]
                    {
                        "llama-server-x86_64-pc-windows-msvc.exe",
                        "llama-runtime.dll"
                    }
                    : Array.Empty<string>();

        foreach (string fileName in fileNames)
        {
            File.WriteAllBytes(Path.Combine(runtimeDir, fileName), Array.Empty<byte>());
        }
    }
}

public static class DirectRuntimeSmoke
{
    public static async Task<FfiPantographRuntime> CreateRuntime(
        string projectRoot,
        string appDataDir,
        IEnumerable<string>? workflowRoots = null,
        FfiPumasApi? pumasApi = null)
    {
        var config = new FfiEmbeddedRuntimeConfig(
            appDataDir,
            projectRoot,
            workflowRoots?.ToList() ?? new List<string>());

        return await FfiPantographRuntime.FfiPantographRuntimeAsync(config, pumasApi);
    }

    public static async Task ExerciseCompileSurface(FfiPantographRuntime runtime, string requestJson)
    {
        await runtime.WorkflowGetCapabilities(requestJson);
        await runtime.WorkflowGetIo(requestJson);
        await runtime.WorkflowPreflight(requestJson);
        await runtime.WorkflowRun(requestJson);
        await runtime.WorkflowCreateSession(requestJson);
        await runtime.WorkflowRunSession(requestJson);
        await runtime.WorkflowGetSessionStatus(requestJson);
        await runtime.WorkflowListSessionQueue(requestJson);
        await runtime.WorkflowCancelSessionQueueItem(requestJson);
        await runtime.WorkflowReprioritizeSessionQueueItem(requestJson);
        await runtime.WorkflowSetSessionKeepAlive(requestJson);
        await runtime.WorkflowCloseSession(requestJson);
        await runtime.Shutdown();
    }
}
