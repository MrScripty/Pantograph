#!/usr/bin/env python3
"""Prospectively written M4-v2 saved-data checks. No model/Cargo/native calls.

Only Git reads, hashes, JSON/artifact reads and temporary SQLite copies are used.
Original evidence is never refreshed or opened by SQLite. Reports are created
exclusively outside the evidence root. Missing phases are errors, never zero.
"""
import argparse
import copy
import hashlib
import io
import json
import math
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile

HEAD = "c25fd49d19dd9268b5d63a794835c568eb32ff9b"
TREE = "4ac114a25c45e90e5c64623f5053ed3ab6193024"
PROTOCOL = "m4-serial-residency-v2-readback"
WORK = [8, 8, 12, 12, 8]
MIB = 1024 ** 2
TASK = {8: 536895107, 12: 536908694}
GUARD = 48 * MIB
RESIDENT = 32 * MIB
IDENTITY = ("workflow_id", "workflow_run_id", "node_id", "task_id", "attempt_id",
            "execution_request_id", "candidate_id", "reservation_lease_id")
OWNER_FIELDS = {"loaded_instance": 16, "loaded_profile": 32, "effective_settings": 32}
KIND_FILTERS = {
    "run_detail": {"scheduler_estimate_produced", "scheduler_queue_placement", "scheduler_queue_control",
                   "scheduler_run_delayed", "scheduler_model_lifecycle_changed", "scheduler_run_admitted",
                   "run_started", "run_terminal", "run_snapshot_accepted", "node_execution_status",
                   "inference_execution_diagnostic_observed", "io_artifact_observed", "diagnostic_error_occurred"},
    "node_status": {"node_execution_status", "scheduler_task_attempt_lifecycle_changed",
                    "diagnostic_error_occurred", "inference_execution_diagnostic_observed"},
    "io_artifact": {"io_artifact_observed", "retention_artifact_state_changed"},
}


def check(value, message):
    if not value:
        raise AssertionError(message)


def read(path):
    return json.loads(path.read_text())


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def hashes(root):
    return {str(p.relative_to(root)): sha(p) for p in sorted(root.rglob("*")) if p.is_file()}


def one(rows, predicate, label):
    matches = [r for r in rows if predicate(r)]
    check(len(matches) == 1, f"expected exactly one {label}, found {len(matches)}")
    return matches[0]


def integer(value, label):
    check(type(value) is int and value >= 0, f"missing/invalid integer: {label}")
    return value


def owner(value):
    check(isinstance(value, dict), "native owner missing")
    for field, size in OWNER_FIELDS.items():
        data = value.get(field)
        check(isinstance(data, list) and len(data) == size
              and all(type(b) is int and 0 <= b <= 255 for b in data), f"invalid owner {field}")
    check(value.get("cpu_threads") == 1 and integer(value.get("generation"), "generation") > 0,
          "wrong native CPU thread/generation")
    return value


def warm(previous, current):
    owner(previous)
    owner(current)
    check(all(previous[k] == current[k] for k in (*OWNER_FIELDS, "cpu_threads")), "warm owner identity changed")
    check(current["generation"] == previous["generation"] + 2, "warm generation is not actual +2")


def resources(resource):
    rows = resource.get("claims") if isinstance(resource, dict) else None
    check(isinstance(rows, list) and len(rows) == 2, "complete RAM/VRAM claims missing")
    check({r.get("kind") for r in rows} == {"ram_bytes", "vram_bytes"}, "resource kinds missing/duplicated")
    return {r["kind"]: integer(r.get("bytes"), "claim bytes") for r in rows}


def charge_map(snapshot):
    check(snapshot.get("resident_uncertain") is False, "uncertain resident accounting")
    ids = snapshot.get("active_lease_ids")
    rows = snapshot.get("active_claims")
    check(isinstance(ids, list) and isinstance(rows, list) and len(set(ids)) == len(ids), "invalid active lease IDs")
    result = {r["reservation_id"]: resources(r) for r in rows}
    check(len(result) == len(rows) and set(result) == set(ids), "claim/lease identities not bijective")
    return result


def idle_charges(snapshot, guard):
    check(charge_map(snapshot) == {guard: {"ram_bytes": GUARD, "vram_bytes": 0}}, "wrong sole guard charges")
    check(resources(snapshot.get("resident_claims")) == {"ram_bytes": RESIDENT, "vram_bytes": 0}, "wrong resident charges")


def empty_charges(snapshot):
    check(not charge_map(snapshot) and snapshot.get("resident_claims") is None, "stop ACK left charges")


def registry_projection(registry):
    runtime = one(registry["runtimes"], lambda r: r["runtime_id"] == "candle", "Candle registry")
    resident = runtime.get("model_resource_residency")
    return {"active_lease_ids": runtime["active_reservation_ids"],
            "active_claims": runtime.get("active_reservation_claims", []),
            "resident_claims": resident.get("requirements") if resident else None,
            "resident_uncertain": runtime.get("resident_resources_uncertain", False)}


def registry_empty(registry):
    check(not registry["reservations"], "registry leases remain after shutdown/refusal")
    for runtime in registry["runtimes"]:
        check(not runtime["active_reservation_ids"] and not runtime.get("active_reservation_claims", [])
              and runtime.get("model_resource_residency") is None and not runtime.get("resident_resources_uncertain", False),
              "registry charges remain")
        check(runtime.get("runtime_instance_id") is None and not runtime.get("models", []), "registry native owner/model remains")
        if runtime["runtime_id"] == "candle":
            check(runtime["status"] == "stopped", "Candle did not acknowledge stopped state")


def sqlite_events(path):
    # Original database is read as bytes only. Any SQLite journal work is local.
    with tempfile.TemporaryDirectory(prefix="m4-v2-review-sqlite-") as directory:
        target = Path(directory) / path.name
        for suffix in ("", "-wal", "-shm"):
            original = Path(str(path) + suffix)
            if original.exists():
                shutil.copyfile(original, str(target) + suffix)
        with sqlite3.connect(target) as database:
            rows = database.execute("SELECT event_seq,workflow_run_id,node_id,occurred_at_ms,payload_json FROM diagnostic_events ORDER BY event_seq").fetchall()
    return [{"event_seq": seq, "workflow_run_id": run, "node_id": node,
             "occurred_at_ms": timestamp, "payload": json.loads(payload)}
            for seq, run, node, timestamp, payload in rows]


def frozen_sources(repo, manifest):
    names = list(manifest)
    queries = "".join(f"{HEAD}:{name}\n" for name in names).encode()
    output = io.BytesIO(subprocess.check_output(["git", "-C", str(repo), "cat-file", "--batch"], input=queries))
    for name in names:
        fields = output.readline().decode().split()
        check(len(fields) == 3 and fields[1] == "blob", "frozen source blob missing: " + name)
        data = output.read(int(fields[2]))
        check(output.read(1) == b"\n" and hashlib.sha256(data).hexdigest() == manifest[name], "frozen source mismatch: " + name)


def verify_freeze(root, repo, report):
    source = read(root / "source-freeze.json")
    check(source["head"] == HEAD and source["tree"] == TREE, "wrong v2 source freeze")
    check(subprocess.check_output(["git", "-C", str(repo), "rev-parse", HEAD + "^{tree}"]).decode().strip() == TREE, "wrong frozen tree")
    frozen_sources(repo, source["source_sha256"])
    frozen_plan = subprocess.check_output(["git", "-C", str(repo), "show", HEAD + ":docs/plans/cpu-scheduler/plan.md"])
    check(hashlib.sha256(frozen_plan).hexdigest() == source["contract_sha256"], "frozen contract mismatch")
    manifest = read(root / "pre-run-manifest-sha256.json")
    for name, expected in manifest.items():
        check(sha(root / name) == expected, "pre-run freeze file changed: " + name)
    binary = read(root / "binary-freeze.json")
    check(binary["source_head"] == HEAD and binary["source_tree"] == TREE
          and binary["protocol"] == PROTOCOL and binary["profile"] == "release"
          and set(binary["features"]) == {"backend-candle", "host-dependency-inventory"}
          and binary["default_features"] is False and binary["build_exit"] == 0
          and binary["attempts"] == 1 and binary["retries_allowed"] is False, "binary freeze scope changed")
    check(sha(Path(binary["binary_path"])) == binary["binary_sha256"]
          == "ff1f35ac0044f987b25cf8d939df68a8e4e00440dce9b420da137f2524f6e539", "frozen v2 release binary changed")
    fixture = read(root / "fixture-freeze.json")
    check(len(fixture["fixture_file_sha256"]) == 14, "wrong fixture population")
    for name, expected in fixture["fixture_file_sha256"].items():
        check(sha(repo / "crates/inference/tests/fixtures/candle_bert" / name) == expected, "fixture changed: " + name)
    accounting = read(root / "accounting-freeze.json")
    for width in WORK:
        model = accounting["models"][str(width)]
        check(model["logical_selected_package_bytes"] * 3 + 512 * MIB == TASK[width], "authoritative package floor changed")
        check(model["expected_actual_intent_ram_max_bytes"] == model["expected_actual_intent_vram_max_bytes"] == TASK[width], "task maxima changed")
    check(accounting["main"]["ram_capacity_bytes"] == 1104 * MIB
          and accounting["main"]["vram_capacity_bytes"] == 1024 * MIB
          and accounting["main"]["declared_guard_ram_bytes"] == GUARD
          and accounting["main"]["declared_resident_ram_bytes"] == RESIDENT
          and accounting["pressure"]["ram_capacity_bytes"] == 80 * MIB, "frozen budgets changed")
    environment = read(root / "environment-freeze.json")
    check(environment["protocol"] == PROTOCOL and environment["attempts"] == 1 and environment["retries"] == 0
          and environment["cancellation_cases_in_this_invocation"] == 0
          and environment["cpu_threads_per_private_gateway"] == 1, "environment scope changed")
    check(environment["fixed_runtime_env"]["PUMAS_REGISTRY_DB_PATH"] == str(root / "main/registry.db"), "wrong isolated registry path")
    check("PANTOGRAPH_CPU_EMBEDDING_FIXTURE_ROOT" in environment["explicit_runtime_unsets"], "fixture override not excluded")
    report["freeze"] = {"head": HEAD, "tree": TREE, "source_files_verified": len(source["source_sha256"]),
                        "fixture_files_verified": 14, "source_manifest_sha256": sha(root / "source-freeze.json"),
                        "pre_run_manifest_files_verified": len(manifest), "release_binary_sha256": binary["binary_sha256"],
                        "accounting_manifest_sha256": sha(root / "accounting-freeze.json"),
                        "environment_manifest_sha256": sha(root / "environment-freeze.json")}
    return fixture, environment


def verify_prior_evidence(report):
    prefix_path = Path("/workspace/m4-prefix-review-v1/final-report.json")
    supplement_path = Path("/workspace/m4-cancellation-review-v1/review.json")
    check(sha(prefix_path) == "2ea0afa8648bd643e963b632e369b6e17002f22af65692b0d0b53fefecbab827", "original prefix review changed")
    check(sha(supplement_path) == "bf59970f50683b3952f2491042ecb61599321c344593255b4f75d27076046797", "supplement review changed")
    prefix = read(prefix_path)
    prior_root = Path("/workspace/m4-qualification-v1")
    archive = Path("/workspace/pantograph-m4/docs/plans/cpu-scheduler/reports/m4-main-failure-v1/original-main-evidence.tar.gz")
    check(sha(archive) == "d6780daaa309f1e00b31f03e695c3bc5bed1ad2633ff68fd224bb276581b85fc", "original main archive changed")
    for name, expected in prefix["preservation"]["original_main_file_sha256"].items():
        check(sha(prior_root / name) == expected, "original failed main file changed: " + name)
    supplement = read(supplement_path)
    for name, expected in supplement["preservation"]["original_nonbinary_file_sha256"].items():
        check(sha(Path("/workspace/m4-cancellation-v1") / name) == expected, "original supplement changed: " + name)
    check(sha(prior_root / "binaries/frozen-cpu-embedding-workflow-probe") == "c10590a7b8b9bd0b4462ed3e6b2bc2c5d727d33f393c878ca9c3aed6e1a4b17f", "old main binary changed")
    report["prior_preservation"] = {"original_main_exit": 1, "original_main_files_verified": 54,
                                    "supplement_files_verified": 16, "supplement_cases_reexecuted": 0}


def verify_artifact(root, step, index, width, reference, events):
    run = step["response"]["workflow_run_id"]
    vector = one(step["response"]["outputs"], lambda r: r["node_id"] == "vectors" and r["port_id"] == "vector", "public vector")["value"]
    expected = reference["golden_vector"]
    check(len(vector) == len(expected) == width and all(type(v) in (int, float) and math.isfinite(v) for v in vector), "wrong vector shape/value")
    error = [abs(a - b) for a, b in zip(vector, expected)]
    check(all(d <= reference["atol"] + reference["rtol"] * abs(b) for d, b in zip(error, expected)), "golden mismatch")
    facts = [e for e in events if e["workflow_run_id"] == run and e["payload"]["payload_type"] == "io_artifact_observed"
             and e["payload"].get("producer_node_id") == "vectors" and e["payload"].get("producer_port_id") == "vector"]
    check(len(facts) == 2 and {e["payload"]["artifact_role"] for e in facts} == {"node_output", "workflow_output"}, "raw vector artifact facts missing")
    ids = {e["payload"]["payload_artifact_id"] for e in facts}
    check(len(ids) == 1, "vector facts do not identify one body")
    manifest = read(root / "app-data/serial-cpu-artifacts/manifest.json")
    artifact = one(manifest["artifacts"], lambda a: a["descriptor"]["artifact_id"] in ids, "stored vector artifact")
    descriptor = artifact["descriptor"]
    body = (root / "app-data/serial-cpu-artifacts/bodies" / artifact["body_file"]).read_bytes()
    check(json.loads(body) == vector and descriptor["attribution"]["workflow_run_id"] == run, "stored vector/public run mismatch")
    check(all(e["payload"]["content_hash"] == descriptor["content_hash"]
              and e["payload"]["size_bytes"] == len(body) for e in facts), "raw artifact hash/size joins disagree")
    retained = read(root / f"step-{index}-retained-readback.json")
    check(len(retained["retained_vectors"]) == 1 and retained["retained_vectors"][0]["value"] == vector, "retained readback differs")
    check(retained["retained_vectors"][0]["artifact"]["workflow_run_id"] == run, "readback points to another run")
    refresh = read(root / f"step-{index}-projection-refresh.json")
    boundary = read(root / f"step-{index}-projection-boundary.json")
    inspection = read(root / f"step-{index}-inspection.json")
    check(not refresh["failed"] and len(refresh["advanced"]) == 3, "projection refresh failed/partial")
    # Check independent event-kind boundaries for the raw prefix visible then.
    latest = integer(boundary.get("visible_latest_event_seq"), "projection boundary")
    prefix = [e for e in events if e["event_seq"] <= latest]
    check(len(prefix) == boundary["visible_event_count"] < 500, "projection prefix incomplete/over bound")
    required = {kind: max((e["event_seq"] for e in prefix if e["payload"]["payload_type"] in wanted), default=0)
                for kind, wanted in KIND_FILTERS.items()}
    check(required == boundary["required_cursors"], "wrong relevant-event boundaries")
    for kind, cursor in required.items():
        advance = one(refresh["advanced"], lambda row: row["projection_kind"] == kind, "projection kind")
        check(advance["projection_state"]["status"] == "current"
              and advance["projection_state"]["last_applied_event_seq"] == cursor, "projection cursor mismatch")
    check(inspection["run"]["workflow_run_id"] == run and inspection["run"]["status"] == "completed", "missing projected terminal")
    input_fact = one(events, lambda e: e["workflow_run_id"] == run and e["payload"]["payload_type"] == "io_artifact_observed"
                     and e["payload"]["artifact_role"] == "workflow_input", "actual input fact")
    input_artifact = one(manifest["artifacts"], lambda a: a["descriptor"]["artifact_id"] == input_fact["payload"]["payload_artifact_id"], "stored input")
    check(sha(root / "app-data/serial-cpu-artifacts/bodies" / input_artifact["body_file"]) == reference["text_sha256"], "input changed")
    return {"width": width, "run_id": run, "artifact_id": descriptor["artifact_id"],
            "body_sha256": hashlib.sha256(body).hexdigest(), "body_bytes": len(body), "max_absolute_error": max(error),
            "recorded_blake3_joins_agree": True, "blake3_body_recomputed": False, "projection_cursors": required}


def verify_owner(root, name, work, fixture, seen_runs, instances):
    directory = root / "main" / name
    for file, expected_hash in fixture["fixture_file_sha256"].items():
        width_name, relative = file.split("/", 1)
        copied = directory / "shared-resources/models/embedding/qualification" / ("synthetic-" + width_name) / relative
        check(sha(copied) == expected_hash, "owner copied model/fixture bytes differ: " + name + "/" + file)
    result = read(directory / "owner-result.json")
    check(result["fixed_work"] == work and result["execution_ok"] is True and result["execution_error"] is None, name + " execution failed")
    check(result["shutdown_ok"] is True and not result["shutdown_wait_timed_out"] and result["shutdown_error"] is None,
          name + " shutdown not acknowledged")
    check(all(row["ok"] is True for row in result["session_closes"]), name + " metadata close failed")
    registry_empty(result["registry_after_shutdown"])
    records = read(directory / "native-and-service-log.json")
    check(records == result["native_records"], "owner native log copies disagree")
    events = read(directory / "events.json")
    check(events == sqlite_events(directory / "app-data/serial-cpu-diagnostics.db"), "raw SQLite/export disagreement")
    check(len(events) == result["event_count"] < 500 and result["event_error"] is None, "raw event count/error")
    check(all(events[i]["event_seq"] < events[i+1]["event_seq"] for i in range(len(events)-1)), "raw event order broken")
    pressure = name == "pressure"
    native = [row["record"] for row in records if row["target"] == "embedded::serial_cpu_residency"]
    grants = [row["record"] for row in records if row["target"] == "workflow::serial_ready" and row["record"].get("phase") == "service_opportunity"]
    rows = result["rows"]
    check(len(rows) == len(work) and len(grants) == len(work), "wrong actual opportunity/public-call count")
    expected = []
    for i in range(len(work)):
        expected += ["preparation_plan"]
        if not pressure:
            if i and work[i] != work[i-1]:
                expected += ["switch_stop_ack"]
            expected += ["cold_loaded" if not i or work[i] != work[i-1] else "retained", "native_drained", "task_registry_ack"]
        else:
            expected += ["preparation_refused"]
    if not pressure:
        expected += ["shutdown_stop_ack"]
    check([r.get("phase") for r in native] == expected, "unexpected phase population/order in " + name)
    position = 0
    current_owner = None
    guard = None
    old_idle = None
    previous_guard_lease = None
    outputs, intervals, identities = [], [], []
    for index, (row, width) in enumerate(zip(rows, work)):
        check(row == read(directory / f"step-{index}.json") and row["index"] == index
              and row["model_alias"] == f"bert-{width}" and row["model_id"] == f"embedding/qualification/synthetic-bert-{width}", "public work sequence mismatch")
        plan = native[position]
        position += 1
        run = plan["workflow_run_id"]
        check(run not in seen_runs and plan["task_id"] == "infer" and plan["node_id"] == "infer"
              and plan["workflow_id"] == f"cpu-m4-bert-{width}", "plan run/task/workflow identity mismatch")
        seen_runs.add(run)
        session = read(directory / f"bert-{width}-authenticated-session.json")
        snapshot_event = one(events, lambda e: e["workflow_run_id"] == run
                             and e["payload"]["payload_type"] == "run_snapshot_accepted", "accepted run snapshot")
        check(row["session_id"] == session["session_id"]
              == snapshot_event["payload"]["workflow_execution_session_id"], "actual session/run snapshot join failed")
        check(one(grants, lambda g: g["workflow_run_id"] == run, "actual service grant")["task_id"] == plan["task_id"], "grant does not bind actual task")
        for field in ("model_ref", "resolved_model_ref"):
            ref = plan[field]
            check(ref["model_id"].removeprefix("pumas://models/") == row["model_id"]
                  and ref["selected_artifact_id"] == "main" and ref.get("revision") is None
                  and ref.get("selected_artifact_path_digest") is None, "logical/resolved model target changed")
        check((plan["runtime_id"], plan["runtime_variant_id"], plan["device_id"]) == ("candle", "candle.cpu", "cpu"), "wrong native runtime/device")
        check(plan["actual_task_ram_bytes"] == plan["actual_task_vram_bytes"] == plan["package_conservative_floor_bytes"] == TASK[width], "actual task hints/floor changed")
        check(plan["required_ram_bytes"] == GUARD + RESIDENT + TASK[width] and plan["required_vram_bytes"] == TASK[width]
              and plan["guard_or_task_acquired_by_this_preparation"] is False, "wrong pre-admission arithmetic")
        check((plan["declared_guard_ram_bytes"], plan["declared_guard_vram_bytes"], plan["declared_resident_ram_bytes"], plan["declared_resident_vram_bytes"])
              == (GUARD, 0, RESIDENT, 0), "declarations changed")
        budget = {r["kind"]: (r["total_bytes"], r["safety_margin_bytes"]) for r in plan["declared_capacity"]["resources"]}
        check(budget == {"ram_bytes": ((80 if pressure else 1104) * MIB, 0), "vram_bytes": (1024 * MIB, 0)}, "plan budget changed")
        lifecycle = [e for e in events if e["workflow_run_id"] == run and e["node_id"] == "infer"
                     and e["payload"]["payload_type"] == "scheduler_task_attempt_lifecycle_changed"]
        terminal = one(events, lambda e: e["workflow_run_id"] == run and e["payload"]["payload_type"] == "run_terminal", "actual run terminal")
        if pressure:
            refused = native[position]
            position += 1
            check(refused["preparation_request_id"] == plan["preparation_request_id"] and refused["refusal_stage"] == "headroom_admission"
                  and refused["no_task_start_authorized"] is True, "pressure not an actual headroom refusal")
            check(plan["required_ram_bytes"] > 80 * MIB and row["response"] is None and isinstance(row["error"], str)
                  and row["error"] and not lifecycle and terminal["payload"]["status"] == "failed", "pressure executed or lacked Failed terminal")
            registry_empty(row["registry_before"])
            registry_empty(row["registry_after"])
            diagnostics = result["diagnostics_after_shutdown"]
            check(all(diagnostics[k] == 0 for k in ("loads", "forward_drains", "retains", "stops"))
                  and not diagnostics["fenced"] and not diagnostics["has_idle_owner"], "pressure mutated native state")
            continue
        check(row["error"] is None and row["response"]["workflow_run_id"] == run and terminal["payload"]["status"] == "completed", "actual successful run missing")
        check(isinstance(row["readiness_resume_run_ids"], list)
              and all(resumed == run for resumed in row["readiness_resume_run_ids"]), "readiness resume changed actual run")
        same = index > 0 and work[index] == work[index-1]
        if index:
            check(registry_projection(row["registry_before"]) == old_idle, "previous guard/resident missing before next public call")
            check(row["registry_before"]["reservations"] == [previous_guard_lease], "original guard lease incarnation mutated before next call")
        else:
            registry_empty(row["registry_before"])
        if index and not same:
            stopped = native[position]
            position += 1
            check(stopped["old_owner"] == current_owner and stopped["preparation_request_id"] == plan["preparation_request_id"], "switch stopped wrong owner")
            check(stopped["charged_before_stop"] == old_idle, "old charges changed before switch ACK")
            idle_charges(stopped["charged_before_stop"], guard)
            empty_charges(stopped["charged_after_ack"])
            intervals.append({"kind": "switch_stop", "physical_stop_ns": integer(stopped.get("stop_ns"), "physical stop"),
                              "registry_ack_ns": integer(stopped.get("acknowledgement_ns"), "stop ACK")})
        prepared, drained, ack = native[position:position+3]
        position += 3
        check(prepared["preparation_request_id"] == plan["preparation_request_id"], "preparation join failed")
        check(prepared["owner"] == drained["previous_owner"], "prepared owner differs from genuine receipt predecessor")
        warm(drained["previous_owner"], drained["current_owner"])
        if same:
            check(prepared["owner"] == current_owner, "retained owner was relabelled/replaced")
        else:
            guard = integer(prepared.get("guard_lease_id"), "guard ID")
            instance = tuple(owner(prepared["owner"])["loaded_instance"])
            check(instance not in instances, "cold load/reload reused a previous private native instance")
            instances.add(instance)
            idle_charges(prepared["charged_after_publication"], guard)
            intervals.append({"kind": "cold_load" if index == 0 else "cold_switch_load",
                              "load_ns": integer(prepared.get("load_ns"), "cold load"),
                              "publication_ns": integer(prepared.get("publication_ns"), "cold publication")})
        check(all(drained.get(k) == ack.get(k) and drained.get(k) is not None for k in IDENTITY), "native/ACK full identity mismatch")
        check(all(plan[k] == drained[k] for k in ("workflow_id", "workflow_run_id", "node_id", "task_id")), "plan/native identity mismatch")
        check(drained["registry_task_release_acknowledged"] is False and ack["registry_task_release_acknowledged"] is True
              and ack["scheduler_cleanup_acknowledged"] is False, "native/registry/scheduler ACK conflated")
        task = integer(drained.get("reservation_lease_id"), "task lease")
        check(task != guard and charge_map(drained["charged_after_native_drain"])
              == {guard: {"ram_bytes": GUARD, "vram_bytes": 0}, task: {"ram_bytes": TASK[width], "vram_bytes": TASK[width]}}, "actual acquired task/full guard charges wrong")
        idle_charges(ack["charged_after_registry_ack"], guard)
        removed = copy.deepcopy(drained["charged_after_native_drain"])
        removed["active_lease_ids"].remove(task)
        removed["active_claims"] = [r for r in removed["active_claims"] if r["reservation_id"] != task]
        check(removed == ack["charged_after_registry_ack"], "task ACK modified more than the exact task lease")
        check(registry_projection(row["registry_after"]) == ack["charged_after_registry_ack"], "public return preceded matching task ACK")
        guard_lease = one(row["registry_after"]["reservations"], lambda lease: lease["reservation_id"] == guard, "original mode guard lease")
        check(len(row["registry_after"]["reservations"]) == 1 and guard_lease["runtime_id"] == "candle"
              and guard_lease["workflow_id"] == "cpu-serial-residency-owner"
              and guard_lease["model_id"] == row["model_id"] and not guard_lease["pin_runtime"]
              and guard_lease["retention_hint"] == "ephemeral", "guard identity/retention changed")
        if same:
            check(guard_lease == previous_guard_lease, "retained guard lease incarnation changed")
        else:
            check(guard_lease["reservation_owner_id"] == plan["preparation_request_id"], "guard not owned by its actual cold preparation")
        previous_guard_lease = guard_lease
        started = one(lifecycle, lambda e: e["payload"]["transition"] == "started", "actual runtime Start")
        completed = one(lifecycle, lambda e: e["payload"]["transition"] == "completed", "actual runtime Completed")
        check(len(lifecycle) == 2 and started["event_seq"] < completed["event_seq"], "extra/reordered runtime attempt")
        for event in (started, completed):
            payload = event["payload"]
            check(payload["execution_class"] == "runtime" and payload["scheduler_task_id"] == drained["task_id"]
                  and payload["scheduler_attempt_id"] == drained["attempt_id"], "raw attempt identity mismatch")
        payload = completed["payload"]
        check(payload["reservation_id"] == f"runtime-registry.{task}" and payload["selected_runtime_id"] == "candle"
              and payload["selected_runtime_variant_id"] == "candle.cpu" and payload["selected_device_id"] == "cpu", "Completed dispatch/lease mismatch")
        intervals.append({"kind": "forward_and_drain", "run_id": run,
                          "forward_and_drain_ns": integer(drained.get("forward_and_drain_ns"), "actual forward/drain")})
        outputs.append(verify_artifact(directory, row, index, width, fixture["models"][f"bert-{width}"], events))
        identities.append({k: drained[k] for k in IDENTITY} | {"previous_owner": drained["previous_owner"], "current_owner": drained["current_owner"]})
        current_owner = drained["current_owner"]
        old_idle = ack["charged_after_registry_ack"]
    if not pressure:
        stopped = native[position]
        position += 1
        check(stopped["old_owner"] == current_owner and stopped["charged_before_stop"] == old_idle, "shutdown did not consume final actual owner")
        empty_charges(stopped["charged_after_ack"])
        check(registry_projection(result["registry_before_shutdown"]) == old_idle, "final guard/resident disappeared before stop")
        intervals.append({"kind": "final_stop", "physical_stop_ns": integer(stopped.get("stop_ns"), "final physical stop"),
                          "registry_ack_ns": integer(stopped.get("acknowledgement_ns"), "final ACK")})
    check(position == len(native), "unreviewed native phases remain")
    expected_counts = {"loads": 0 if pressure else 3 if len(work) == 5 else 1,
                       "retains": 2 if len(work) == 5 else 0,
                       "forward_drains": 0 if pressure else len(work),
                       "stops": 0 if pressure else 3 if len(work) == 5 else 1}
    diagnostics = result["diagnostics_after_shutdown"]
    check(all(diagnostics[k] == value for k, value in expected_counts.items())
          and not diagnostics["fenced"] and not diagnostics["has_idle_owner"], "counters differ from independently counted raw phases")
    # Root-specific facts are preserved, not treated as global native identity.
    return {"owner": name, "work": work, "counts": expected_counts, "outputs": outputs,
            "native_identities": identities, "intervals_ns": intervals, "event_count": len(events),
            "records": records}


def review(args, report):
    root, repo = args.input.resolve(), args.repo.resolve()
    fixture, environment = verify_freeze(root, repo, report)
    verify_prior_evidence(report)
    statuses = [p for p in (root / "execution-status.json", root / "main-execution-status.json") if p.exists()]
    check(len(statuses) == 1, "exactly one completed execution status is required")
    status = read(statuses[0])
    check(status["state"] == "completed" and status["exit_code"] == 0 and status["attempt"] == 1
          and status["retry_allowed"] is False and status["source_head"] == HEAD, "v2 process incomplete/failed/retried/wrong source")
    binary = root / "binaries/frozen-cpu-embedding-workflow-probe"
    check(sha(binary) == status["binary_sha256"], "executed frozen binary mismatch")
    check(status["argv"] == [str(binary), str(root / "main"), "--serial-residency"], "wrong native invocation scope")
    report["execution"] = {"attempt": 1, "exit_code": 0, "started_utc": status["started_utc"],
                           "completed_utc": status["completed_utc"], "binary_sha256": sha(binary)}
    manifest = read(root / "main/provenance.json")
    check(manifest["source_head"] == HEAD and manifest["source_head_tree"] == TREE
          and not manifest["source_status"] and manifest["binary_sha256"] == sha(binary), "runtime provenance differs from freeze")
    for name, expected in environment["fixed_runtime_env"].items():
        if name in manifest["environment"]:
            check(manifest["environment"][name] == expected, "runtime environment mismatch: " + name)
    summaries = []
    runs, instances = set(), set()
    summaries.append(verify_owner(root, "retained", WORK, fixture, runs, instances))
    for index, width in enumerate(WORK):
        summaries.append(verify_owner(root, f"comparison-{index}", [width], fixture, runs, instances))
    summaries.append(verify_owner(root, "pressure", [8], fixture, runs, instances))
    # Require each acknowledged teardown to precede the next owner's actual
    # first event using raw wall-clock records, with source order as support.
    starts = []
    for summary in summaries:
        directory = root / "main" / summary["owner"]
        events = read(directory / "events.json")
        starts.append(min(e["occurred_at_ms"] for e in events))
    for old, next_start in zip(summaries, starts[1:]):
        state = read(root / "main" / old["owner"] / "owner-result.json")["registry_after_shutdown"]
        check(max(r["last_transition_at_ms"] for r in state["runtimes"]) <= next_start, "later owner started before prior stop/ACK snapshot")
    log = read(root / "main/native-and-service-log.json")
    joined = [row for summary in summaries for row in summary.pop("records")]
    check(log["rows"] == joined and len(joined) == 61 <= 128 and log["overflow"] is False, "collector count/order/overflow mismatch")
    line_sizes = [len(json.dumps(r["record"], sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()) for r in joined]
    check(max(line_sizes) < 8192 and sum(line_sizes) == log["bytes"] < 1024 * 1024, "collector size bounds/missing rows")
    totals = {key: sum(s["counts"][key] for s in summaries) for key in ("loads", "retains", "forward_drains", "stops")}
    check(totals == {"loads": 8, "retains": 2, "forward_drains": 10, "stops": 8}
          and len(instances) == 8 and len(runs) == 11, "global actual work/instance/run totals mismatch")
    report["owners"] = summaries
    report["totals"] = totals | {"distinct_actual_native_instances": len(instances), "public_runs_including_pressure": len(runs), "collected_rows": 61}
    report["main_count_and_pressure_criteria_passed"] = True
    report["acceptance_scope"] = "independent saved-data v2 main counts/output/ownership/pressure review only; overall M4 acceptance requires combined source/tests/supplement/CI review"
    report["limitations"] = ["Native lifecycle authority is the independently reviewed frozen source and genuine private receipts; JSON facts do not grant capability authority.",
                             "Log order and per-phase durations are real observations; separate buffering and clock granularity do not provide per-component wall-clock timestamps.",
                             "Missing intervals fail instead of being replaced by zero; forward/drain is not split into compute-only time.",
                             "Generic candle-1 and lease IDs may repeat across private owners; only full private loaded-instance/profile/settings/generation joins are used for native lineage.",
                             "Artifact body SHA256, actual parsed values, golden tolerances, raw event/manifest stored-BLAKE3 joins are verified; BLAKE3 is not recomputed by this standard-library script.",
                             "No speedup, allocator/RSS budget, GPU, semantic-quality, fairness-equivalence, interrupted-compute or training claim follows."]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, default=Path("/workspace/m4-main-v2"))
    parser.add_argument("--repo", type=Path, default=Path("/workspace/pantograph-m4"))
    parser.add_argument("--output", type=Path, default=Path("/workspace/m4-main-v2-review/report.json"))
    args = parser.parse_args()
    check(not args.output.resolve().is_relative_to(args.input.resolve()), "output must be outside original evidence")
    before = hashes(args.input / "main") if (args.input / "main").exists() else {}
    report = {"review": "prospectively declared independent saved-data checks", "checks_passed": False,
              "main_count_and_pressure_criteria_passed": False, "native_invocations_by_reviewer": 0}
    try:
        review(args, report)
        report["checks_passed"] = True
    except Exception as error:
        report["finding"] = f"{type(error).__name__}: {error}"
        report["acceptance_scope"] = "Partial/Blocked; no saved-data failure permits native continuation or retry"
    after = hashes(args.input / "main") if (args.input / "main").exists() else {}
    report["preservation"] = {"input_files_unchanged_by_review": before == after, "main_file_sha256": before}
    check(before == after, "input evidence changed during review")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x") as stream:
        json.dump(report, stream, indent=2, sort_keys=True)
        stream.write("\n")
    print(json.dumps({"checks_passed": report["checks_passed"], "report": str(args.output), "finding": report.get("finding")}))
    raise SystemExit(0 if report["checks_passed"] else 1)


if __name__ == "__main__":
    main()
