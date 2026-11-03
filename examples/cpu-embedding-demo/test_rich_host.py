"""Actual HTTP multi-branch qualification; browser proof is captured separately."""
import json
import subprocess
import sys
import unittest

from serve import ROOT, transport
import test_host

WORKFLOW = "cpu-multibranch-probe"


class RichHostTests(unittest.TestCase):
    rich = True
    post = test_host.InferenceHostTests.post
    command = test_host.InferenceHostTests.command

    @classmethod
    def setUpClass(cls):
        cls.old_limit = transport.MAX_REQUEST_BYTES
        transport.MAX_REQUEST_BYTES = 128 * 1024
        test_host.InferenceHostTests.setUpClass.__func__(cls)

    @classmethod
    def tearDownClass(cls):
        try:
            test_host.InferenceHostTests.tearDownClass.__func__(cls)
        finally:
            transport.MAX_REQUEST_BYTES = cls.old_limit

    def test_two_real_models_text_report_and_retained_outputs(self):
        graph = self.command("load_workflow", {"path": WORKFLOW})["graph"]
        edit = self.command("create_workflow_execution_session", {"workflowId": WORKFLOW, "graph": graph})
        updated = self.command("update_node_data", {"executionId": edit["session_id"],
                                                    "nodeId": "prompt", "data": {"text": "hello"}})
        graph = updated["graph"]
        self.command("save_workflow", {"name": WORKFLOW, "graph": graph})
        summary = self.command("refresh_current_graph_validation_summary", {"request": {
            "graph_session_id": edit["session_id"], "graph_revision": updated["graph_revision"]}})["summary"]
        self.assertTrue(summary["submit_gate"]["allowed"])
        for node in ["deps", "context-deps"]:
            resolution = self.command("resolve_dependency_environment_action_intent", {"request": {
                "contract_version": 1, "graph_session_id": edit["session_id"],
                "graph_revision": updated["graph_revision"], "validation_session_id": summary["validation_session_id"],
                "target_node_id": node, "action": "resolve"}})
            self.assertEqual(resolution["status"], "request_ready")
        execution = self.command("workflow_create_execution_session", {"request": {"workflow_id": WORKFLOW, "keep_alive": False}})
        try:
            self.command("publish_graph_session_executable_validation_snapshot", {"request": {
                "workflow_id": WORKFLOW, "workflow_semantic_version": "0.1.0", "graph_session_id": edit["session_id"],
                "validation_session_id": summary["validation_session_id"]}})
            result = self.command("workflow_run_execution_session", {"request": {
                "session_id": execution["session_id"], "workflow_semantic_version": "0.1.0",
                "inputs": [{"node_id": n["id"], "port_id": "text", "value": n["data"]["text"]}
                           for n in graph["nodes"] if n["node_type"] == "text-input"]}})
        finally:
            self.command("workflow_close_execution_session", {"request": {"session_id": execution["session_id"]}})
        reference = json.loads(subprocess.check_output([sys.executable, str(ROOT / "examples/cpu-embedding-demo/reference.py")]))
        outputs = {item["node_id"]: item["value"] for item in result["outputs"]}
        for node, width in [("vectors", 8), ("context-vector", 12)]:
            self.assertEqual(len(outputs[node]), width)
            self.assertLess(max(abs(a-b) for a, b in zip(outputs[node], reference["models"][str(width)]["vector"])), 1e-4)
        self.assertIn("different world\nhello", outputs["report"])
        self.assertIn("candle.cpu", outputs["report"])
        self.assertIn("Image unavailable:", outputs["report"])
        self.assertIn("Text generation is unqualified:", outputs["report"])
        request = {"request": {"workflow_run_id": result["workflow_run_id"]}}
        statuses = self.command("workflow_node_status_query", request)["nodes"]
        for node in ["infer", "context-infer"]:
            status = next(item for item in statuses if item["node_id"] == node)
            self.assertEqual(status["status"], "completed")
            self.assertEqual(status["runtime_id"], "candle")
        artifacts = self.command("workflow_io_artifact_query", request)["artifacts"]
        for node, value in outputs.items():
            artifact = next(item for item in artifacts if item["node_id"] == node and item["artifact_role"] == "workflow_output")
            body = self.command("workflow_read_artifact_body", {"request": {"artifact_id": artifact["artifact_id"]}})
            text = bytes(body["body"]).decode()
            self.assertEqual(text if node == "report" else json.loads(text), value)

    def test_fixed_topology_and_model_scope_fail_closed(self):
        original = self.command("load_workflow", {"path": WORKFLOW})["graph"]
        for mutate in [lambda g: g["edges"].pop(),
                       lambda g: g["nodes"][3]["data"].update(runtime="pytorch"),
                       lambda g: g["nodes"][3]["data"].update(pumas_model_ref={"model_id": "external"})]:
            graph = json.loads(json.dumps(original))
            mutate(graph)
            status, error = self.post("save_workflow", {"name": WORKFLOW, "graph": graph})
            self.assertEqual(status, 400)
            self.assertEqual(error["code"], "capability_violation")


if __name__ == "__main__":
    unittest.main()
