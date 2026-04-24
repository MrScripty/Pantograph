use rusqlite::{params, Connection};

use crate::ids::DEFAULT_BUCKET_NAME;
use crate::sqlite_rows::fetch_session;
use crate::{
    AttributionError, AttributionRepository, BucketCreateRequest, BucketDeleteRequest, BucketId,
    BucketSelection, ClientRegistrationRequest, ClientRegistrationResponse,
    ClientSessionLifecycleState, ClientSessionOpenRequest, ClientSessionResumeRequest,
    CredentialProofRequest, CredentialSecret, SqliteAttributionStore, WorkflowId,
    WorkflowRunStartRequest,
};

fn register(store: &mut SqliteAttributionStore) -> ClientRegistrationResponse {
    store
        .register_client(ClientRegistrationRequest {
            display_name: Some("local gui".to_string()),
            metadata_json: None,
        })
        .expect("register client")
}

fn workflow_id() -> WorkflowId {
    WorkflowId::try_from("workflow-alpha".to_string()).expect("valid workflow id")
}

#[test]
fn registered_client_opens_and_resumes_session_with_default_bucket() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    let registered = register(&mut store);

    let opened = store
        .open_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: Some("launch".to_string()),
        })
        .expect("open session");

    assert_eq!(opened.session.client_id, registered.client.client_id);
    assert_eq!(opened.default_bucket.name, DEFAULT_BUCKET_NAME);
    assert_eq!(
        opened.default_bucket_assignment.client_session_id,
        opened.session.client_session_id
    );

    let resumed = store
        .resume_session(ClientSessionResumeRequest {
            credential: registered.credential_proof_request(),
            client_session_id: opened.session.client_session_id.clone(),
            reason: Some("reconnect".to_string()),
        })
        .expect("resume connected session");
    assert_eq!(
        resumed.latest_lifecycle_state,
        ClientSessionLifecycleState::Connected
    );
}

#[test]
fn second_active_session_is_rejected_until_takeover() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    let registered = register(&mut store);
    let first = store
        .open_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: None,
        })
        .expect("open first session");

    let duplicate = store
        .open_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: None,
        })
        .expect_err("duplicate active session rejected");
    assert!(matches!(
        duplicate,
        AttributionError::DuplicateActiveSession { .. }
    ));

    let takeover = store
        .open_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: true,
            reason: Some("explicit takeover".to_string()),
        })
        .expect("takeover session");
    assert_eq!(
        takeover.superseded_session_id,
        Some(first.session.client_session_id.clone())
    );
    let prior = fetch_session(&store.conn, &first.session.client_session_id)
        .expect("prior session persisted");
    assert_eq!(
        prior.latest_lifecycle_state,
        ClientSessionLifecycleState::TakenOver
    );
    assert_eq!(
        prior.superseded_by_session_id,
        Some(takeover.session.client_session_id)
    );
}

#[test]
fn workflow_run_uses_default_bucket_and_survives_reopen() {
    let temp = tempfile::NamedTempFile::new().expect("temp db");
    let path = temp.path().to_path_buf();
    let run_id = {
        let mut store = SqliteAttributionStore::open(&path).expect("store");
        let registered = register(&mut store);
        let opened = store
            .open_session(ClientSessionOpenRequest {
                credential: registered.credential_proof_request(),
                takeover: false,
                reason: None,
            })
            .expect("open session");
        let run = store
            .start_workflow_run(WorkflowRunStartRequest {
                credential: registered.credential_proof_request(),
                client_session_id: opened.session.client_session_id,
                workflow_id: workflow_id(),
                bucket_selection: BucketSelection::Default,
            })
            .expect("start run");
        assert_eq!(run.bucket_id, opened.default_bucket.bucket_id);
        run.workflow_run_id
    };

    let conn = Connection::open(&path).expect("reopen db");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workflow_runs WHERE workflow_run_id = ?1",
            params![run_id.as_str()],
            |row| row.get(0),
        )
        .expect("query run");
    assert_eq!(count, 1);
}

#[test]
fn explicit_bucket_must_belong_to_session_client() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    let first = register(&mut store);
    let second = register(&mut store);
    let first_session = store
        .open_session(ClientSessionOpenRequest {
            credential: first.credential_proof_request(),
            takeover: false,
            reason: None,
        })
        .expect("first session");
    let second_bucket = store
        .create_bucket(BucketCreateRequest {
            credential: second.credential_proof_request(),
            name: "other-client-bucket".to_string(),
            metadata_json: None,
        })
        .expect("second bucket");

    let err = store
        .start_workflow_run(WorkflowRunStartRequest {
            credential: first.credential_proof_request(),
            client_session_id: first_session.session.client_session_id,
            workflow_id: workflow_id(),
            bucket_selection: BucketSelection::Explicit(second_bucket.bucket_id),
        })
        .expect_err("cross-client bucket rejected");
    assert!(matches!(err, AttributionError::BucketClientMismatch));
}

#[test]
fn bucket_name_rules_and_delete_protections_are_enforced() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    let registered = register(&mut store);
    let opened = store
        .open_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: None,
        })
        .expect("open session");

    let default_delete = store
        .delete_bucket(BucketDeleteRequest {
            credential: registered.credential_proof_request(),
            bucket_id: opened.default_bucket.bucket_id,
            reason: Some("cleanup".to_string()),
        })
        .expect_err("default bucket protected");
    assert!(matches!(
        default_delete,
        AttributionError::DefaultBucketProtected
    ));

    let bucket = store
        .create_bucket(BucketCreateRequest {
            credential: registered.credential_proof_request(),
            name: "analysis".to_string(),
            metadata_json: None,
        })
        .expect("create bucket");
    let duplicate = store
        .create_bucket(BucketCreateRequest {
            credential: registered.credential_proof_request(),
            name: "analysis".to_string(),
            metadata_json: None,
        })
        .expect_err("duplicate bucket rejected");
    assert!(matches!(
        duplicate,
        AttributionError::BucketNameCollision { .. }
    ));

    let deleted = store
        .delete_bucket(BucketDeleteRequest {
            credential: registered.credential_proof_request(),
            bucket_id: bucket.bucket_id,
            reason: Some("done".to_string()),
        })
        .expect("delete non-default bucket");
    assert!(deleted.deleted_at_ms.is_some());
}

#[test]
fn credential_records_do_not_persist_raw_secret() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    let registered = register(&mut store);
    let secret = registered.credential_secret.expose_secret().to_string();

    let stored_secret_count: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM client_credentials
             WHERE CAST(salt AS TEXT) = ?1 OR CAST(digest AS TEXT) = ?1",
            params![secret],
            |row| row.get(0),
        )
        .expect("query credential rows");
    assert_eq!(stored_secret_count, 0);

    let bad = CredentialProofRequest {
        credential_id: registered.credential.client_credential_id,
        secret: CredentialSecret::from_raw_for_test("wrong"),
    };
    let err = store.verify_credential(&bad).expect_err("bad proof");
    assert!(matches!(err, AttributionError::CredentialMismatch));
}

#[test]
fn unsupported_schema_version_fails_closed() {
    let conn = Connection::open_in_memory().expect("conn");
    conn.execute(
        "CREATE TABLE attribution_schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_ms INTEGER NOT NULL
         )",
        [],
    )
    .expect("create schema table");
    conn.execute(
        "INSERT INTO attribution_schema_migrations (version, applied_at_ms)
         VALUES (999, 1)",
        [],
    )
    .expect("insert future version");

    let mut store = SqliteAttributionStore::from_existing_without_migration(conn);
    let err = store.initialize().expect_err("future version rejected");
    assert!(matches!(
        err,
        AttributionError::UnsupportedSchemaVersion { found: 999 }
    ));
}

#[test]
fn generated_ids_validate_as_non_empty_boundary_values() {
    let bucket_id = BucketId::generate();
    let reparsed = BucketId::try_from(bucket_id.as_str().to_string()).expect("reparse id");
    assert_eq!(bucket_id, reparsed);
}
