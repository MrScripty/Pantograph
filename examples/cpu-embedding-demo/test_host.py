"""Real HTTP editor-command → scheduler → Candle → retained-vector qualification.

Build cpu_embedding_workflow_probe first. This does not qualify browser rendering.
"""
import http.client
from http.server import ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest

from serve import ROOT, InferenceService, transport

WORKFLOW = "cpu-embedding-probe"


class InferenceHostTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT))
        binary = Path(metadata["target_directory"]) / "debug/examples/cpu_embedding_workflow_probe"
        cls.directory = tempfile.TemporaryDirectory(prefix="pantograph-inference-http-test-")
        cls.output = Path(cls.directory.name) / "run"
        cls.evidence = {}
        cls.native = InferenceService(binary, cls.output)
        cls.origin = "http://127.0.0.1:3001"
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), transport.handler_for(cls.native, b"fixture", {cls.origin}))
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.thread.join()
        cls.server.server_close()
        cls.native.close()
        shutdown = json.loads((cls.output / "browser-shutdown.json").read_text())
        assert shutdown["ok"]
        report_path = os.environ.get("PANTOGRAPH_HTTP_EVIDENCE_PATH")
        if report_path and cls.evidence:
            cls.evidence["shutdown"] = shutdown
            cls.evidence["session_close"] = json.loads((cls.output / "browser-session-close.json").read_text())
            cls.evidence["host_result"] = json.loads((cls.output / "browser-last-run.json").read_text())
            raw = json.dumps(cls.evidence, indent=2).replace(str(cls.output), "<probe_root>")
            with Path(report_path).open("x") as report:
                report.write(raw + "\n")
        cls.directory.cleanup()

    def post(self, command, args, origin=None):
        client = http.client.HTTPConnection("127.0.0.1", self.server.server_port, timeout=35)
        client.request("POST", transport.COMMAND_PREFIX + command, json.dumps(args),
                       {"Content-Type": "application/json", "Origin": origin or self.origin})
        response = client.getresponse()
        result = response.status, json.loads(response.read())
        client.close()
        return result

    def command(self, command, args=None):
        status, result = self.post(command, args or {})
        self.assertEqual(status, 200, result)
        return result

    def test_real_editor_command_lifecycle_and_retained_model_output(self):
        capabilities = self.command("workflow_browser_host_capabilities")
        self.assertTrue(capabilities["graph_edit"])
        self.assertFalse(capabilities["model_library_write"])
        models = self.command("query_port_options", {"nodeType": "puma-lib", "portId": "pumas_model_ref"})
        self.assertEqual(len(models["options"]), 1)
        file = self.command("load_workflow", {"path": f".pantograph/workflows/{WORKFLOW}.json"})
        edit = self.command("create_workflow_execution_session", {"graph": file["graph"], "workflowId": WORKFLOW})
        # Change the seeded prompt through the same command the real editor uses.
        updated = self.command("update_node_data", {"executionId": edit["session_id"],
                                "nodeId": "prompt", "data": {"text": "hello"}})
        self.command("save_workflow", {"name": WORKFLOW, "graph": updated["graph"]})
        validation = self.command("refresh_current_graph_validation_summary", {"request": {
            "graph_session_id": edit["session_id"], "graph_revision": updated["graph_revision"]}})
        self.assertTrue(validation["summary"]["submit_gate"]["allowed"], validation)
        validation_id = validation["summary"]["validation_session_id"]
        resolved = self.command("resolve_dependency_environment_action_intent", {"request": {
            "contract_version": 1, "graph_session_id": edit["session_id"],
            "graph_revision": updated["graph_revision"], "validation_session_id": validation_id,
            "target_node_id": "deps", "action": "resolve"}})
        self.assertEqual(resolved["status"], "request_ready", resolved)
        execution = self.command("workflow_create_execution_session", {"request": {
            "workflow_id": WORKFLOW, "usage_profile": None, "keep_alive": False}})
        try:
            self.command("publish_graph_session_executable_validation_snapshot", {"request": {
                "workflow_id": WORKFLOW, "workflow_semantic_version": "0.1.0",
                "graph_session_id": edit["session_id"], "validation_session_id": validation_id,
                "validation_snapshot_id": None}})
            result = self.command("workflow_run_execution_session", {"request": {
                "session_id": execution["session_id"], "workflow_semantic_version": "0.1.0",
                "inputs": [{"node_id": "prompt", "port_id": "text", "value": "hello"}],
                "output_targets": None, "override_selection": None, "timeout_ms": None, "priority": None}})
        finally:
            self.command("workflow_close_execution_session", {"request": {"session_id": execution["session_id"]}})
            self.command("remove_execution", {"executionId": edit["session_id"]})
        expected = json.loads((ROOT / "crates/inference/tests/fixtures/candle_bert/bert-8/golden.json").read_text())["single_vectors"][1]
        actual = next(output["value"] for output in result["outputs"] if output["node_id"] == "vectors")
        self.assertEqual(len(actual), len(expected))
        self.assertLess(max(abs(a-b) for a, b in zip(actual, expected)), 1e-4)
        run_request = {"request": {"workflow_run_id": result["workflow_run_id"]}}
        nodes = self.command("workflow_node_status_query", run_request)
        self.assertEqual(next(node["status"] for node in nodes["nodes"] if node["node_id"] == "vectors"), "completed")
        inference = next(node for node in nodes["nodes"] if node["node_id"] == "infer")
        self.assertEqual(inference["status"], "completed")
        self.assertEqual(inference["runtime_id"], "candle")
        self.assertIsNotNone(inference["duration_ms"])
        timeline = self.command("workflow_scheduler_timeline_query", run_request)
        completed = next(event for event in timeline["events"] if event.get("scheduler_task_id") == "infer"
                         and event.get("scheduler_attempt_transition") == "completed")
        self.assertEqual(completed["scheduler_attempt_runtime_id"], "candle")
        self.assertEqual(completed["scheduler_attempt_device_id"], "cpu")
        artifacts = self.command("workflow_io_artifact_query", run_request)
        output = next(item for item in artifacts["artifacts"] if item["artifact_role"] == "workflow_output" and item["node_id"] == "vectors")
        body = self.command("workflow_read_artifact_body", {"request": {"artifact_id": output["artifact_id"]}})
        self.assertEqual(json.loads(bytes(body["body"]).decode()), actual)
        self.evidence.update({
            "scope": "real_http_editor_command_lifecycle; browser rendering unqualified",
            "editor_qualified": False, "model_training": "untrained", "model_acquisition_bytes": 0,
            "input": "hello", "max_reference_error": max(abs(a-b) for a, b in zip(actual, expected)),
            "response": result, "scheduler_timeline": timeline, "node_status": nodes,
            "artifacts": artifacts, "retained_output_body": body,
        })

    def test_fail_closed_scope_and_origin(self):
        attempts = [
            ("delete_model", {"modelId": "synthetic-bert-8"}),
            ("load_workflow", {"path": "../../Cargo.toml"}),
            ("workflow_create_execution_session", {"request": {"workflow_id": "other", "keep_alive": False}}),
            ("update_node_data", {"executionId": "missing", "nodeId": "infer", "data": {"runtime": "remote"}}),
            ("resolve_dependency_environment_action_intent", {"request": {"action": "install"}}),
        ]
        for command, args in attempts:
            status, error = self.post(command, args)
            self.assertEqual(status, 400, error)
            self.assertEqual(error["code"], "capability_violation")
        status, error = self.post("workflow_browser_host_capabilities", {}, "https://example.invalid")
        self.assertEqual(status, 403)
        self.assertEqual(error["code"], "capability_violation")
        status, error = self.post("publish_graph_session_executable_validation_snapshot", {"request": {
            "workflow_id": WORKFLOW, "workflow_semantic_version": "0.2.0", "graph_session_id": "missing",
            "validation_session_id": None, "validation_snapshot_id": None}})
        self.assertEqual(status, 400, error)


if __name__ == "__main__":
    unittest.main()
