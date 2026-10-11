#!/usr/bin/env python3
"""Loopback-only browser demo for the real scheduler_demo Rust executable."""
import argparse
import json
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8765)
    args = parser.parse_args()
    subprocess.run(
        ["cargo", "build", "--locked", "-p", "pantograph-workflow-service", "--example", "scheduler_demo"],
        cwd=ROOT, check=True,
    )
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT,
    ))
    binary = Path(metadata["target_directory"]) / "debug" / "examples" / "scheduler_demo"
    page = Path(__file__).with_name("index.html").read_bytes()
    origin = f"http://127.0.0.1:{args.port}"

    class Handler(BaseHTTPRequestHandler):
        def respond(self, status, body, content_type):
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            if self.path == "/":
                self.respond(200, page, "text/html; charset=utf-8")
            else:
                self.respond(404, b"Not found", "text/plain")

        def do_POST(self):
            if self.path != "/run" or self.headers.get("Origin") != origin:
                self.respond(403, b"Use the local demo page", "text/plain")
                return
            try:
                length = int(self.headers.get("Content-Length", "0"))
                if not 0 < length <= 8192:
                    raise ValueError("Input must contain 1 to 8192 bytes")
                payload = self.rfile.read(length)
                json.loads(payload)
                result = subprocess.run(
                    [str(binary)], input=payload, capture_output=True, timeout=30, cwd=ROOT,
                )
                if result.returncode:
                    raise ValueError(result.stderr.decode(errors="replace"))
                json.loads(result.stdout)
                self.respond(200, result.stdout, "application/json")
            except (ValueError, subprocess.TimeoutExpired) as error:
                self.respond(400, str(error).encode(), "text/plain; charset=utf-8")

    print(f"Pantograph scheduler demo: {origin} (Ctrl+C to stop)", flush=True)
    with HTTPServer(("127.0.0.1", args.port), Handler) as server:
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
