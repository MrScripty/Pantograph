using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using uniffi.pantograph_uniffi;

namespace Pantograph.NativeSmoke;

public static class Program
{
    public static void Main()
    {
        Console.WriteLine("Pantograph C# UniFFI compile smoke passed.");
    }
}

public static class DirectRuntimeCompileSmoke
{
    public static Task<FfiPantographRuntime> CreateRuntime(
        string projectRoot,
        string appDataDir,
        IEnumerable<string>? workflowRoots = null,
        FfiPumasApi? pumasApi = null)
    {
        var config = new FfiEmbeddedRuntimeConfig(
            appDataDir,
            projectRoot,
            workflowRoots?.ToList() ?? new List<string>());

        return FfiPantographRuntime.FfiPantographRuntimeAsync(config, pumasApi);
    }

    public static async Task ExerciseWorkflowSessionSurface(
        FfiPantographRuntime runtime,
        string requestJson)
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
