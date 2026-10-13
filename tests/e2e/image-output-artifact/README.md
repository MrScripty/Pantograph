# Image Output retained artifact browser check

The existing Quality Gates `frontend-tests` job runs this fixture in hosted
Google Chrome with ordinary sandboxing. It checks out and verifies the exact
requested source SHA, rejects the sandbox override, and uploads
`image-output-browser-qualification` with the complete test log. Successful JSON
includes the actual source revision, browser version, sandbox mode, scope and
completed cleanup. The log, rather than workflow metadata alone, binds evidence
to the source that was executed.

Run from the repository root:

```sh
node tests/e2e/image-output-artifact/run.mjs
```

The runner builds the actual `ImageOutputNode.svelte` and its artifact decoder,
then launches isolated headless Chromium with its sandbox enabled. Set
`CHROMIUM_BINARY` to select a compatible Chromium executable. A missing sandbox
stops acceptance at browser startup; the runner does not automatically weaken it.
When separately approved for this isolated test run, an explicit override is:

```sh
PANTOGRAPH_IMAGE_BROWSER_NO_SANDBOX=1 node tests/e2e/image-output-artifact/run.mjs
```

This flag disables Chromium's sandbox for that invocation and requires approval
before execution. It is never an automatic fallback or a production setting.
The runner prints the selected sandbox mode before launch and includes the actual
Chromium version and sandbox mode in successful evidence. A run with this override
proves the listed presentation behaviors in an isolated browser; it does not prove
sandboxed startup or sandbox security.

Each invocation owns a temporary build, browser profile, download directory,
loopback HTTP port, CDP connection, and browser process, and releases them on exit.
The HTTP server serves only a preloaded allowlist of build outputs; request URLs
never select filesystem paths. Cleanup attempts component unmount before closing
CDP, terminates the browser with a bounded graceful wait and forced exit if needed,
closes the server, and removes the temporary root. Browser termination also ends
fixture-local transport promises, URL instrumentation, and event listeners.

The fixture substitutes only the graph shell, workflow store, and artifact
transport. Browser canvas encodes real PNG and JPEG fixtures. Chromium decodes
their retained bytes and downloads files whose bytes, extension, and Blob MIME
are checked against those independent fixtures. Native CDP mouse and keyboard
input checks Download, Expand, Close, Escape, focus restoration, and isolation
of control pointer events from the surrounding shell. This does not qualify
the full workflow editor's dragging, panning, or pointer capture behavior.

Negative cases isolate malformed scheduler references, incomplete reads,
nonimage/unreadable artifacts, descriptor/read ID mismatch, MIME mismatch,
read failure, and invalid raster bytes. Exact visible diagnostics identify the
intended failure boundary. Deferred reads and descriptors exercise superseded
success/failure and completion after unmount; URL observations check replacement,
clearing, decode failure, and unmount cleanup.

Evidence is representative CPU browser presentation with controlled artifact
transport. It does not establish native desktop, scheduler execution, model
inference, image generation, hardware acceleration, or GPU capability. JSON
printed after successful completion contains the exercised results and scope.
