#!/usr/bin/env python3
"""Serve the untrained CPU fixture through PR71's loopback HTTP transport."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import threading
from http.server import ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("scheduler_demo_transport", ROOT / "examples/scheduler-demo/serve.py")
transport = importlib.util.module_from_spec(spec)
spec.loader.exec_module(transport)


class InferenceService(transport.NativeService):
    def __init__(self, binary, output):
        # Reuse the verified request locking, bounded framing, pairing and close
        # behavior. Only the executable arguments and isolated registry differ.
        self.request_lock = threading.Lock()
        env = os.environ.copy()
        env.update(PUMAS_REGISTRY_DB_PATH=str(output / "registry.db"),
                   HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1")
        self.process = subprocess.Popen(
            [str(binary), str(output), "--serve-stdio"], stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, cwd=ROOT, env=env, bufsize=0, start_new_session=True,
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8766)
    parser.add_argument("--output-dir", type=Path, required=True,
                        help="New absolute directory; retains the isolated catalog, artifacts and evidence")
    args = parser.parse_args()
    if not args.output_dir.is_absolute() or args.output_dir.exists():
        parser.error("--output-dir must be a new absolute directory")
    subprocess.run(["cargo", "build", "--locked", "-p", "pantograph-embedded-runtime",
                    "--features", "host-dependency-inventory", "--example", "cpu_embedding_workflow_probe"],
                   cwd=ROOT, check=True)
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT))
    binary = Path(metadata["target_directory"]) / "debug/examples/cpu_embedding_workflow_probe"
    service = InferenceService(binary, args.output_dir)
    origin = f"http://127.0.0.1:{args.port}"
    app = f"http://127.0.0.1:3001/?workflowHost={origin}"
    page = f"""<!doctype html><html lang="en"><meta charset="utf-8"><title>Untrained CPU embedding fixture</title>
<body><h1>Untrained BERT-8 · Candle CPU</h1><p>Real scheduler and model inference. This tiny generated
fixture demonstrates numerical execution, not useful semantic embeddings. Zero model download bytes.</p>
<p><a href="{app}">Open the Pantograph workbench</a></p><ol><li>Open Graph and select cpu-embedding-probe.</li>
<li>Edit the prompt and Save as cpu-embedding-probe.</li><li>Resolve the dependency environment if requested,
then Submit.</li><li>Read the retained vector in I/O Inspector.</li></ol>
<p>This host exposes only the isolated fixture. Model search, downloads, deletion, and arbitrary workflows
are unavailable. Evidence remains in the output directory after shutdown.</p></body></html>""".encode()
    allowed = {origin, "http://127.0.0.1:3001", "http://localhost:3001"}
    try:
        # Read the actual native capability response before advertising readiness.
        response = service.request("workflow_browser_host_capabilities", {})
        if not response["ok"]:
            raise RuntimeError(response["error"])
        with ThreadingHTTPServer(("127.0.0.1", args.port), transport.handler_for(service, page, allowed)) as server:
            print(f"CPU fixture host: {origin}\nWorkbench: {app}\nEvidence: {args.output_dir}", flush=True)
            try:
                server.serve_forever()
            except KeyboardInterrupt:
                pass
    finally:
        service.close()


if __name__ == "__main__":
    main()
