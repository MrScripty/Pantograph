use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::records::{RetentionClass, DEFAULT_STANDARD_RETENTION_DAYS};
use crate::util::now_ms;
use crate::DiagnosticsLedgerError;

pub(crate) const SCHEMA_VERSION: i64 = 12;
const SCHEMA_CHECKSUM: &str = "pantograph-diagnostics-ledger-v12";

pub(crate) fn apply_schema(tx: &Transaction<'_>) -> Result<(), DiagnosticsLedgerError> {
    tx.execute_batch(
        r#"
        CREATE TABLE ledger_schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_ms INTEGER NOT NULL,
            checksum TEXT NOT NULL
        );

        CREATE TABLE model_license_usage_events (
            usage_event_id TEXT PRIMARY KEY,
            client_id TEXT NOT NULL,
            client_session_id TEXT NOT NULL,
            bucket_id TEXT NOT NULL,
            workflow_run_id TEXT NOT NULL,
            workflow_id TEXT NOT NULL,
            workflow_version_id TEXT,
            workflow_semantic_version TEXT,
            node_id TEXT NOT NULL,
            node_type TEXT NOT NULL,
            model_id TEXT NOT NULL,
            model_revision TEXT,
            model_hash TEXT,
            model_modality TEXT,
            runtime_backend TEXT,
            guarantee_level TEXT NOT NULL,
            status TEXT NOT NULL,
            started_at_ms INTEGER NOT NULL,
            completed_at_ms INTEGER,
            retention_class TEXT NOT NULL,
            schema_version INTEGER NOT NULL,
            correlation_id TEXT
        );
        CREATE INDEX idx_usage_events_client_time
            ON model_license_usage_events(client_id, started_at_ms);
        CREATE INDEX idx_usage_events_session_time
            ON model_license_usage_events(client_session_id, started_at_ms);
        CREATE INDEX idx_usage_events_bucket_time
            ON model_license_usage_events(bucket_id, started_at_ms);
        CREATE INDEX idx_usage_events_workflow_time
            ON model_license_usage_events(workflow_id, started_at_ms);
        CREATE INDEX idx_usage_events_workflow_version_time
            ON model_license_usage_events(workflow_version_id, started_at_ms);
        CREATE INDEX idx_usage_events_run_node
            ON model_license_usage_events(workflow_run_id, node_id);
        CREATE INDEX idx_usage_events_model_time
            ON model_license_usage_events(model_id, started_at_ms);
        CREATE INDEX idx_usage_events_guarantee_time
            ON model_license_usage_events(guarantee_level, started_at_ms);
        CREATE INDEX idx_usage_events_retention
            ON model_license_usage_events(retention_class, completed_at_ms, started_at_ms);

        CREATE TABLE license_snapshots (
            usage_event_id TEXT PRIMARY KEY
                REFERENCES model_license_usage_events(usage_event_id) ON DELETE CASCADE,
            license_value TEXT,
            source_metadata_json TEXT,
            model_metadata_snapshot_json TEXT,
            unavailable_reason TEXT
        );
        CREATE INDEX idx_license_snapshots_value
            ON license_snapshots(license_value);

        CREATE TABLE model_output_measurements (
            usage_event_id TEXT PRIMARY KEY
                REFERENCES model_license_usage_events(usage_event_id) ON DELETE CASCADE,
            modality TEXT NOT NULL,
            item_count INTEGER,
            character_count INTEGER,
            byte_size INTEGER,
            token_count INTEGER,
            width INTEGER,
            height INTEGER,
            pixel_count INTEGER,
            duration_ms INTEGER,
            sample_rate_hz INTEGER,
            channels INTEGER,
            frame_count INTEGER,
            vector_count INTEGER,
            dimensions INTEGER,
            numeric_representation TEXT,
            top_level_shape TEXT,
            schema_id TEXT,
            schema_digest TEXT,
            unavailable_reasons_json TEXT NOT NULL
        );

        CREATE TABLE usage_lineage (
            usage_event_id TEXT PRIMARY KEY
                REFERENCES model_license_usage_events(usage_event_id) ON DELETE CASCADE,
            node_id TEXT NOT NULL,
            node_type TEXT NOT NULL,
            port_ids_json TEXT NOT NULL,
            composed_parent_chain_json TEXT NOT NULL,
            effective_contract_version TEXT,
            effective_contract_digest TEXT,
            metadata_json TEXT
        );
        CREATE INDEX idx_usage_lineage_contract_version
            ON usage_lineage(effective_contract_version);
        CREATE INDEX idx_usage_lineage_contract_digest
            ON usage_lineage(effective_contract_digest);

        CREATE TABLE diagnostics_retention_policy (
            policy_id TEXT PRIMARY KEY,
            retention_class TEXT NOT NULL UNIQUE,
            retention_days INTEGER NOT NULL,
            applied_at_ms INTEGER NOT NULL,
            explanation TEXT NOT NULL
        );
        "#,
    )?;
    apply_timing_schema(tx)?;
    apply_workflow_run_summary_schema(tx)?;
    apply_event_ledger_schema(tx)?;
    tx.execute(
        "INSERT INTO ledger_schema_migrations (version, applied_at_ms, checksum)
         VALUES (?1, ?2, ?3)",
        params![SCHEMA_VERSION, now_ms(), SCHEMA_CHECKSUM],
    )?;
    tx.execute(
        "INSERT INTO diagnostics_retention_policy
            (policy_id, retention_class, retention_days, applied_at_ms, explanation)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            "standard-local-v1",
            RetentionClass::Standard.as_db(),
            DEFAULT_STANDARD_RETENTION_DAYS,
            now_ms(),
            "Default local model/license usage retention policy"
        ],
    )?;
    Ok(())
}

pub(crate) fn migrate_schema(
    conn: &mut Connection,
    found: i64,
) -> Result<(), DiagnosticsLedgerError> {
    if found > SCHEMA_VERSION {
        return Err(DiagnosticsLedgerError::UnsupportedSchemaVersion { found });
    }
    if found == SCHEMA_VERSION {
        return Ok(());
    }

    let tx = conn.transaction()?;
    if found < 3 {
        tx.execute("DROP TABLE IF EXISTS workflow_timing_observations", [])?;
        apply_timing_schema(&tx)?;
    }
    if found < 4 {
        apply_workflow_run_summary_schema(&tx)?;
    }
    if found < 5 && table_exists(&tx, "model_license_usage_events")? {
        tx.execute(
            "ALTER TABLE model_license_usage_events ADD COLUMN workflow_version_id TEXT",
            [],
        )?;
        tx.execute(
            "ALTER TABLE model_license_usage_events ADD COLUMN workflow_semantic_version TEXT",
            [],
        )?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_usage_events_workflow_version_time
                ON model_license_usage_events(workflow_version_id, started_at_ms)",
            [],
        )?;
    }
    if found < 6 && table_exists(&tx, "usage_lineage")? {
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_usage_lineage_contract_version
                ON usage_lineage(effective_contract_version)",
            [],
        )?;
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_usage_lineage_contract_digest
                ON usage_lineage(effective_contract_digest)",
            [],
        )?;
    }
    if found < 7 {
        apply_event_ledger_schema(&tx)?;
    }
    if found < 8 {
        apply_scheduler_timeline_projection_schema(&tx)?;
    }
    if found < 9 {
        apply_run_list_projection_schema(&tx)?;
    }
    if found < 10 {
        apply_run_detail_projection_schema(&tx)?;
    }
    if found < 11 {
        apply_io_artifact_projection_schema(&tx)?;
    }
    if found < 12 {
        apply_library_usage_projection_schema(&tx)?;
    }
    if found < SCHEMA_VERSION {
        tx.execute(
            "INSERT INTO ledger_schema_migrations (version, applied_at_ms, checksum)
             VALUES (?1, ?2, ?3)",
            params![SCHEMA_VERSION, now_ms(), SCHEMA_CHECKSUM],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn apply_event_ledger_schema(tx: &Transaction<'_>) -> Result<(), DiagnosticsLedgerError> {
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS diagnostic_events (
            event_seq INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id TEXT NOT NULL UNIQUE,
            event_kind TEXT NOT NULL,
            schema_version INTEGER NOT NULL,
            source_component TEXT NOT NULL,
            source_instance_id TEXT,
            occurred_at_ms INTEGER NOT NULL,
            recorded_at_ms INTEGER NOT NULL,
            workflow_run_id TEXT,
            workflow_id TEXT,
            workflow_version_id TEXT,
            workflow_semantic_version TEXT,
            node_id TEXT,
            node_type TEXT,
            node_version TEXT,
            runtime_id TEXT,
            runtime_version TEXT,
            model_id TEXT,
            model_version TEXT,
            client_id TEXT,
            client_session_id TEXT,
            bucket_id TEXT,
            scheduler_policy_id TEXT,
            retention_policy_id TEXT,
            privacy_class TEXT NOT NULL,
            event_retention_class TEXT NOT NULL,
            payload_hash TEXT NOT NULL,
            payload_size_bytes INTEGER NOT NULL,
            payload_ref TEXT,
            payload_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_diagnostic_events_seq_kind
            ON diagnostic_events(event_seq, event_kind);
        CREATE INDEX IF NOT EXISTS idx_diagnostic_events_kind_seq
            ON diagnostic_events(event_kind, event_seq);
        CREATE INDEX IF NOT EXISTS idx_diagnostic_events_run_seq
            ON diagnostic_events(workflow_run_id, event_seq);
        CREATE INDEX IF NOT EXISTS idx_diagnostic_events_workflow_version_seq
            ON diagnostic_events(workflow_version_id, event_seq);
        CREATE INDEX IF NOT EXISTS idx_diagnostic_events_node_seq
            ON diagnostic_events(node_id, event_seq);
        CREATE INDEX IF NOT EXISTS idx_diagnostic_events_model_seq
            ON diagnostic_events(model_id, event_seq);
        CREATE INDEX IF NOT EXISTS idx_diagnostic_events_runtime_seq
            ON diagnostic_events(runtime_id, event_seq);

        CREATE TABLE IF NOT EXISTS projection_state (
            projection_name TEXT PRIMARY KEY,
            projection_version INTEGER NOT NULL,
            last_applied_event_seq INTEGER NOT NULL,
            status TEXT NOT NULL,
            rebuilt_at_ms INTEGER,
            updated_at_ms INTEGER NOT NULL
        );
        "#,
    )?;
    apply_scheduler_timeline_projection_schema(tx)?;
    apply_run_list_projection_schema(tx)?;
    apply_run_detail_projection_schema(tx)?;
    apply_io_artifact_projection_schema(tx)?;
    apply_library_usage_projection_schema(tx)?;
    Ok(())
}

fn apply_scheduler_timeline_projection_schema(
    tx: &Transaction<'_>,
) -> Result<(), DiagnosticsLedgerError> {
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS scheduler_timeline_projection (
            event_seq INTEGER PRIMARY KEY,
            event_id TEXT NOT NULL UNIQUE,
            event_kind TEXT NOT NULL,
            source_component TEXT NOT NULL,
            occurred_at_ms INTEGER NOT NULL,
            recorded_at_ms INTEGER NOT NULL,
            workflow_run_id TEXT NOT NULL,
            workflow_id TEXT NOT NULL,
            workflow_version_id TEXT,
            workflow_semantic_version TEXT,
            scheduler_policy_id TEXT,
            retention_policy_id TEXT,
            summary TEXT NOT NULL,
            detail TEXT,
            payload_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_scheduler_timeline_run_seq
            ON scheduler_timeline_projection(workflow_run_id, event_seq);
        CREATE INDEX IF NOT EXISTS idx_scheduler_timeline_workflow_seq
            ON scheduler_timeline_projection(workflow_id, event_seq);
        CREATE INDEX IF NOT EXISTS idx_scheduler_timeline_policy_seq
            ON scheduler_timeline_projection(scheduler_policy_id, event_seq);
        "#,
    )?;
    Ok(())
}

fn apply_run_list_projection_schema(tx: &Transaction<'_>) -> Result<(), DiagnosticsLedgerError> {
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS run_list_projection (
            workflow_run_id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            workflow_version_id TEXT,
            workflow_semantic_version TEXT,
            status TEXT NOT NULL,
            accepted_at_ms INTEGER,
            enqueued_at_ms INTEGER,
            started_at_ms INTEGER,
            completed_at_ms INTEGER,
            duration_ms INTEGER,
            scheduler_policy_id TEXT,
            retention_policy_id TEXT,
            last_event_seq INTEGER NOT NULL,
            last_updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_run_list_projection_updated
            ON run_list_projection(last_updated_at_ms DESC, last_event_seq DESC);
        CREATE INDEX IF NOT EXISTS idx_run_list_projection_workflow_updated
            ON run_list_projection(workflow_id, last_updated_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_run_list_projection_status_updated
            ON run_list_projection(status, last_updated_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_run_list_projection_retention_updated
            ON run_list_projection(retention_policy_id, last_updated_at_ms DESC);
        "#,
    )?;
    Ok(())
}

fn apply_run_detail_projection_schema(tx: &Transaction<'_>) -> Result<(), DiagnosticsLedgerError> {
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS run_detail_projection (
            workflow_run_id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            workflow_version_id TEXT,
            workflow_semantic_version TEXT,
            status TEXT NOT NULL,
            accepted_at_ms INTEGER,
            enqueued_at_ms INTEGER,
            started_at_ms INTEGER,
            completed_at_ms INTEGER,
            duration_ms INTEGER,
            scheduler_policy_id TEXT,
            retention_policy_id TEXT,
            client_id TEXT,
            client_session_id TEXT,
            bucket_id TEXT,
            workflow_run_snapshot_id TEXT,
            workflow_presentation_revision_id TEXT,
            latest_estimate_json TEXT,
            latest_queue_placement_json TEXT,
            started_payload_json TEXT,
            terminal_payload_json TEXT,
            terminal_error TEXT,
            timeline_event_count INTEGER NOT NULL,
            last_event_seq INTEGER NOT NULL,
            last_updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_run_detail_projection_workflow_updated
            ON run_detail_projection(workflow_id, last_updated_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_run_detail_projection_version_updated
            ON run_detail_projection(workflow_version_id, last_updated_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_run_detail_projection_status_updated
            ON run_detail_projection(status, last_updated_at_ms DESC);
        "#,
    )?;
    Ok(())
}

fn apply_io_artifact_projection_schema(tx: &Transaction<'_>) -> Result<(), DiagnosticsLedgerError> {
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS io_artifact_projection (
            event_seq INTEGER PRIMARY KEY,
            event_id TEXT NOT NULL UNIQUE,
            occurred_at_ms INTEGER NOT NULL,
            recorded_at_ms INTEGER NOT NULL,
            workflow_run_id TEXT NOT NULL,
            workflow_id TEXT NOT NULL,
            workflow_version_id TEXT,
            workflow_semantic_version TEXT,
            node_id TEXT,
            node_type TEXT,
            node_version TEXT,
            runtime_id TEXT,
            runtime_version TEXT,
            model_id TEXT,
            model_version TEXT,
            artifact_id TEXT NOT NULL,
            artifact_role TEXT NOT NULL,
            media_type TEXT,
            size_bytes INTEGER,
            content_hash TEXT,
            payload_ref TEXT,
            retention_policy_id TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_io_artifact_projection_run_seq
            ON io_artifact_projection(workflow_run_id, event_seq);
        CREATE INDEX IF NOT EXISTS idx_io_artifact_projection_run_node_seq
            ON io_artifact_projection(workflow_run_id, node_id, event_seq);
        CREATE INDEX IF NOT EXISTS idx_io_artifact_projection_role_seq
            ON io_artifact_projection(artifact_role, event_seq);
        "#,
    )?;
    Ok(())
}

fn apply_library_usage_projection_schema(
    tx: &Transaction<'_>,
) -> Result<(), DiagnosticsLedgerError> {
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS library_usage_projection (
            asset_id TEXT PRIMARY KEY,
            total_access_count INTEGER NOT NULL,
            run_access_count INTEGER NOT NULL,
            total_network_bytes INTEGER NOT NULL,
            last_accessed_at_ms INTEGER NOT NULL,
            last_operation TEXT NOT NULL,
            last_cache_status TEXT,
            last_workflow_run_id TEXT,
            last_workflow_id TEXT,
            last_workflow_version_id TEXT,
            last_workflow_semantic_version TEXT,
            last_client_id TEXT,
            last_client_session_id TEXT,
            last_bucket_id TEXT,
            last_event_seq INTEGER NOT NULL,
            last_updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_library_usage_projection_accessed
            ON library_usage_projection(last_accessed_at_ms DESC, last_event_seq DESC);
        CREATE INDEX IF NOT EXISTS idx_library_usage_projection_workflow
            ON library_usage_projection(last_workflow_id, last_accessed_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_library_usage_projection_workflow_version
            ON library_usage_projection(last_workflow_version_id, last_accessed_at_ms DESC);

        CREATE TABLE IF NOT EXISTS library_usage_run_projection (
            asset_id TEXT NOT NULL,
            workflow_run_id TEXT NOT NULL,
            workflow_id TEXT,
            workflow_version_id TEXT,
            workflow_semantic_version TEXT,
            first_event_seq INTEGER NOT NULL,
            last_event_seq INTEGER NOT NULL,
            last_accessed_at_ms INTEGER NOT NULL,
            PRIMARY KEY(asset_id, workflow_run_id)
        );
        CREATE INDEX IF NOT EXISTS idx_library_usage_run_projection_workflow
            ON library_usage_run_projection(workflow_id, asset_id);
        CREATE INDEX IF NOT EXISTS idx_library_usage_run_projection_version
            ON library_usage_run_projection(workflow_version_id, asset_id);
        "#,
    )?;
    Ok(())
}

fn table_exists(tx: &Transaction<'_>, table_name: &str) -> Result<bool, DiagnosticsLedgerError> {
    let exists = tx.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
        )",
        params![table_name],
        |row| row.get::<_, bool>(0),
    )?;
    Ok(exists)
}

fn apply_timing_schema(tx: &Transaction<'_>) -> Result<(), DiagnosticsLedgerError> {
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS workflow_timing_observations (
            observation_key TEXT PRIMARY KEY,
            observation_scope TEXT NOT NULL,
            workflow_run_id TEXT NOT NULL,
            workflow_id TEXT NOT NULL,
            graph_fingerprint TEXT NOT NULL,
            node_id TEXT,
            node_type TEXT,
            runtime_id TEXT,
            status TEXT NOT NULL,
            started_at_ms INTEGER NOT NULL,
            ended_at_ms INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            recorded_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_workflow_timing_lookup
            ON workflow_timing_observations(
                observation_scope,
                workflow_id,
                graph_fingerprint,
                node_id,
                node_type,
                runtime_id,
                status,
                recorded_at_ms
            );
        CREATE INDEX IF NOT EXISTS idx_workflow_timing_retention
            ON workflow_timing_observations(recorded_at_ms);
        "#,
    )?;
    Ok(())
}

fn apply_workflow_run_summary_schema(tx: &Transaction<'_>) -> Result<(), DiagnosticsLedgerError> {
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS workflow_run_summaries (
            workflow_run_id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            session_id TEXT,
            graph_fingerprint TEXT,
            status TEXT NOT NULL,
            started_at_ms INTEGER NOT NULL,
            ended_at_ms INTEGER,
            duration_ms INTEGER,
            node_count_at_start INTEGER NOT NULL,
            event_count INTEGER NOT NULL,
            last_error TEXT,
            recorded_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_workflow_run_summaries_workflow_time
            ON workflow_run_summaries(workflow_id, started_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_workflow_run_summaries_recorded
            ON workflow_run_summaries(recorded_at_ms DESC);
        "#,
    )?;
    Ok(())
}

pub(crate) fn current_schema_version(
    conn: &Connection,
) -> Result<Option<i64>, DiagnosticsLedgerError> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM sqlite_master
             WHERE type = 'table' AND name = 'ledger_schema_migrations'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        return Ok(None);
    }
    let version = conn.query_row(
        "SELECT MAX(version) FROM ledger_schema_migrations",
        [],
        |row| row.get(0),
    )?;
    Ok(Some(version))
}
