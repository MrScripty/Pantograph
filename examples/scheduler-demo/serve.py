#!/usr/bin/env python3
"""Loopback browser host backed by one persistent, real Rust scheduler service.

The service owns an ephemeral ledger, retained small artifact bodies and an
isolated empty Pumas library. No recorded responses or model weights are loaded.
"""
import argparse
import json
import os
import re
import select
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import subprocess
import tempfile
import time
import threading
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[2]
MAX_REQUEST_BYTES = 8192
COMMAND_PREFIX = "/api/workflow/commands/"


class NativeService:
    def __init__(self, binary, directory):
        self.request_lock = threading.Lock()
        env = os.environ.copy()
        env["PUMAS_REGISTRY_DB_PATH"] = str(Path(directory) / "pumas-registry.db")
        self.process = subprocess.Popen(
            [str(binary), "--serve-stdio"], stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, cwd=ROOT, env=env, bufsize=0,
        )

    def request(self, command, args):
        payload = json.dumps({"command": command, "args": args}).encode() + b"\n"
        if not self.request_lock.acquire(timeout=30):
            raise TimeoutError("Native workflow service is busy; retry after the current request")
        try:
            return self.exchange(payload)
        finally:
            self.request_lock.release()

    def exchange(self, payload, timeout=30):
        deadline = time.monotonic() + timeout
        os.set_blocking(self.process.stdin.fileno(), False)
        os.set_blocking(self.process.stdout.fileno(), False)
        response = bytearray()
        try:
            while payload or b"\n" not in response:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError("Native workflow service exceeded its response deadline")
                readable, writable, _ = select.select(
                    [self.process.stdout], [self.process.stdin] if payload else [], [], remaining,
                )
                if writable:
                    sent = os.write(self.process.stdin.fileno(), payload[:4096])
                    payload = payload[sent:]
                if readable:
                    chunk = os.read(self.process.stdout.fileno(), 65536)
                    if not chunk:
                        raise RuntimeError("Native workflow service exited; restart the demo host")
                    response.extend(chunk)
                    if len(response) > 8 * 1024 * 1024:
                        raise RuntimeError("Native workflow response exceeds the demo limit")
            try:
                result = json.loads(response)
            except ValueError as error:
                raise RuntimeError("Native workflow service returned invalid JSON") from error
            if not isinstance(result, dict) or not isinstance(result.get("ok"), bool):
                raise RuntimeError("Native workflow service returned an invalid response envelope")
            return result
        except (OSError, RuntimeError, TimeoutError):
            # A timed-out protocol cannot pair subsequent requests and replies.
            self.process.kill()
            raise

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        self.process.stdout.close()


def handler_for(service, page, allowed_origins):
    # Keep response values independent of request bytes, including folded or
    # duplicate headers. Configuration must itself be a single loopback origin.
    trusted_origins = tuple(allowed_origins)
    for origin in trusted_origins:
        if not isinstance(origin, str) or not re.fullmatch(
            r"http://(?:127\.0\.0\.1|localhost|\[::1\])(?::[1-9][0-9]{0,4})?", origin,
        ) or (urlsplit(origin).port is not None and not 1 <= urlsplit(origin).port <= 65535):
            raise ValueError("Allowed origins must be exact HTTP loopback origins")

    class Handler(BaseHTTPRequestHandler):
        def setup(self):
            super().setup()
            # Chromium may open idle speculative connections before sending headers.
            # Bound those sockets independently of active command connections.
            self.connection.settimeout(10)

        def allowed_origin(self):
            origins = self.headers.get_all("Origin", [])
            if len(origins) != 1 or any(ord(char) < 33 or ord(char) > 126 for char in origins[0]):
                return None
            return next((origin for origin in trusted_origins if origins[0] == origin), None)

        def respond(self, status, body, content_type="application/json"):
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("Vary", "Origin")
            origin = self.allowed_origin()
            if origin is not None:
                self.send_header("Access-Control-Allow-Origin", origin)
            self.end_headers()
            self.wfile.write(body)

        def error(self, status, code, message):
            self.respond(status, json.dumps({
                "code": code, "message": message, "details": None, "diagnostics": None,
            }).encode())

        def do_GET(self):
            if self.path == "/":
                self.respond(200, page, "text/html; charset=utf-8")
            else:
                self.error(404, "not_found", "No such demo route")

        def do_OPTIONS(self):
            origin = self.allowed_origin()
            if origin is None:
                self.error(403, "capability_violation", "Use an allowed loopback browser origin")
                return
            if self.path != "/run" and not self.path.startswith(COMMAND_PREFIX):
                self.error(404, "not_found", "No such demo route")
                return
            self.send_response(204)
            self.send_header("Access-Control-Allow-Origin", origin)
            self.send_header("Access-Control-Allow-Methods", "POST, OPTIONS")
            self.send_header("Access-Control-Allow-Headers", "Content-Type")
            self.send_header("Vary", "Origin")
            self.send_header("Content-Length", "0")
            self.end_headers()

        def do_POST(self):
            if self.allowed_origin() is None:
                self.error(403, "capability_violation", "Use an allowed loopback browser origin")
                return
            if self.path == "/run":
                command = "run"
            elif self.path.startswith(COMMAND_PREFIX):
                command = self.path[len(COMMAND_PREFIX):]
                if not command or not all(c.isascii() and (c.isalnum() or c == "_") for c in command):
                    self.error(400, "invalid_request", "Invalid workflow command")
                    return
            else:
                self.error(404, "not_found", "No such demo route")
                return
            try:
                length = int(self.headers.get("Content-Length", "0"))
                if not 0 < length <= MAX_REQUEST_BYTES:
                    raise ValueError("Input must contain 1 to 8192 bytes")
                self.connection.settimeout(10)
                args = json.loads(self.rfile.read(length))
                if command != "run" and not isinstance(args, dict):
                    raise ValueError("Command arguments must be a JSON object")
                response = service.request(command, args)
                if response["ok"]:
                    self.respond(200, json.dumps(response["value"]).encode())
                else:
                    self.respond(400, json.dumps(response["error"]).encode())
            except (ValueError, UnicodeError) as error:
                self.error(400, "invalid_request", str(error))
            except (OSError, RuntimeError, TimeoutError) as error:
                self.error(503, "runtime_not_ready", str(error))

    return Handler


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
    # Explicit origins only: no wildcard or arbitrary host/path forwarding.
    allowed_origins = {origin, "http://127.0.0.1:3001", "http://localhost:3001"}
    with tempfile.TemporaryDirectory(prefix="pantograph-loopback-") as directory:
        service = NativeService(binary, directory)
        try:
            with ThreadingHTTPServer(("127.0.0.1", args.port), handler_for(service, page, allowed_origins)) as server:
                print(f"Pantograph scheduler demo: {origin} (Ctrl+C to stop)", flush=True)
                print(f"Application: http://127.0.0.1:3001/?workflowHost={origin}", flush=True)
                try:
                    server.serve_forever()
                except KeyboardInterrupt:
                    pass
        finally:
            service.close()


if __name__ == "__main__":
    main()
