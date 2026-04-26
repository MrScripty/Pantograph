use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::records::{RetentionClass, DEFAULT_STANDARD_RETENTION_DAYS};
use crate::util::now_ms;
use crate::DiagnosticsLedgerError;

pub(crate) const SCHEMA_VERSION: i64 = 4;
const SCHEMA_CHECKSUM: &str = "pantograph-diagnostics-ledger-v4";

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
