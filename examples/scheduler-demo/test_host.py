"""Integration checks against the real persistent Rust service and HTTP boundary.

Build scheduler_demo first, then run with the normal Cargo environment:
    python3 -m unittest discover -s examples/scheduler-demo -p 'test_*.py'
"""
import http.client
from http.server import ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import socket
import time
from concurrent.futures import ThreadPoolExecutor
import unittest

from serve import ROOT, NativeService, handler_for


class LiveHostTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT,
        ))
        binary = Path(metadata["target_directory"]) / "debug/examples/scheduler_demo"
        cls.binary = binary
        cls.directory = tempfile.TemporaryDirectory(prefix="pantograph-host-test-")
        cls.native = NativeService(binary, cls.directory.name)
        cls.origin = "http://127.0.0.1:3001"
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), handler_for(cls.native, b"demo", {cls.origin}))
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.thread.join()
        cls.server.server_close()
        cls.native.close()
        cls.directory.cleanup()

    def post(self, route, value, origin=None, raw=False):
        client = http.client.HTTPConnection("127.0.0.1", self.server.server_port, timeout=35)
        client.request("POST", route, body=value if raw else json.dumps(value), headers={
            "Content-Type": "application/json", "Origin": origin or self.origin,
        })
        response = client.getresponse()
        result = (response.status, json.loads(response.read()))
        client.close()
        return result

    def command(self, command, args):
        return self.post("/api/workflow/commands/" + command, args)

    def test_idle_preconnect_does_not_block_active_command(self):
        with socket.create_connection(("127.0.0.1", self.server.server_port), timeout=2):
            started = time.monotonic()
            status, capabilities = self.command("workflow_browser_host_capabilities", {})
            self.assertEqual(status, 200)
            self.assertTrue(capabilities["model_library_read"])
            self.assertLess(time.monotonic() - started, 2)

    def test_concurrent_native_responses_remain_paired(self):
        messages = [f"Concurrent actual run {index}" for index in range(8)]
        with ThreadPoolExecutor(max_workers=8) as executor:
            results = list(executor.map(lambda message: self.post("/run", {"items": [{"prompt": message}]}), messages))
        for message, (status, report) in zip(messages, results):
            self.assertEqual(status, 200, report)
            self.assertTrue(report["execution"]["ok"])
            self.assertEqual(report["execution"]["result"]["outputs"][0]["value"], message)

    def test_real_runs_remain_queryable_and_bodies_are_retained(self):
        message = "Actual local execution — whitespace\n preserved."
        status, report = self.post("/run", {"items": [{"prompt": message}]})
        self.assertEqual(status, 200)
        self.assertTrue(report["execution"]["ok"], report)
        run_id = next(event["workflow_run_id"] for event in report["diagnostics"] if event.get("workflow_run_id"))
        status, artifacts = self.command("workflow_io_artifact_query", {"request": {"workflow_run_id": run_id}})
        self.assertEqual(status, 200)
        output = next(item for item in artifacts["artifacts"] if item["artifact_role"] == "workflow_output")
        self.assertNotEqual(output["retention_state"], "metadata_only")
        status, body = self.command("workflow_read_artifact_body", {"request": {"artifact_id": output["artifact_id"]}})
        self.assertEqual(status, 200)
        self.assertEqual(bytes(body["body"]).decode(), message)
        status, failed = self.post("/run", {"items": [{"missing": "no text"}]})
        self.assertEqual(status, 200)
        self.assertFalse(failed["execution"]["ok"])
        self.assertTrue(failed["diagnostics"])
        status, old = self.command("workflow_run_detail_query", {"request": {"workflow_run_id": run_id}})
        self.assertEqual(status, 200)
        self.assertIn(run_id, json.dumps(old))
        self.assertIn(message, bytes(self.command("workflow_read_artifact_body", {"request": {"artifact_id": output["artifact_id"]}})[1]["body"]).decode())

    def test_real_library_read_and_explicit_unsupported_command(self):
        status, capabilities = self.command("workflow_browser_host_capabilities", {})
        self.assertEqual(status, 200)
        self.assertTrue(capabilities["model_library_read"])
        self.assertFalse(capabilities["model_library_write"])
        status, models = self.command("query_port_options", {"nodeType": "puma-lib", "portId": "pumas_model_ref"})
        self.assertEqual(status, 200, models)
        self.assertEqual(models["options"], [])
        status, error = self.command("delete_model", {"modelId": "unavailable"})
        self.assertEqual(status, 400)
        self.assertEqual(error["code"], "capability_violation")

    def test_real_artifact_and_query_errors_are_not_suppressed(self):
        status, error = self.command("workflow_read_artifact_body", {"request": {"artifact_id": "does-not-exist"}})
        self.assertEqual(status, 400)
        self.assertTrue(error["message"])
        status, error = self.command("workflow_run_detail_query", {"request": {"workflow_run_id": 42}})
        self.assertEqual(status, 400)
        self.assertTrue(error["message"])

    def test_one_shot_does_not_require_or_open_pumas_registry(self):
        env = os.environ.copy()
        env.pop("PUMAS_REGISTRY_DB_PATH", None)
        result = subprocess.run([str(self.binary)], input=json.dumps({"items": [{"prompt": "One-shot still works"}]}),
                                capture_output=True, text=True, env=env, check=True, timeout=30)
        self.assertTrue(json.loads(result.stdout)["execution"]["ok"])

    def test_saved_graph_reads_are_real_and_path_scoped(self):
        status, workflows = self.command("list_workflows", {})
        self.assertEqual(status, 200)
        self.assertEqual([item["id"] for item in workflows], ["scheduler-demo"])
        status, graph = self.command("load_workflow", {"path": ".pantograph/workflows/scheduler-demo.json"})
        self.assertEqual(status, 200)
        self.assertEqual(len(graph["graph"]["nodes"]), 3)
        status, inspection = self.command("workflow_graph_inspect", {"request": {"path": ".pantograph/workflows/scheduler-demo.json"}})
        self.assertEqual(status, 200, inspection)
        status, error = self.command("load_workflow", {"path": "../../private.json"})
        self.assertEqual(status, 400)
        self.assertEqual(error["code"], "capability_violation")

    def test_origin_and_input_validation(self):
        status, error = self.post("/run", {}, origin="https://untrusted.example")
        self.assertEqual(status, 403)
        self.assertEqual(error["code"], "capability_violation")
        self.assertEqual(self.post("/run", "{broken", raw=True)[0], 400)
        self.assertEqual(self.post("/run", "x" * 8193, raw=True)[0], 400)
        self.assertEqual(self.command("workflow_run_list_query", [])[0], 400)

class ProtocolDeadlineTests(unittest.TestCase):
    def test_partial_stdout_cannot_bypass_response_deadline(self):
        import sys
        service = NativeService.__new__(NativeService)
        service.process = subprocess.Popen(
            [sys.executable, "-c", "import sys,time; sys.stdout.write('{'); sys.stdout.flush(); time.sleep(5)"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, bufsize=0,
        )
        try:
            with self.assertRaises(TimeoutError):
                service.exchange(b"{}\n", timeout=0.1)
        finally:
            service.close()


if __name__ == "__main__":
    unittest.main()
