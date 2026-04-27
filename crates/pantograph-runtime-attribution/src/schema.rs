use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::AttributionError;

pub(crate) const SCHEMA_VERSION: i64 = 5;

pub(crate) fn apply_schema(tx: &Transaction<'_>) -> Result<(), AttributionError> {
    tx.execute_batch(
        r#"
        CREATE TABLE attribution_schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_ms INTEGER NOT NULL
        );

        CREATE TABLE clients (
            client_id TEXT PRIMARY KEY,
            display_name TEXT,
            metadata_json TEXT,
            status TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL
        );

        CREATE TABLE client_credentials (
            client_credential_id TEXT PRIMARY KEY,
            client_id TEXT NOT NULL REFERENCES clients(client_id),
            salt BLOB NOT NULL,
            digest BLOB NOT NULL,
            status TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL,
            revoked_at_ms INTEGER
        );
        CREATE INDEX idx_client_credentials_client ON client_credentials(client_id);

        CREATE TABLE client_sessions (
            client_session_id TEXT PRIMARY KEY,
            client_id TEXT NOT NULL REFERENCES clients(client_id),
            opened_at_ms INTEGER NOT NULL,
            latest_lifecycle_state TEXT NOT NULL,
            grace_deadline_ms INTEGER,
            superseded_by_session_id TEXT
                REFERENCES client_sessions(client_session_id)
        );
        CREATE UNIQUE INDEX idx_client_sessions_one_active
            ON client_sessions(client_id)
            WHERE latest_lifecycle_state IN ('opening', 'connected', 'disconnected_grace');
        CREATE INDEX idx_client_sessions_client ON client_sessions(client_id);

        CREATE TABLE session_lifecycle_records (
            event_id TEXT PRIMARY KEY,
            client_session_id TEXT NOT NULL REFERENCES client_sessions(client_session_id),
            lifecycle_state TEXT NOT NULL,
            occurred_at_ms INTEGER NOT NULL,
            reason TEXT,
            related_session_id TEXT REFERENCES client_sessions(client_session_id)
        );
        CREATE INDEX idx_session_lifecycle_session
            ON session_lifecycle_records(client_session_id, occurred_at_ms);

        CREATE TABLE buckets (
            bucket_id TEXT PRIMARY KEY,
            client_id TEXT NOT NULL REFERENCES clients(client_id),
            name TEXT NOT NULL,
            metadata_json TEXT,
            created_at_ms INTEGER NOT NULL,
            deleted_at_ms INTEGER,
            deletion_reason TEXT
        );
        CREATE UNIQUE INDEX idx_buckets_active_name
            ON buckets(client_id, name)
            WHERE deleted_at_ms IS NULL;
        CREATE INDEX idx_buckets_client ON buckets(client_id);

        CREATE TABLE default_bucket_assignments (
            client_session_id TEXT PRIMARY KEY REFERENCES client_sessions(client_session_id),
            bucket_id TEXT NOT NULL REFERENCES buckets(bucket_id),
            assigned_at_ms INTEGER NOT NULL
        );
        CREATE INDEX idx_default_bucket_assignments_bucket
            ON default_bucket_assignments(bucket_id);

        CREATE TABLE workflow_runs (
            workflow_run_id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            client_id TEXT NOT NULL REFERENCES clients(client_id),
            client_session_id TEXT NOT NULL REFERENCES client_sessions(client_session_id),
            bucket_id TEXT NOT NULL REFERENCES buckets(bucket_id),
            status TEXT NOT NULL,
            started_at_ms INTEGER NOT NULL,
            completed_at_ms INTEGER
        );
        CREATE INDEX idx_workflow_runs_client ON workflow_runs(client_id);
        CREATE INDEX idx_workflow_runs_session ON workflow_runs(client_session_id);
        CREATE INDEX idx_workflow_runs_bucket ON workflow_runs(bucket_id);
        CREATE INDEX idx_workflow_runs_workflow ON workflow_runs(workflow_id);

        CREATE TABLE workflow_versions (
            workflow_version_id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            semantic_version TEXT NOT NULL,
            execution_fingerprint TEXT NOT NULL,
            executable_topology_json TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL,
            UNIQUE(workflow_id, semantic_version),
            UNIQUE(workflow_id, execution_fingerprint)
        );
        CREATE INDEX idx_workflow_versions_workflow
            ON workflow_versions(workflow_id, created_at_ms);

        CREATE TABLE workflow_presentation_revisions (
            workflow_presentation_revision_id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            workflow_version_id TEXT NOT NULL REFERENCES workflow_versions(workflow_version_id),
            presentation_fingerprint TEXT NOT NULL,
            presentation_metadata_json TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL,
            UNIQUE(workflow_version_id, presentation_fingerprint)
        );
        CREATE INDEX idx_workflow_presentation_revisions_workflow
            ON workflow_presentation_revisions(workflow_id, created_at_ms);
        CREATE INDEX idx_workflow_presentation_revisions_version
            ON workflow_presentation_revisions(workflow_version_id, created_at_ms);

        CREATE TABLE workflow_run_snapshots (
            workflow_run_snapshot_id TEXT PRIMARY KEY,
            workflow_run_id TEXT NOT NULL UNIQUE,
            workflow_id TEXT NOT NULL,
            workflow_version_id TEXT NOT NULL REFERENCES workflow_versions(workflow_version_id),
            workflow_presentation_revision_id TEXT NOT NULL REFERENCES workflow_presentation_revisions(workflow_presentation_revision_id),
            workflow_semantic_version TEXT NOT NULL,
            workflow_execution_fingerprint TEXT NOT NULL,
            workflow_execution_session_id TEXT NOT NULL,
            workflow_execution_session_kind TEXT NOT NULL,
            usage_profile TEXT,
            keep_alive INTEGER NOT NULL,
            retention_policy TEXT NOT NULL,
            scheduler_policy TEXT NOT NULL,
            priority INTEGER NOT NULL,
            timeout_ms INTEGER,
            inputs_json TEXT NOT NULL,
            output_targets_json TEXT,
            override_selection_json TEXT,
            created_at_ms INTEGER NOT NULL
        );
        CREATE INDEX idx_workflow_run_snapshots_workflow_version
            ON workflow_run_snapshots(workflow_version_id, created_at_ms);
        CREATE INDEX idx_workflow_run_snapshots_presentation_revision
            ON workflow_run_snapshots(workflow_presentation_revision_id, created_at_ms);
        "#,
    )?;
    tx.execute(
        "INSERT INTO attribution_schema_migrations (version, applied_at_ms) VALUES (?1, ?2)",
        params![SCHEMA_VERSION, Utc::now().timestamp_millis()],
    )?;
    Ok(())
}

pub(crate) fn current_schema_version(conn: &Connection) -> Result<Option<i64>, AttributionError> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'attribution_schema_migrations'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        return Ok(None);
    }
    let version = conn.query_row(
        "SELECT MAX(version) FROM attribution_schema_migrations",
        [],
        |row| row.get(0),
    )?;
    Ok(Some(version))
}
