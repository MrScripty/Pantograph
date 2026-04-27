use rusqlite::{params, Connection};

use crate::ids::DEFAULT_BUCKET_NAME;
use crate::sqlite_rows::fetch_session;
use crate::{
    AttributionError, AttributionRepository, BucketCreateRequest, BucketDeleteRequest, BucketId,
    BucketSelection, ClientRegistrationRequest, ClientRegistrationResponse,
    ClientSessionLifecycleState, ClientSessionOpenRequest, ClientSessionResumeRequest,
    CredentialProofRequest, CredentialSecret, SqliteAttributionStore, WorkflowId,
    WorkflowPresentationRevisionResolveRequest, WorkflowRunId, WorkflowRunSnapshotRequest,
    WorkflowRunStartRequest, WorkflowVersionResolveRequest,
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

fn workflow_version_request(
    semantic_version: &str,
    execution_fingerprint: &str,
) -> WorkflowVersionResolveRequest {
    WorkflowVersionResolveRequest {
        workflow_id: workflow_id(),
        semantic_version: semantic_version.to_string(),
        execution_fingerprint: execution_fingerprint.to_string(),
        executable_topology_json: serde_json::json!({
            "schema_version": 1,
            "nodes": [],
            "edges": []
        })
        .to_string(),
    }
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
fn workflow_version_resolution_creates_and_reuses_existing_fingerprint() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    let first = store
        .resolve_workflow_version(workflow_version_request(
            "1.0.0",
            "workflow-exec-blake3:abc",
        ))
        .expect("resolve version");
    let second = store
        .resolve_workflow_version(workflow_version_request(
            "1.0.0",
            "workflow-exec-blake3:abc",
        ))
        .expect("reuse version");

    assert_eq!(first.workflow_version_id, second.workflow_version_id);
    assert_eq!(first.semantic_version, "1.0.0");
    assert_eq!(first.execution_fingerprint, "workflow-exec-blake3:abc");
}

#[test]
fn workflow_version_resolution_rejects_semantic_version_conflicts() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    store
        .resolve_workflow_version(workflow_version_request(
            "1.0.0",
            "workflow-exec-blake3:abc",
        ))
        .expect("resolve version");

    let err = store
        .resolve_workflow_version(workflow_version_request(
            "1.0.0",
            "workflow-exec-blake3:def",
        ))
        .expect_err("semantic conflict");

    assert!(matches!(
        err,
        AttributionError::WorkflowSemanticVersionConflict { .. }
    ));
}

#[test]
fn workflow_version_resolution_rejects_fingerprint_version_conflicts() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    store
        .resolve_workflow_version(workflow_version_request(
            "1.0.0",
            "workflow-exec-blake3:abc",
        ))
        .expect("resolve version");

    let err = store
        .resolve_workflow_version(workflow_version_request(
            "1.0.1",
            "workflow-exec-blake3:abc",
        ))
        .expect_err("fingerprint conflict");

    assert!(matches!(
        err,
        AttributionError::WorkflowFingerprintVersionConflict { .. }
    ));
}

#[test]
fn workflow_version_resolution_rejects_invalid_semantic_versions() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    let err = store
        .resolve_workflow_version(workflow_version_request("1", "workflow-exec-blake3:abc"))
        .expect_err("invalid semantic version");

    assert!(matches!(
        err,
        AttributionError::InvalidWorkflowSemanticVersion { .. }
    ));
}

#[test]
fn workflow_presentation_revision_resolution_reuses_same_display_metadata() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    let version = store
        .resolve_workflow_version(workflow_version_request(
            "1.0.0",
            "workflow-exec-blake3:abc",
        ))
        .expect("resolve version");
    let presentation_json = serde_json::json!({
        "schema_version": 1,
        "nodes": [{"node_id": "node-a", "position": {"x": 0.0, "y": 1.0}}],
        "edges": []
    })
    .to_string();

    let first = store
        .resolve_workflow_presentation_revision(WorkflowPresentationRevisionResolveRequest {
            workflow_id: workflow_id(),
            workflow_version_id: version.workflow_version_id.clone(),
            presentation_fingerprint: "workflow-presentation-blake3:abc".to_string(),
            presentation_metadata_json: presentation_json.clone(),
        })
        .expect("resolve presentation revision");
    let second = store
        .resolve_workflow_presentation_revision(WorkflowPresentationRevisionResolveRequest {
            workflow_id: workflow_id(),
            workflow_version_id: version.workflow_version_id.clone(),
            presentation_fingerprint: "workflow-presentation-blake3:abc".to_string(),
            presentation_metadata_json: presentation_json,
        })
        .expect("reuse presentation revision");

    assert_eq!(
        first.workflow_presentation_revision_id,
        second.workflow_presentation_revision_id
    );
    assert_eq!(first.workflow_version_id, version.workflow_version_id);
    assert_eq!(
        first.presentation_fingerprint,
        "workflow-presentation-blake3:abc"
    );
}

#[test]
fn workflow_presentation_revision_rejects_fingerprint_metadata_conflict() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    let version = store
        .resolve_workflow_version(workflow_version_request(
            "1.0.0",
            "workflow-exec-blake3:abc",
        ))
        .expect("resolve version");
    store
        .resolve_workflow_presentation_revision(WorkflowPresentationRevisionResolveRequest {
            workflow_id: workflow_id(),
            workflow_version_id: version.workflow_version_id.clone(),
            presentation_fingerprint: "workflow-presentation-blake3:abc".to_string(),
            presentation_metadata_json: serde_json::json!({"schema_version": 1}).to_string(),
        })
        .expect("resolve presentation revision");

    let err = store
        .resolve_workflow_presentation_revision(WorkflowPresentationRevisionResolveRequest {
            workflow_id: workflow_id(),
            workflow_version_id: version.workflow_version_id,
            presentation_fingerprint: "workflow-presentation-blake3:abc".to_string(),
            presentation_metadata_json: serde_json::json!({"schema_version": 1, "changed": true})
                .to_string(),
        })
        .expect_err("fingerprint metadata conflict");

    assert!(matches!(
        err,
        AttributionError::WorkflowPresentationRevisionConflict { .. }
    ));
}

#[test]
fn workflow_run_snapshot_records_immutable_version_and_queue_context() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    let version = store
        .resolve_workflow_version(workflow_version_request(
            "1.0.0",
            "workflow-exec-blake3:abc",
        ))
        .expect("resolve version");
    let run_id = WorkflowRunId::generate();

    let snapshot = store
        .create_workflow_run_snapshot(WorkflowRunSnapshotRequest {
            workflow_run_id: run_id.clone(),
            workflow_id: workflow_id(),
            workflow_version_id: version.workflow_version_id.clone(),
            workflow_semantic_version: version.semantic_version.clone(),
            workflow_execution_fingerprint: version.execution_fingerprint.clone(),
            workflow_execution_session_id: "session-1".to_string(),
            workflow_execution_session_kind: "workflow".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
            retention_policy: "keep_alive".to_string(),
            scheduler_policy: "priority_then_fifo".to_string(),
            priority: 5,
            timeout_ms: Some(1000),
            inputs_json: serde_json::json!([{"node_id": "input"}]).to_string(),
            output_targets_json: None,
            override_selection_json: Some(serde_json::json!({"runtime_id": "local"}).to_string()),
        })
        .expect("create snapshot");

    assert_eq!(snapshot.workflow_run_id, run_id);
    assert_eq!(snapshot.workflow_version_id, version.workflow_version_id);
    assert_eq!(snapshot.workflow_execution_session_kind, "workflow");
    assert_eq!(snapshot.usage_profile.as_deref(), Some("interactive"));
    assert!(snapshot.keep_alive);
    assert_eq!(snapshot.retention_policy, "keep_alive");
    assert_eq!(snapshot.scheduler_policy, "priority_then_fifo");
    assert_eq!(snapshot.priority, 5);
    assert_eq!(snapshot.timeout_ms, Some(1000));

    let projection = store
        .workflow_run_version_projection(&run_id)
        .expect("query run version projection")
        .expect("projection");
    assert_eq!(projection.snapshot.workflow_run_id, run_id);
    assert_eq!(
        projection.workflow_version.workflow_version_id,
        version.workflow_version_id
    );
    assert_eq!(
        projection.workflow_version.executable_topology_json,
        version.executable_topology_json
    );
}

#[test]
fn workflow_run_snapshot_rejects_mismatched_version_facts() {
    let mut store = SqliteAttributionStore::open_in_memory().expect("store");
    let version = store
        .resolve_workflow_version(workflow_version_request(
            "1.0.0",
            "workflow-exec-blake3:abc",
        ))
        .expect("resolve version");

    let err = store
        .create_workflow_run_snapshot(WorkflowRunSnapshotRequest {
            workflow_run_id: WorkflowRunId::generate(),
            workflow_id: workflow_id(),
            workflow_version_id: version.workflow_version_id,
            workflow_semantic_version: "1.0.0".to_string(),
            workflow_execution_fingerprint: "workflow-exec-blake3:def".to_string(),
            workflow_execution_session_id: "session-1".to_string(),
            workflow_execution_session_kind: "workflow".to_string(),
            usage_profile: None,
            keep_alive: false,
            retention_policy: "ephemeral".to_string(),
            scheduler_policy: "priority_then_fifo".to_string(),
            priority: 0,
            timeout_ms: None,
            inputs_json: "[]".to_string(),
            output_targets_json: None,
            override_selection_json: None,
        })
        .expect_err("mismatched version facts");

    assert!(matches!(
        err,
        AttributionError::WorkflowFingerprintVersionConflict { .. }
    ));
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
fn attribution_boundary_json_round_trips_without_debug_secret_leak() {
    let registration = ClientRegistrationResponse {
        client: crate::records::ClientRecord {
            client_id: crate::ClientId::try_from("client_boundary".to_string()).expect("client id"),
            display_name: Some("boundary client".to_string()),
            metadata_json: None,
            status: crate::ClientStatus::Active,
            created_at_ms: 1,
        },
        credential: crate::ClientCredential {
            client_credential_id: crate::ClientCredentialId::try_from("cred_boundary".to_string())
                .expect("credential id"),
            client_id: crate::ClientId::try_from("client_boundary".to_string()).expect("client id"),
            status: crate::ClientCredentialStatus::Active,
            created_at_ms: 1,
            revoked_at_ms: None,
        },
        credential_secret: CredentialSecret::from_boundary_secret("boundary-secret")
            .expect("secret"),
    };

    let serialized = serde_json::to_value(&registration).expect("serialize registration");
    assert_eq!(serialized["credential_secret"], "boundary-secret");
    assert_eq!(
        format!("{:?}", registration.credential_secret),
        "CredentialSecret(<redacted>)"
    );

    let proof: CredentialProofRequest = serde_json::from_value(serde_json::json!({
        "credential_id": "cred_boundary",
        "secret": "boundary-secret"
    }))
    .expect("deserialize proof");
    assert_eq!(proof.secret.expose_secret(), "boundary-secret");

    let explicit: BucketSelection = serde_json::from_value(serde_json::json!({
        "type": "explicit",
        "bucket_id": "bucket_boundary"
    }))
    .expect("deserialize explicit bucket");
    assert!(matches!(explicit, BucketSelection::Explicit(_)));

    let default = serde_json::to_value(BucketSelection::Default).expect("serialize default");
    assert_eq!(default, serde_json::json!({ "type": "default" }));
}

#[test]
fn attribution_boundary_json_rejects_malformed_secret() {
    let err = serde_json::from_value::<CredentialProofRequest>(serde_json::json!({
        "credential_id": "cred_boundary",
        "secret": "bad\nsecret"
    }))
    .expect_err("control characters rejected");

    assert!(err.to_string().contains("credential.secret"));
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
