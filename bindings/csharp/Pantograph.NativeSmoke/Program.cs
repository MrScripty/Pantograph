using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Threading.Tasks;
using uniffi.pantograph_headless;

namespace Pantograph.NativeSmoke;

public static class Program
{
    private const string TextWorkflowId = "csharp-runtime-text";
    private const string DiffusionWorkflowId = "csharp-runtime-diffusion";

    public static async Task Main()
    {
        string smokeRoot = RequireEnv("PANTOGRAPH_CSHARP_SMOKE_ROOT");
        string smokeMode = Environment.GetEnvironmentVariable("PANTOGRAPH_CSHARP_SMOKE_MODE") ?? "text";
        string appDataDir = Path.Combine(smokeRoot, "app-data");
        string projectRoot = Path.Combine(smokeRoot, "project");

        InstallFakeDefaultRuntime(appDataDir);

        using FfiPantographRuntime runtime =
            await DirectRuntimeSmoke.CreateRuntime(projectRoot, appDataDir);

        if (smokeMode == "diffusion")
        {
            await RunDiffusionSmoke(runtime, projectRoot);
        }
        else if (smokeMode == "text")
        {
            await RunTextSmoke(runtime, projectRoot);
        }
        else
        {
            throw new InvalidOperationException($"Unsupported PANTOGRAPH_CSHARP_SMOKE_MODE: {smokeMode}");
        }

        await runtime.Shutdown();
    }

    private static async Task RunTextSmoke(FfiPantographRuntime runtime, string projectRoot)
    {
        WriteTextWorkflow(projectRoot, TextWorkflowId);
        ExerciseGraphAuthoringDiscovery(runtime);

        string createResponse = await runtime.WorkflowCreateSession(
            $$"""{"workflow_id":"{{TextWorkflowId}}","keep_alive":true}""");
        string sessionId = ReadString(createResponse, "session_id");

        string sessionRunResponse =
            await runtime.WorkflowRunSession(TextSessionRunRequest(sessionId, "session run"));
        AssertFirstOutputValue(sessionRunResponse, "session run");

        string statusResponse =
            await runtime.WorkflowGetSessionStatus($$"""{"session_id":"{{sessionId}}"}""");
        if (ReadString(statusResponse, "session", "workflow_id") != TextWorkflowId)
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

        Console.WriteLine("Pantograph C# native binding runtime smoke passed.");
    }

    private static void ExerciseGraphAuthoringDiscovery(FfiPantographRuntime runtime)
    {
        string definitions = runtime.WorkflowGraphListNodeDefinitions();
        if (!definitions.Contains("\"node_type\":\"text-input\"", StringComparison.Ordinal))
        {
            throw new InvalidOperationException($"Expected text-input definition: {definitions}");
        }

        string textInput = runtime.WorkflowGraphGetNodeDefinition("text-input");
        if (!textInput.Contains("\"category\":\"input\"", StringComparison.Ordinal)
            || !textInput.Contains("\"id\":\"text\"", StringComparison.Ordinal))
        {
            throw new InvalidOperationException($"Unexpected text-input definition: {textInput}");
        }

        string grouped = runtime.WorkflowGraphGetNodeDefinitionsByCategory();
        if (!grouped.Contains("\"input\"", StringComparison.Ordinal)
            || !grouped.Contains("\"node_type\":\"text-input\"", StringComparison.Ordinal))
        {
            throw new InvalidOperationException($"Unexpected grouped definitions: {grouped}");
        }

        string queryable = runtime.WorkflowGraphGetQueryablePorts();
        if (!queryable.Contains("\"node_type\":\"puma-lib\"", StringComparison.Ordinal)
            || !queryable.Contains("\"port_id\":\"model_path\"", StringComparison.Ordinal))
        {
            throw new InvalidOperationException($"Unexpected queryable ports: {queryable}");
        }
    }

    private static async Task RunDiffusionSmoke(FfiPantographRuntime runtime, string projectRoot)
    {
        string modelPath = RequireEnv("PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH");
        string modelId = Environment.GetEnvironmentVariable("PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID")
            ?? "diffusion/smoke/imported-model";
        string prompt = Environment.GetEnvironmentVariable("PANTOGRAPH_DIFFUSION_SMOKE_PROMPT")
            ?? "paper lantern in the rain";
        string outputPath = Environment.GetEnvironmentVariable("PANTOGRAPH_DIFFUSION_SMOKE_OUTPUT") ?? "";

        WriteDiffusionWorkflow(projectRoot, DiffusionWorkflowId, modelPath, modelId);

        string createResponse = await runtime.WorkflowCreateSession(
            $$"""{"workflow_id":"{{DiffusionWorkflowId}}","keep_alive":true}""");
        string sessionId = ReadString(createResponse, "session_id");

        string response = await runtime.WorkflowRunSession(DiffusionSessionRunRequest(sessionId, prompt));

        string closeResponse = await runtime.WorkflowCloseSession(
            $$"""{"session_id":"{{sessionId}}"}""");
        if (!ReadBool(closeResponse, "ok"))
        {
            throw new InvalidOperationException($"Expected close ok=true: {closeResponse}");
        }

        string imageValue = ReadString(response, "outputs", "0", "value");
        byte[] imageBytes = DecodeImageValue(imageValue);
        if (imageBytes.Length == 0)
        {
            throw new InvalidOperationException($"Diffusion response had an empty image payload: {response}");
        }

        if (!string.IsNullOrWhiteSpace(outputPath))
        {
            string? outputDir = Path.GetDirectoryName(outputPath);
            if (!string.IsNullOrEmpty(outputDir))
            {
                Directory.CreateDirectory(outputDir);
            }

            File.WriteAllBytes(outputPath, imageBytes);
        }

        Console.WriteLine(
            $"Pantograph C# native binding diffusion smoke passed: {imageBytes.Length} image bytes.");
    }

    private static string DiffusionSessionRunRequest(string sessionId, string prompt) =>
        $$"""
        {
          "session_id": "{{sessionId}}",
          "inputs": [{
            "node_id": "text-input-1",
            "port_id": "text",
            "value": {{JsonSerializer.Serialize(prompt)}}
          }],
          "output_targets": [{
            "node_id": "image-output-1",
            "port_id": "image"
          }],
          "run_id": "csharp-diffusion-session-run-1",
          "timeout_ms": 120000
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
            if (element.ValueKind == JsonValueKind.Array && int.TryParse(property, out int index))
            {
                element = element[index];
            }
            else
            {
                element = element.GetProperty(property);
            }
        }

        return element.GetString()
            ?? throw new InvalidOperationException($"Expected string at {string.Join(".", propertyPath)}");
    }

    private static bool ReadBool(string responseJson, string propertyName)
    {
        using JsonDocument document = JsonDocument.Parse(responseJson);
        return document.RootElement.GetProperty(propertyName).GetBoolean();
    }

    private static byte[] DecodeImageValue(string imageValue)
    {
        string base64 = imageValue;
        int dataUrlSeparator = imageValue.IndexOf(";base64,", StringComparison.Ordinal);
        if (dataUrlSeparator >= 0)
        {
            base64 = imageValue[(dataUrlSeparator + ";base64,".Length)..];
        }

        return Convert.FromBase64String(base64);
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

    private static void WriteDiffusionWorkflow(
        string projectRoot,
        string workflowId,
        string modelPath,
        string modelId)
    {
        string workflowPath = PrepareWorkflowPath(projectRoot, workflowId);
        string workflowJson = $$"""
        {
          "version": "1.0",
          "metadata": {
            "name": "C# Runtime Diffusion Smoke",
            "created": "2026-01-01T00:00:00Z",
            "modified": "2026-01-01T00:00:00Z"
          },
          "graph": {
            "nodes": [
              {
                "id": "puma-lib-model",
                "node_type": "puma-lib",
                "data": {
                  "label": "Puma-Lib Model",
                  "selectionMode": "library",
                  "modelPath": {{JsonSerializer.Serialize(modelPath)}},
                  "model_id": {{JsonSerializer.Serialize(modelId)}},
                  "model_type": "diffusion",
                  "task_type_primary": "text-to-image",
                  "recommended_backend": "diffusers",
                  "runtime_engine_hints": ["diffusers", "pytorch"],
                  "selected_binding_ids": [],
                  "dependency_bindings": [],
                  "dependency_requirements": {
                    "model_id": {{JsonSerializer.Serialize(modelId)}},
                    "platform_key": "smoke",
                    "backend_key": "pytorch",
                    "dependency_contract_version": 1,
                    "validation_state": "resolved",
                    "validation_errors": [],
                    "bindings": [],
                    "selected_binding_ids": []
                  },
                  "dependency_requirements_id": {{JsonSerializer.Serialize(modelId)}},
                  "inference_settings": []
                },
                "position": { "x": 240.0, "y": -160.0 }
              },
              {
                "id": "text-input-1",
                "node_type": "text-input",
                "data": {
                  "definition": {
                    "category": "input",
                    "io_binding_origin": "client_session",
                    "label": "Prompt",
                    "description": "Prompt supplied by the caller",
                    "inputs": [{ "id": "text", "label": "Text", "data_type": "string", "required": false, "multiple": false }],
                    "outputs": [{ "id": "text", "label": "Text", "data_type": "string", "required": false, "multiple": false }]
                  },
                  "text": "paper lantern in the rain"
                },
                "position": { "x": 0.0, "y": 0.0 }
              },
              {
                "id": "diffusion-inference-1",
                "node_type": "diffusion-inference",
                "data": {
                  "model_type": "diffusion",
                  "steps": 1,
                  "guidance_scale": 0.0,
                  "width": 64,
                  "height": 64,
                  "seed": 42,
                  "environment_ref": {
                    "state": "ready",
                    "env_ids": []
                  }
                },
                "position": { "x": 240.0, "y": 0.0 }
              },
              {
                "id": "image-output-1",
                "node_type": "image-output",
                "data": {
                  "definition": {
                    "category": "output",
                    "io_binding_origin": "client_session",
                    "label": "Generated Image",
                    "description": "Generated image output",
                    "inputs": [{ "id": "image", "label": "Image", "data_type": "image", "required": false, "multiple": false }],
                    "outputs": [{ "id": "image", "label": "Image", "data_type": "image", "required": false, "multiple": false }]
                  }
                },
                "position": { "x": 520.0, "y": 0.0 }
              }
            ],
            "edges": [
              {
                "id": "e-model-path",
                "source": "puma-lib-model",
                "source_handle": "model_path",
                "target": "diffusion-inference-1",
                "target_handle": "model_path"
              },
              {
                "id": "e-model-settings",
                "source": "puma-lib-model",
                "source_handle": "inference_settings",
                "target": "diffusion-inference-1",
                "target_handle": "inference_settings"
              },
              {
                "id": "e-prompt",
                "source": "text-input-1",
                "source_handle": "text",
                "target": "diffusion-inference-1",
                "target_handle": "prompt"
              },
              {
                "id": "e-image",
                "source": "diffusion-inference-1",
                "source_handle": "image",
                "target": "image-output-1",
                "target_handle": "image"
              }
            ]
          }
        }
        """;
        File.WriteAllText(workflowPath, workflowJson);
    }

    private static string PrepareWorkflowPath(string projectRoot, string workflowId)
    {
        string workflowsDir = Path.Combine(projectRoot, ".pantograph", "workflows");
        Directory.CreateDirectory(workflowsDir);
        return Path.Combine(workflowsDir, $"{workflowId}.json");
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
            workflowRoots?.ToList() ?? new List<string>(),
            maxLoadedSessions: null);

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
