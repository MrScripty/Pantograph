use pantograph_diagnostics_ledger::{
    DiagnosticsLedgerError, DiagnosticsLedgerRepository, RuntimeHostObservationOutcome,
    RuntimeHostObservationProfile, RuntimeHostObservationQuery, RuntimeHostRequestObservation,
    SqliteDiagnosticsLedger, RUNTIME_HOST_OBSERVATION_STORED_LIMIT,
};

fn observation(id: u32, outcome: RuntimeHostObservationOutcome) -> RuntimeHostRequestObservation {
    RuntimeHostRequestObservation {
        observation_id: format!("observation.{id}"),
        execution_request_id: format!("request.{id}"),
        workflow_id: "workflow.a".into(),
        workflow_run_id: "run.a".into(),
        task_id: "task.a".into(),
        profile: RuntimeHostObservationProfile {
            host_epoch: "host.epoch.a".into(),
            request_fingerprint: "a".repeat(64),
        },
        outcome,
        host_elapsed_ms: u64::from(id),
        recorded_at_ms: i64::from(id),
    }
}

fn query() -> RuntimeHostObservationQuery {
    RuntimeHostObservationQuery {
        profile: observation(1, RuntimeHostObservationOutcome::Completed).profile,
        since_ms: 0,
        until_ms: 100,
        sample_limit: 500,
    }
}

#[test]
fn success_summary_excludes_failed_cancelled_dropped_and_nonterminal_attempts() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().unwrap();
    for (id, outcome) in [
        (10, RuntimeHostObservationOutcome::Completed),
        (20, RuntimeHostObservationOutcome::Completed),
        (1, RuntimeHostObservationOutcome::Failed),
        (2, RuntimeHostObservationOutcome::CancellationAcknowledged),
        (3, RuntimeHostObservationOutcome::ShutdownAcknowledged),
        (4, RuntimeHostObservationOutcome::Rejected),
        (5, RuntimeHostObservationOutcome::Abandoned),
        (6, RuntimeHostObservationOutcome::Nonterminal),
    ] {
        ledger
            .record_runtime_host_observation(observation(id, outcome))
            .unwrap();
    }
    let summary = ledger.runtime_host_observation_summary(query()).unwrap();
    assert_eq!(summary.observed_count, 8);
    assert_eq!(summary.completed_count, 2);
    assert_eq!(summary.other_outcome_count, 6);
    assert_eq!(summary.median_completed_host_elapsed_ms, Some(15));
    let mut empty = query();
    empty.profile.request_fingerprint = "b".repeat(64);
    let summary = ledger.runtime_host_observation_summary(empty).unwrap();
    assert_eq!(summary.completed_count, 0);
    assert_eq!(summary.median_completed_host_elapsed_ms, None);
}

#[test]
fn matching_never_falls_back_to_other_profiles_epochs_or_stale_samples() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().unwrap();
    for id in [10, 20, 30] {
        ledger
            .record_runtime_host_observation(observation(
                id,
                RuntimeHostObservationOutcome::Completed,
            ))
            .unwrap();
    }
    let mut bounded = query();
    bounded.since_ms = 15;
    bounded.until_ms = 25;
    assert_eq!(
        ledger
            .runtime_host_observation_summary(bounded.clone())
            .unwrap()
            .median_completed_host_elapsed_ms,
        Some(20)
    );
    bounded.profile.host_epoch = "other.host.epoch".into();
    assert_eq!(
        ledger
            .runtime_host_observation_summary(bounded)
            .unwrap()
            .observed_count,
        0
    );
    let mut latest = query();
    latest.sample_limit = 1;
    assert_eq!(
        ledger
            .runtime_host_observation_summary(latest)
            .unwrap()
            .median_completed_host_elapsed_ms,
        Some(30)
    );
}

#[test]
fn repeated_identity_is_idempotent_but_changed_payload_is_a_conflict() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().unwrap();
    let original = observation(10, RuntimeHostObservationOutcome::Completed);
    ledger
        .record_runtime_host_observation(original.clone())
        .unwrap();
    ledger
        .record_runtime_host_observation(original.clone())
        .unwrap();
    let mut conflict = original;
    conflict.outcome = RuntimeHostObservationOutcome::Failed;
    assert!(ledger.record_runtime_host_observation(conflict).is_err());
    assert_eq!(
        ledger
            .runtime_host_observation_summary(query())
            .unwrap()
            .completed_count,
        1
    );
}

#[test]
fn invalid_boundary_values_fail_before_persistence_or_query() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().unwrap();
    for invalid in [
        RuntimeHostRequestObservation {
            host_elapsed_ms: u64::MAX,
            ..observation(1, RuntimeHostObservationOutcome::Completed)
        },
        RuntimeHostRequestObservation {
            recorded_at_ms: -1,
            ..observation(2, RuntimeHostObservationOutcome::Completed)
        },
    ] {
        assert!(ledger.record_runtime_host_observation(invalid).is_err());
    }
    for invalid in [
        RuntimeHostObservationQuery {
            sample_limit: 0,
            ..query()
        },
        RuntimeHostObservationQuery {
            sample_limit: 501,
            ..query()
        },
        RuntimeHostObservationQuery {
            since_ms: 101,
            ..query()
        },
        RuntimeHostObservationQuery {
            profile: RuntimeHostObservationProfile {
                host_epoch: "epoch".into(),
                request_fingerprint: "not-a-digest".into(),
            },
            ..query()
        },
    ] {
        assert!(ledger.runtime_host_observation_summary(invalid).is_err());
    }
    assert_eq!(
        ledger
            .runtime_host_observation_summary(query())
            .unwrap()
            .observed_count,
        0
    );
}

#[test]
fn journal_is_bounded_and_does_not_imply_full_history() {
    let mut ledger = SqliteDiagnosticsLedger::open_in_memory().unwrap();
    for id in 1..=RUNTIME_HOST_OBSERVATION_STORED_LIMIT + 1 {
        ledger
            .record_runtime_host_observation(observation(
                id,
                RuntimeHostObservationOutcome::Completed,
            ))
            .unwrap();
    }
    let mut oldest = query();
    oldest.until_ms = 1;
    assert_eq!(
        ledger
            .runtime_host_observation_summary(oldest)
            .unwrap()
            .observed_count,
        0
    );
    let mut newest = query();
    newest.since_ms = i64::from(RUNTIME_HOST_OBSERVATION_STORED_LIMIT);
    newest.until_ms = newest.since_ms + 1;
    assert_eq!(
        ledger
            .runtime_host_observation_summary(newest)
            .unwrap()
            .observed_count,
        2
    );
}

#[test]
fn v26_additive_migration_preserves_existing_data_and_reopens_observations() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ledger.sqlite3");
    drop(SqliteDiagnosticsLedger::open(&path).unwrap());
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(
        "DROP TABLE runtime_host_request_observations;
         UPDATE ledger_schema_migrations SET version = 26, checksum = 'pantograph-diagnostics-ledger-v26';
         CREATE TABLE migration_preservation_fixture (value TEXT NOT NULL);
         INSERT INTO migration_preservation_fixture VALUES ('keep');",
    ).unwrap();
    drop(conn);
    let mut ledger = SqliteDiagnosticsLedger::open(&path).unwrap();
    ledger
        .record_runtime_host_observation(observation(10, RuntimeHostObservationOutcome::Completed))
        .unwrap();
    drop(ledger);
    let ledger = SqliteDiagnosticsLedger::open(&path).unwrap();
    assert_eq!(
        ledger
            .runtime_host_observation_summary(query())
            .unwrap()
            .completed_count,
        1
    );
    let conn = rusqlite::Connection::open(path).unwrap();
    let preserved: String = conn
        .query_row(
            "SELECT value FROM migration_preservation_fixture",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(preserved, "keep");
    let version: i64 = conn
        .query_row(
            "SELECT max(version) FROM ledger_schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, 27);
}

#[test]
fn concurrent_identical_retry_waits_for_the_original_writer_and_is_idempotent() {
    use std::{cell::RefCell, sync::mpsc, time::Duration};

    const WAIT_TIMEOUT: Duration = Duration::from_secs(10);
    thread_local! {
        static WRITER_RELEASE: RefCell<Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>> =
            const { RefCell::new(None) };
    }

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("identical-retry.sqlite3");
    drop(SqliteDiagnosticsLedger::open(&path).unwrap());
    let original = observation(10, RuntimeHostObservationOutcome::Completed);
    let retry_conn = rusqlite::Connection::open(&path).unwrap();
    retry_conn
        .busy_handler(Some(|_| {
            WRITER_RELEASE.with(|release| {
                let Some((blocked, committed)) = release.borrow_mut().take() else {
                    return false;
                };
                blocked.send(()).is_ok() && committed.recv_timeout(WAIT_TIMEOUT).is_ok()
            })
        }))
        .unwrap();
    let mut retry_ledger = SqliteDiagnosticsLedger::from_connection(retry_conn).unwrap();
    let mut writer = rusqlite::Connection::open(&path).unwrap();
    let tx = writer
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    tx.execute(
        "INSERT INTO runtime_host_request_observations
            (observation_id, host_epoch, request_fingerprint, recorded_at_ms, payload_json)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            original.observation_id,
            original.profile.host_epoch,
            original.profile.request_fingerprint,
            original.recorded_at_ms,
            serde_json::to_string(&original).unwrap(),
        ],
    )
    .unwrap();

    let (blocked_tx, blocked_rx) = mpsc::channel();
    let (committed_tx, committed_rx) = mpsc::channel();
    let retry = std::thread::spawn(move || {
        WRITER_RELEASE.with(|release| {
            *release.borrow_mut() = Some((blocked_tx, committed_rx));
        });
        retry_ledger.record_runtime_host_observation(original)
    });

    // Wait for actual lock contention, not a timing-dependent thread interleaving.
    // A deferred transaction instead reads the absent row and fails to upgrade.
    let blocked = blocked_rx.recv_timeout(WAIT_TIMEOUT);
    tx.commit().unwrap();
    let _ = committed_tx.send(());
    let result = retry.join().unwrap();
    assert!(
        blocked.is_ok(),
        "the retry must wait for the original writer"
    );
    result.unwrap();

    let ledger = SqliteDiagnosticsLedger::open(path).unwrap();
    let summary = ledger.runtime_host_observation_summary(query()).unwrap();
    assert_eq!(summary.observed_count, 1);
    assert_eq!(summary.completed_count, 1);
    assert_eq!(summary.median_completed_host_elapsed_ms, Some(10));
}

#[test]
fn concurrent_writers_cannot_overwrite_one_observation_identity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("concurrent.sqlite3");
    drop(SqliteDiagnosticsLedger::open(&path).unwrap());
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let mut handles = Vec::new();
    for elapsed in [10, 99] {
        let path = path.clone();
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            let mut ledger = SqliteDiagnosticsLedger::open(path).unwrap();
            let mut attempt = observation(10, RuntimeHostObservationOutcome::Completed);
            attempt.host_elapsed_ms = elapsed;
            barrier.wait();
            ledger.record_runtime_host_observation(attempt)
        }));
    }
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(matches!(
        results.into_iter().find_map(Result::err),
        Some(DiagnosticsLedgerError::InvalidField {
            field: "observation_id_conflict"
        })
    ));
    let ledger = SqliteDiagnosticsLedger::open(path).unwrap();
    let summary = ledger.runtime_host_observation_summary(query()).unwrap();
    assert_eq!(summary.completed_count, 1);
    assert!(matches!(
        summary.median_completed_host_elapsed_ms,
        Some(10 | 99)
    ));
}
