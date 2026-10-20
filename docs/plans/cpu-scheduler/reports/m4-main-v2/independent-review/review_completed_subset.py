#!/usr/bin/env python3
"""Saved-data-only decomposition of FAILED v2; never changes admission criteria.

Imports unchanged pre-outcome owner/freeze checks. No runtime/model/Cargo calls.
SQLite uses the imported helper's temporary byte copies, never original DBs.
Exit 0 means this partial-failure analysis reproduced, NOT cohort acceptance.
"""
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = Path('/workspace/m4-main-v2')
REPO = Path('/workspace/pantograph-m4')


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    strict = load('prospective_strict', 'review_saved_main.py')
    initial = load('prospective_pinned', 'review_saved_main.initial.py')
    read, check, sha = strict.read, strict.check, strict.sha
    check(sha(HERE / 'review_saved_main.py') == '747465e0debd5b358e3031bc4960b48c83d3c8ea08f805e9b742d76e3bd73a1f', 'strict helper changed')
    check(sha(HERE / 'review_saved_main.initial.py') == 'e8267d1b1f741820c1e159784e8356b66984af7ca72d2bc5802cae41a5361985', 'pinned helper changed')
    before = strict.hashes(ROOT)
    preserved_reports = {name: sha(HERE / name) for name in ('report.json', 'original-prospective-report.json', 'original-prospective-review-status.json')}
    report = {'review': 'independent saved-data-only completed-owner subset and pressure FAILURE decomposition',
              'native_invocations_by_reviewer': 0, 'overall_main_accepted': False,
              'main_count_and_pressure_criteria_passed': False, 'M4_C4': 'Partial/Blocked',
              'criteria_relaxed': False, 'native_retry_authorized': False}
    fixture, environment = strict.verify_freeze(ROOT, REPO, report)
    strict.verify_prior_evidence(report)
    status = read(ROOT / 'execution-status.json')
    binary = ROOT / 'binaries/frozen-cpu-embedding-workflow-probe'
    check(status['state'] == 'completed' and status['exit_code'] == 1
          and status['attempt_within_v2'] == 1 and status['retry_allowed'] is False, 'actual failed one-shot status changed')
    check(status['source_head'] == strict.HEAD and status['source_tree'] == strict.TREE
          and status['binary_sha256'] == sha(binary), 'executed source/binary differs')
    check(status['argv'] == [str(binary), str(ROOT / 'main'), '--serial-residency'], 'execution scope changed')
    check(status['independent_review_script_sha256'] == sha(HERE / 'review_saved_main.initial.py'), 'execution did not pin original helper')
    provenance = read(ROOT / 'main/provenance.json')
    check(provenance['source_head'] == strict.HEAD and provenance['source_head_tree'] == strict.TREE
          and not provenance['source_status'] and provenance['binary_sha256'] == sha(binary), 'runtime provenance differs')
    for name, value in environment['fixed_runtime_env'].items():
        if name in provenance['environment']:
            check(provenance['environment'][name] == value, 'runtime environment changed: ' + name)
    result = read(ROOT / 'main/m4-result.json')
    check(result['main_and_pressure_protocols_ok'] is False and result['native_cohort_retries'] == 0
          and result['log_overflow'] is False and result['execution_error'] == 'M4 close/shutdown did not acknowledge every actual owner', 'overall failure changed')
    report['execution'] = status
    report['producer_overall_failure'] = result['execution_error']
    report['original_prospective_failure_preserved'] = preserved_reports
    summaries, runs, instances = [], set(), set()
    owners = [('retained', strict.WORK)] + [(f'comparison-{i}', [w]) for i, w in enumerate(strict.WORK)]
    for name, work in owners:
        summaries.append(strict.verify_owner(ROOT, name, work, fixture, runs, instances))
    # Also reproduce all six checks from the EXACT helper pinned at invocation.
    pinned_runs, pinned_instances = set(), set()
    for name, work in owners:
        initial.verify_owner(ROOT, name, work, fixture, pinned_runs, pinned_instances)
    check(runs == pinned_runs and instances == pinned_instances, 'unchanged helpers disagree on six completed owners')
    check(len(runs) == 10 and len(instances) == 8, 'successful run/native instance count wrong')
    totals = {k: sum(s['counts'][k] for s in summaries) for k in ('loads', 'retains', 'forward_drains', 'stops')}
    check(totals == {'loads': 8, 'retains': 2, 'forward_drains': 10, 'stops': 8}, 'completed-owner physical totals wrong')
    pressure_dir = ROOT / 'main/pressure'
    pressure = read(pressure_dir / 'owner-result.json')
    events = read(pressure_dir / 'events.json')
    check(events == strict.sqlite_events(pressure_dir / 'app-data/serial-cpu-diagnostics.db'), 'pressure raw SQLite/export differ')
    expected_kinds = ['run_snapshot_accepted', 'library_asset_accessed', 'scheduler_estimate_produced', 'scheduler_queue_placement']
    check([e['event_seq'] for e in events] == [1, 2, 3, 4]
          and [e['payload']['payload_type'] for e in events] == expected_kinds, 'pressure initial event population changed')
    terminal = [e for e in events if e['payload']['payload_type'] == 'run_terminal']
    lifecycle = [e for e in events if e['payload']['payload_type'] == 'scheduler_task_attempt_lifecycle_changed']
    check(not terminal and not lifecycle and pressure['event_count'] == 4 and pressure['event_error'] is None, 'pressure terminal/Start facts changed')
    records = read(pressure_dir / 'native-and-service-log.json')
    check(records == pressure['native_records'] and len(records) == 3, 'pressure log population differs')
    grant, plan, refused = [r['record'] for r in records]
    check([r['phase'] for r in (grant, plan, refused)] == ['service_opportunity', 'preparation_plan', 'preparation_refused'], 'pressure phase order changed')
    run = plan['workflow_run_id']
    check(run not in runs and all(e['workflow_run_id'] == run for e in events)
          and grant['workflow_run_id'] == run and grant['task_id'] == plan['task_id'] == plan['node_id'] == 'infer', 'pressure actual grant/run mismatch')
    row = strict.one(pressure['rows'], lambda r: r['index'] == 0, 'pressure row')
    check(row == read(pressure_dir / 'step-0.json') and row['response'] is None
          and row['error'] == 'internal_error: invalid_request: runtime-host execution port failed: additive guard/resident/task CPU residency headroom refused', 'pressure public refusal changed')
    check(events[0]['payload']['workflow_execution_session_id'] == row['session_id'], 'pressure session mismatch')
    check(plan['actual_task_ram_bytes'] == plan['actual_task_vram_bytes'] == plan['package_conservative_floor_bytes'] == strict.TASK[8]
          and plan['required_ram_bytes'] == strict.GUARD + strict.RESIDENT + strict.TASK[8] == 620781187
          and plan['required_vram_bytes'] == strict.TASK[8] and not plan['guard_or_task_acquired_by_this_preparation'], 'pressure additive headroom arithmetic wrong')
    budget = {r['kind']: (r['total_bytes'], r['safety_margin_bytes']) for r in plan['declared_capacity']['resources']}
    check(budget == {'ram_bytes': (80 * strict.MIB, 0), 'vram_bytes': (1024 * strict.MIB, 0)}
          and plan['required_ram_bytes'] > budget['ram_bytes'][0], 'pressure was not short of RAM')
    check(refused['preparation_request_id'] == plan['preparation_request_id']
          and refused['refusal_stage'] == 'headroom_admission' and refused['no_task_start_authorized'] is True, 'pressure refusal did not bind actual preparation')
    for snapshot in (row['registry_before'], row['registry_after'], pressure['registry_before_shutdown'], pressure['registry_after_shutdown']):
        strict.registry_empty(snapshot)
    diag = pressure['diagnostics_after_shutdown']
    check(all(diag[k] == 0 for k in ('loads', 'retains', 'forward_drains', 'stops'))
          and not diag['fenced'] and not diag['has_idle_owner'], 'pressure physical work/owner exists')
    close = strict.one(pressure['session_closes'], lambda c: c['kind'] == 'execution_session', 'pressure execution-session close')
    check(close['ok'] is False and close['session_id'] == row['session_id']
          and close['error'] == f"scheduler_busy: session '{row['session_id']}' is currently running", 'pressure close failure changed')
    check(pressure['shutdown_ok'] is True and pressure['shutdown_error'] is None
          and not pressure['shutdown_wait_timed_out'], 'pressure native shutdown did not complete')
    check(not (pressure_dir / 'native-oracle.json').exists(), 'unexpected completed pressure native oracle')
    report['pressure'] = {'complete_pressure_gate_passed': False, 'required_failed_terminal_observed': False,
        'execution_session_close_acknowledged': False, 'native_oracle_saved': False,
        'observed_headroom_refusal_before_start': True, 'run_id': run, 'session_id': row['session_id'],
        'raw_event_count': 4, 'raw_event_kinds': expected_kinds, 'raw_run_terminal_count': 0,
        'raw_attempt_lifecycle_count': 0, 'required_ram_bytes': plan['required_ram_bytes'],
        'available_ram_bytes': 80 * strict.MIB, 'native_counts': {k: diag[k] for k in totals},
        'zero_registry_charges_at_all_saved_boundaries': True, 'public_error': row['error'],
        'session_close_error': close['error'], 'native_shutdown_acknowledged': True,
        'producer_execution_ok_is_only_expected_error_branch': pressure['execution_ok']}
    # Cross-owner raw timestamps support the sequential acknowledged source path.
    all_names = [name for name, _ in owners] + ['pressure']
    cross_owner = []
    for old, following in zip(all_names, all_names[1:]):
        old_state = read(ROOT / 'main' / old / 'owner-result.json')['registry_after_shutdown']
        old_ack_ms = max(r['last_transition_at_ms'] for r in old_state['runtimes'])
        next_ms = min(e['occurred_at_ms'] for e in read(ROOT / 'main' / following / 'events.json'))
        check(old_ack_ms <= next_ms, 'later owner preceded previous stopped/ACK snapshot')
        cross_owner.append({'previous': old, 'next': following, 'previous_ack_snapshot_ms': old_ack_ms, 'next_first_event_ms': next_ms})
    joined = [r for s in summaries for r in s.pop('records')] + records
    log = read(ROOT / 'main/native-and-service-log.json')
    check(log['rows'] == joined and len(joined) == 61 <= 128 and log['overflow'] is False, 'global log population/order/bound wrong')
    sizes = [len(json.dumps(r['record'], sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()) for r in joined]
    check(max(sizes) < 8192 and sum(sizes) == log['bytes'] < 1024 * 1024, 'global log size bounds wrong')
    report['completed_owners'] = summaries
    report['six_completed_owners_pass_both_unchanged_helpers'] = True
    report['positive_scoped_result'] = {'retained_loads': 3, 'fresh_comparison_loads': 5,
        'completed_runs': 10, 'distinct_actual_native_instances': 8, 'totals': totals,
        'all_ten_goldens_match': True, 'maximum_absolute_error': max(o['max_absolute_error'] for s in summaries for o in s['outputs']),
        'full_native_lineage_and_exact_task_ack_checked': True, 'all_six_owner_session_closes_acknowledged': True,
        'all_six_owner_native_shutdowns_acknowledged': True, 'global_log_rows': 61,
        'global_log_bytes': sum(sizes), 'largest_record_bytes': max(sizes), 'cross_owner_order': cross_owner}
    report['source_causal_observation'] = {
        'frozen_head': strict.HEAD,
        'worker': 'workflow/task_execution_worker.rs:1527 preparation error returns runtime_branch_failed before candidate/Start',
        'session_api': 'workflow/session_execution_api.rs:460 enqueue-and-wait outcome is returned through workflow_response_from_runtime_branch_worker_outcome; RuntimeBranchFailed maps to Internal unless cancellation',
        'conclusion': 'Raw pressure evidence has no run terminal and close remains busy. This pre-Start failure route does not establish completed session finalization. Fix/requalification is outside this read-only review; no reserved T4 helper inspected.',
        'classification': 'source-assisted explanation; absence of terminal and busy close independently proven from raw saved evidence'}
    report['limitations'] = [
        'The overall process and complete pressure criteria FAILED; successful owner subset does not change either result.',
        'Counters and native identity use actual raw phase receipts, full loaded-instance/profile/settings/generation; generic candle-1 and lease IDs are only local.',
        'Load, publication, forward-plus-drain, physical stop and registry ACK durations are distinct actual observations. No compute-only or per-component wall-clock timing inference.',
        'Source controls native capability authority and phase hook meaning; JSON does not itself grant native authority.',
        'Artifact body SHA256, parsed values, goldens and raw stored-BLAKE3 joins checked; standard-library helper does not recompute BLAKE3.',
        'No speedup, allocator/RSS, GPU, training, semantic-quality or interrupted-compute qualification.',
        'Original v1 main and cancellation supplement are separately scoped preserved evidence, not successful v2 pressure substitutes.']
    post = read(ROOT / 'post-execution-file-sha256.json')
    check(all(sha(ROOT / path) == digest for path, digest in post.items()), 'original post-execution file hash changed')
    after = strict.hashes(ROOT)
    check(before == after and all(sha(HERE / name) == digest for name, digest in preserved_reports.items()), 'review mutated evidence/original failure reports')
    report['preservation_v2'] = {'all_input_files_unchanged_by_review': True, 'input_file_count': len(before),
        'post_execution_manifest_sha256': sha(ROOT / 'post-execution-file-sha256.json'),
        'post_execution_manifest_entries_verified': len(post),
        'input_file_hash_map_sha256': hashlib.sha256(json.dumps(before, sort_keys=True, separators=(',', ':')).encode()).hexdigest(),
        'original_failure_reports_unchanged': preserved_reports}
    report['analysis_reproduced'] = True
    output = HERE / 'completed-subset-review.json'
    with output.open('x') as stream:
        json.dump(report, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps({'analysis_reproduced': True, 'overall_main_accepted': False, 'M4_C4': 'Partial/Blocked',
                      'report': str(output), 'report_sha256': sha(output)}))


if __name__ == '__main__':
    main()
