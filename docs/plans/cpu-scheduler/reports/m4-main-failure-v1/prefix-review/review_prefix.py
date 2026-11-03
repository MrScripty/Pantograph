#!/usr/bin/env python3
"""Read preserved M4 prefix evidence; never execute a model or modify its inputs.

Uses Python's standard library. SQLite is opened only on a temporary byte-copy.
Frozen source is read from Git objects, so later authorized repairs cannot silently
replace the source which produced the evidence. Output creation is exclusive.
"""
import argparse
import hashlib
import io
import json
import math
import pathlib
import shutil
import sqlite3
import subprocess
import tarfile
import tempfile

HEAD = "7139b2ad547e5b195482146f17b6e27e633c92ed"
TREE = "75852b54dd591d5e01a00f5345256cbff260711f"
PREFIX_DIGEST = "f66cad0672f6ed803e5580cef7fdb0e554b6002d1574a831eb48c2a32ba2fa21"
LATER_SIDECARS = {
    "retained/app-data/serial-cpu-diagnostics.db-shm",
    "retained/app-data/serial-cpu-diagnostics.db-wal",
}


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def read(path):
    return json.loads(path.read_text())


def require(value, message):
    if not value:
        raise AssertionError(message)


def files_under(root):
    return {str(p.relative_to(root)): sha(p) for p in sorted(root.rglob("*")) if p.is_file()}


def digest_map(mapping):
    return hashlib.sha256(json.dumps(mapping, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args])


def frozen_blobs(repo, paths):
    requests = "".join(f"{HEAD}:{name}\n" for name in paths).encode()
    raw = io.BytesIO(subprocess.check_output(["git", "-C", str(repo), "cat-file", "--batch"], input=requests))
    result = {}
    for name in paths:
        header = raw.readline().decode().strip().split()
        require(len(header) == 3 and header[1] == "blob", f"missing frozen blob: {name}")
        result[name] = raw.read(int(header[2]))
        require(raw.read(1) == b"\n", "invalid cat-file framing")
    return result


def claims(resource):
    return {r["kind"]: r["bytes"] for r in resource["claims"]}


def main(args):
    root = args.input.resolve()
    repo = args.repo.resolve()
    output = args.output.resolve()
    require(not output.is_relative_to(root), "output must be outside preserved evidence")
    before = files_under(root / "main")
    # Historical digest includes the main/ prefix in each relative filename.
    historical = {"main/" + name: value for name, value in before.items() if name not in LATER_SIDECARS}
    require(len(historical) == 54 and digest_map(historical) == PREFIX_DIGEST, "original main prefix changed")
    archive = args.archive.resolve()
    archive_digest = sha(archive)
    require(archive_digest == "d6780daaa309f1e00b31f03e695c3bc5bed1ad2633ff68fd224bb276581b85fc", "original archive changed")
    archive_hashes = {}
    with tarfile.open(archive) as bundle:
        for member in bundle.getmembers():
            if member.isfile():
                archive_hashes[member.name] = hashlib.sha256(bundle.extractfile(member).read()).hexdigest()
                require(sha(root / member.name) == archive_hashes[member.name], "original archive/live mismatch: " + member.name)
    require(len(archive_hashes) == 71, "unexpected original archive file count")
    frozen = read(root / "source-freeze.json")
    require(frozen["head"] == HEAD and frozen["tree"] == TREE, "wrong frozen source")
    require(git(repo, "rev-parse", HEAD + "^{tree}").decode().strip() == TREE, "wrong Git tree")
    source = frozen_blobs(repo, list(frozen["source_sha256"]))
    for name, expected in frozen["source_sha256"].items():
        require(hashlib.sha256(source[name]).hexdigest() == expected, "frozen source mismatch: " + name)
    working_changes = [name for name, expected in frozen["source_sha256"].items()
                       if not (repo / name).is_file() or sha(repo / name) != expected]
    manifests = read(root / "pre-run-manifest-sha256.json")
    for name, expected in manifests.items():
        require(sha(root / name) == expected, "pre-run manifest changed: " + name)
    binary_freeze = read(root / "binary-freeze.json")
    binaries = {}
    for name in ("release_example", "cancellation_test_binary"):
        item = binary_freeze[name]
        require(sha(pathlib.Path(item["path"])) == item["sha256"], "frozen binary mismatch")
        binaries[name] = {"sha256": item["sha256"], "bytes": pathlib.Path(item["path"]).stat().st_size}
    fixture = read(root / "fixture-freeze.json")
    fixture_base = "crates/inference/tests/fixtures/candle_bert/"
    fixture_paths = [fixture_base + name for name in fixture["fixture_file_sha256"]]
    fixture_blobs = frozen_blobs(repo, fixture_paths)
    for name, expected in fixture["fixture_file_sha256"].items():
        require(hashlib.sha256(fixture_blobs[fixture_base + name]).hexdigest() == expected, "frozen fixture mismatch")
        width, suffix = name.split("/", 1)
        copied = root / "main/retained/shared-resources/models/embedding/qualification" / ("synthetic-" + width) / suffix
        require(sha(copied) == expected, "copied fixture mismatch")

    status = read(root / "main-execution-status.json")
    result = read(root / "main/m4-result.json")
    owner = read(root / "main/retained/owner-result.json")
    step = read(root / "main/retained/step-0.json")
    events = read(root / "main/retained/events.json")
    records = [row["record"] for row in read(root / "main/retained/native-and-service-log.json")]
    require(status["attempt"] == 1 and status["exit_code"] == 1 and not status["retry_allowed"], "original failed status missing")
    require(result["execution_error"] == owner["execution_error"] == "M4 retained vector artifact missing", "failure changed")
    require(not result["main_and_pressure_protocols_ok"] and result["native_cohort_retries"] == 0, "false main success/retry")
    require(owner["rows"] == [step] and step["index"] == 0, "prefix includes unexpected calls")
    require(not list((root / "main").glob("comparison*")) and not (root / "main/pressure").exists(), "unexpected remaining main owner")
    run_id = step["response"]["workflow_run_id"]
    require(len(events) == 12 and all(e["workflow_run_id"] == run_id for e in events), "wrong raw run join")
    require([e["event_seq"] for e in events] == list(range(1, 13)), "event sequence gap")
    lifecycle = [e for e in events if e["payload"]["payload_type"] == "scheduler_task_attempt_lifecycle_changed" and e["node_id"] == "infer"]
    require([e["payload"]["transition"] for e in lifecycle] == ["started", "completed"], "runtime lifecycle missing")
    terminal = [e for e in events if e["payload"]["payload_type"] == "run_terminal"]
    require(len(terminal) == 1 and terminal[0]["payload"]["status"] == "completed", "actual raw terminal absent")
    phases = [r["phase"] for r in records]
    expected_phases = ["service_opportunity", "preparation_plan", "cold_loaded", "native_drained", "task_registry_ack", "shutdown_stop_ack"]
    require(phases == expected_phases, "unexpected native phase sequence/count")
    _, plan, load, drain, ack, stop = records
    identity_keys = ["workflow_id", "workflow_run_id", "node_id", "task_id", "attempt_id", "execution_request_id", "candidate_id", "reservation_lease_id"]
    require(all(drain[k] == ack[k] for k in identity_keys), "cleanup identity mismatch")
    require(drain["workflow_run_id"] == run_id and drain["task_id"] == "infer", "wrong native task")
    require(all(e["payload"]["scheduler_attempt_id"] == drain["attempt_id"] for e in lifecycle), "Start/completion/native attempt mismatch")
    require(lifecycle[1]["payload"]["reservation_id"] == "runtime-registry." + str(drain["reservation_lease_id"]), "completed lease mismatch")
    old, new = drain["previous_owner"], drain["current_owner"]
    require(old == load["owner"] and new["generation"] == old["generation"] + 2, "native receipt lineage broken")
    require(all(old[k] == new[k] for k in old if k != "generation") and stop["old_owner"] == new, "native owner changed")
    guard = load["guard_lease_id"]
    task = drain["reservation_lease_id"]
    ram = plan["actual_task_ram_bytes"]
    vram = plan["actual_task_vram_bytes"]
    require(ram == vram == plan["package_conservative_floor_bytes"] == 536895107, "actual task claim changed")
    at_load = load["charged_after_publication"]
    at_drain = drain["charged_after_native_drain"]
    at_ack = ack["charged_after_registry_ack"]
    require(at_load == at_ack == stop["charged_before_stop"], "guard/resident changed across task ACK")
    require(at_drain["active_lease_ids"] == [guard, task] and at_ack["active_lease_ids"] == [guard], "wrong task-only removal")
    acquired = {r["reservation_id"]: claims(r) for r in at_drain["active_claims"]}
    require(acquired[task] == {"ram_bytes": ram, "vram_bytes": vram}, "actual acquired task charge wrong")
    require(acquired[guard] == {"ram_bytes": 48 * 1024**2, "vram_bytes": 0}, "guard charge wrong")
    resident = claims(at_drain["resident_claims"])
    require(resident == {"ram_bytes": 32 * 1024**2, "vram_bytes": 0}, "resident charge wrong")
    require(not any(r["resident_uncertain"] for r in [at_load, at_drain, at_ack, stop["charged_after_ack"]]), "unexpected uncertain resources")
    require(stop["charged_after_ack"] == {"active_claims": [], "active_lease_ids": [], "resident_claims": None, "resident_uncertain": False}, "shutdown ACK left charges")
    require(owner["shutdown_ok"] and not owner["registry_after_shutdown"]["reservations"], "shutdown failed")
    runtime = owner["registry_after_shutdown"]["runtimes"][0]
    require(runtime["status"] == "stopped" and runtime["runtime_instance_id"] is None and not runtime["models"], "native owner not retired")
    require(all(value > 0 for value in [load["load_ns"], load["publication_ns"], drain["forward_and_drain_ns"], stop["stop_ns"], stop["acknowledgement_ns"]]), "missing actual interval")

    manifest = read(root / "main/retained/app-data/serial-cpu-artifacts/manifest.json")
    output_events = [e for e in events if e["payload"]["payload_type"] == "io_artifact_observed" and e["payload"].get("producer_node_id") == "vectors"]
    require(len(output_events) == 2, "missing workflow-output/node-output raw artifact facts")
    artifact_ids = {e["payload"]["payload_artifact_id"] for e in output_events}
    require(len(artifact_ids) == 1, "multiple output bodies")
    artifact = next(a for a in manifest["artifacts"] if a["descriptor"]["artifact_id"] in artifact_ids)
    descriptor = artifact["descriptor"]
    body_path = root / "main/retained/app-data/serial-cpu-artifacts/bodies" / artifact["body_file"]
    body = body_path.read_bytes()
    vector = json.loads(body)
    returned = next(o["value"] for o in step["response"]["outputs"] if o["node_id"] == "vectors" and o["port_id"] == "vector")
    require(vector == returned and len(body) == descriptor["byte_length"] == 160, "retained body differs from public response")
    require(descriptor["attribution"]["workflow_run_id"] == run_id, "artifact wrong run")
    require(all(e["payload"]["size_bytes"] == len(body) and e["payload"]["content_hash"] == descriptor["content_hash"] for e in output_events), "artifact event/manifest mismatch")
    golden = json.loads(fixture_blobs[fixture_base + "bert-8/golden.json"])["single_vectors"][0]
    reference = fixture["models"]["bert-8"]
    require(golden == reference["golden_vector"] and len(vector) == len(golden) == 8, "golden join wrong")
    errors = [abs(a - b) for a, b in zip(vector, golden)]
    require(all(math.isfinite(a) and err <= reference["atol"] + reference["rtol"] * abs(b) for a, b, err in zip(vector, golden, errors)), "vector outside independent golden tolerance")
    input_artifact = next(a for a in manifest["artifacts"] if a["descriptor"]["artifact_role"] == "workflow_input")
    input_body = root / "main/retained/app-data/serial-cpu-artifacts/bodies" / input_artifact["body_file"]
    require(sha(input_body) == reference["text_sha256"], "actual input differs from frozen golden input")

    db_path = root / "main/retained/app-data/serial-cpu-diagnostics.db"
    with tempfile.TemporaryDirectory(prefix="m4-prefix-sqlite-") as temp:
        copied = pathlib.Path(temp) / db_path.name
        for suffix in ("", "-wal", "-shm"):
            original = pathlib.Path(str(db_path) + suffix)
            if original.exists():
                shutil.copyfile(original, str(copied) + suffix)
        with sqlite3.connect(copied) as database:
            tables = ["diagnostic_events", "projection_state", "scheduler_timeline_projection", "run_list_projection", "run_detail_projection", "io_artifact_projection", "node_status_projection"]
            counts = {name: database.execute(f'SELECT count(*) FROM "{name}"').fetchone()[0] for name in tables}
            rows = database.execute("SELECT event_seq, workflow_run_id, node_id, occurred_at_ms, payload_json FROM diagnostic_events ORDER BY event_seq").fetchall()
            require([{ "event_seq": seq, "workflow_run_id": run, "node_id": node, "occurred_at_ms": occurred, "payload": json.loads(payload)} for seq, run, node, occurred, payload in rows] == events, "raw SQLite/export mismatch")
    require(counts["diagnostic_events"] == 12 and all(n == 0 for table, n in counts.items() if table != "diagnostic_events"), "projection gap not reproduced")
    facade_source = source["crates/pantograph-embedded-runtime/src/serial_cpu_residency.rs"].decode()
    query_source = source["crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs"].decode()
    producer_source = source["crates/pantograph-embedded-runtime/examples/cpu_embedding_workflow_probe/m4_residency_probe.rs"].decode()
    require("workflow_run_inspection_query(request)" in facade_source and "M4 retained vector artifact missing" in producer_source, "causal source anchors absent")
    require("ledger.query_run_detail_projection" in query_source and "ledger.query_io_artifact_projection" in query_source, "projection query anchors absent")
    require(files_under(root / "main") == before, "analysis modified original prefix")
    report = {
        "review": "read-only original-prefix reanalysis; not a cohort retry or M4 acceptance",
        "analysis_checks_passed": True,
        "original_attempt": {"attempt": 1, "exit_code": 1, "error": result["execution_error"], "started_utc": status["started_utc"], "completed_utc": status["completed_utc"]},
        "qualification": {"M4_C4": "Partial/Blocked", "three_vs_five_objective": "unqualified", "completed_retained_prefix_calls": 1, "remaining_retained_calls": 4, "comparison_calls_executed": 0, "pressure_calls_executed": 0, "separate_cancellation_cases": "not qualified or assessed by this prefix review", "no_continuation_or_retry_authorized": True},
        "freeze": {"head": HEAD, "tree": TREE, "frozen_source_files_verified": len(source), "pre_run_manifests_verified": len(manifests), "fixture_files_verified": len(fixture_paths), "binaries": binaries, "working_source_drift_since_freeze": working_changes, "working_source_drift_does_not_replace_frozen_git_objects": True},
        "preservation": {"original_main_files": len(historical), "current_main_files": len(before), "original_named_sha256_map_digest": digest_map(historical), "inputs_unchanged_after_analysis": True, "original_main_file_sha256": historical, "later_sidecars_observed_before_analysis": {name: value for name, value in before.items() if name in LATER_SIDECARS}, "sidecar_note": "Two diagnostic SQLite sidecars appeared after the prior independent 54-file fingerprint and were already present in the original 71-file archive. All original 54 files still match. This script opens only temporary copies and leaves originals/sidecars/archive unchanged; it does not attribute sidecar creation.", "original_archive_sha256": archive_digest, "original_archive_files_verified_against_live": len(archive_hashes), "original_archive_main_files": sum(name.startswith("main/") for name in archive_hashes), "original_archive_includes_both_later_sidecars": all("main/" + name in archive_hashes for name in LATER_SIDECARS)},
        "actual_prefix": {"workflow_run_id": run_id, "native_identity": {k: drain[k] for k in identity_keys}, "phase_order": phases, "loads": 1, "forward_drains": 1, "retains": 0, "stops": 1, "native_previous_owner": old, "native_current_owner": new, "exact_task_lease": task, "original_guard_lease": guard, "actual_task_claims": acquired[task], "guard_claims": acquired[guard], "resident_claims": resident, "charged_ram_during_forward": ram + 80 * 1024**2, "charged_vram_during_forward": vram, "guard_and_resident_ram_after_task_ack": 80 * 1024**2, "task_registry_ack_is_not_scheduler_ack": ack["scheduler_cleanup_acknowledged"] is False, "shutdown_ack_all_charges_removed": True, "intervals_ns": {"load": load["load_ns"], "publication": load["publication_ns"], "forward_and_drain": drain["forward_and_drain_ns"], "stop": stop["stop_ns"], "stop_registry_ack": stop["acknowledgement_ns"]}, "timing_scope": "actual copied intervals for this call only; no speedup or missing-phase zero"},
        "output": {"artifact_id": descriptor["artifact_id"], "body_sha256": sha(body_path), "body_bytes": len(body), "body_equals_public_response": True, "event_and_manifest_recorded_blake3_agree": True, "blake3_body_recomputed": False, "independent_committed_golden_sha256": fixture["fixture_file_sha256"]["bert-8/golden.json"], "max_absolute_error": max(errors), "atol": reference["atol"], "rtol": reference["rtol"], "within_golden_tolerance": True, "untrained_fixture_not_semantic_model_quality": True},
        "projection_gap": {"copied_sqlite_table_counts": counts, "raw_sqlite_matches_export": True, "actual_raw_run_terminal": "completed", "raw_output_facts": len(output_events), "retained_output_body_exists": True, "cause": "The frozen facade forwarded inspection directly to read-only projection queries. Raw events/artifact storage existed, but projection tables and projection_state were empty; projection_batch_size did not refresh them. The producer required a vectors artifact from the empty projected inspection and raised its preserved error.", "scope_limit": "This is source-assisted causal analysis of preserved data, not a replay of the failed inspection or runtime execution."},
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("x") as stream:
        json.dump(report, stream, indent=2, sort_keys=True)
        stream.write("\n")
    print(json.dumps({"analysis_checks_passed": True, "report": str(output), "original_exit_code": 1, "M4_C4": "Partial/Blocked", "max_absolute_error": max(errors)}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=pathlib.Path, default=pathlib.Path("/workspace/m4-qualification-v1"))
    parser.add_argument("--repo", type=pathlib.Path, default=pathlib.Path("/workspace/pantograph-m4"))
    parser.add_argument("--archive", type=pathlib.Path, default=pathlib.Path("/workspace/pantograph-m4/docs/plans/cpu-scheduler/reports/m4-main-failure-v1/original-main-evidence.tar.gz"))
    parser.add_argument("--output", type=pathlib.Path, default=pathlib.Path("/workspace/m4-prefix-review-v1/final-report.json"))
    main(parser.parse_args())
