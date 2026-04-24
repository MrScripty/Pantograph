use rusqlite::{params, Connection, OptionalExtension, Transaction};
use uuid::Uuid;

use crate::{
    AttributionError, BucketId, BucketRecord, ClientCredential, ClientCredentialId,
    ClientCredentialStatus, ClientId, ClientSessionId, ClientSessionLifecycleState,
    ClientSessionRecord, ClientStatus, SessionLifecycleRecord,
};

pub(crate) struct PersistedCredential {
    pub(crate) record: ClientCredential,
    pub(crate) client_status: ClientStatus,
    pub(crate) salt: Vec<u8>,
    pub(crate) digest: Vec<u8>,
}

pub(crate) fn sqlite_conversion_error(error: AttributionError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

pub(crate) fn fetch_credential(
    conn: &Connection,
    credential_id: &ClientCredentialId,
) -> Result<PersistedCredential, AttributionError> {
    conn.query_row(
        "SELECT cc.client_credential_id, cc.client_id, cc.status, cc.created_at_ms,
                cc.revoked_at_ms, cc.salt, cc.digest, c.status
         FROM client_credentials cc
         JOIN clients c ON c.client_id = cc.client_id
         WHERE cc.client_credential_id = ?1",
        params![credential_id.as_str()],
        |row| {
            Ok(PersistedCredential {
                record: ClientCredential {
                    client_credential_id: ClientCredentialId::try_from(row.get::<_, String>(0)?)
                        .map_err(sqlite_conversion_error)?,
                    client_id: ClientId::try_from(row.get::<_, String>(1)?)
                        .map_err(sqlite_conversion_error)?,
                    status: credential_status_from_db(&row.get::<_, String>(2)?),
                    created_at_ms: row.get(3)?,
                    revoked_at_ms: row.get(4)?,
                },
                salt: row.get(5)?,
                digest: row.get(6)?,
                client_status: client_status_from_db(&row.get::<_, String>(7)?),
            })
        },
    )
    .optional()?
    .ok_or(AttributionError::NotFound {
        entity: "client_credential",
    })
}

pub(crate) fn active_session_for_client(
    conn: &Connection,
    client_id: &ClientId,
) -> Result<Option<ClientSessionRecord>, AttributionError> {
    conn.query_row(
        "SELECT client_session_id, client_id, opened_at_ms, latest_lifecycle_state,
                grace_deadline_ms, superseded_by_session_id
         FROM client_sessions
         WHERE client_id = ?1
           AND latest_lifecycle_state IN ('opening', 'connected', 'disconnected_grace')",
        params![client_id.as_str()],
        session_from_row,
    )
    .optional()
    .map_err(AttributionError::from)
}

pub(crate) fn fetch_session(
    conn: &Connection,
    session_id: &ClientSessionId,
) -> Result<ClientSessionRecord, AttributionError> {
    conn.query_row(
        "SELECT client_session_id, client_id, opened_at_ms, latest_lifecycle_state,
                grace_deadline_ms, superseded_by_session_id
         FROM client_sessions
         WHERE client_session_id = ?1",
        params![session_id.as_str()],
        session_from_row,
    )
    .optional()?
    .ok_or(AttributionError::NotFound {
        entity: "client_session",
    })
}

pub(crate) fn session_from_row(
    row: &rusqlite::Row<'_>,
) -> Result<ClientSessionRecord, rusqlite::Error> {
    Ok(ClientSessionRecord {
        client_session_id: ClientSessionId::try_from(row.get::<_, String>(0)?)
            .map_err(sqlite_conversion_error)?,
        client_id: ClientId::try_from(row.get::<_, String>(1)?).map_err(sqlite_conversion_error)?,
        opened_at_ms: row.get(2)?,
        latest_lifecycle_state: session_state_from_db(&row.get::<_, String>(3)?),
        grace_deadline_ms: row.get(4)?,
        superseded_by_session_id: row
            .get::<_, Option<String>>(5)?
            .map(ClientSessionId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
    })
}

pub(crate) fn update_session_state(
    tx: &Transaction<'_>,
    session_id: &ClientSessionId,
    state: ClientSessionLifecycleState,
    grace_deadline_ms: Option<i64>,
    superseded_by_session_id: Option<&ClientSessionId>,
    now: i64,
    reason: Option<&str>,
) -> Result<(), AttributionError> {
    tx.execute(
        "UPDATE client_sessions
         SET latest_lifecycle_state = ?2,
             grace_deadline_ms = ?3,
             superseded_by_session_id = COALESCE(?4, superseded_by_session_id)
         WHERE client_session_id = ?1",
        params![
            session_id.as_str(),
            state.as_db(),
            grace_deadline_ms,
            superseded_by_session_id.map(ClientSessionId::as_str)
        ],
    )?;
    insert_lifecycle_record(tx, session_id, state, now, reason, superseded_by_session_id)?;
    Ok(())
}

pub(crate) fn insert_lifecycle_record(
    tx: &Transaction<'_>,
    session_id: &ClientSessionId,
    state: ClientSessionLifecycleState,
    now: i64,
    reason: Option<&str>,
    related_session_id: Option<&ClientSessionId>,
) -> Result<(), AttributionError> {
    tx.execute(
        "INSERT INTO session_lifecycle_records
            (event_id, client_session_id, lifecycle_state, occurred_at_ms, reason,
             related_session_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            Uuid::new_v4().to_string(),
            session_id.as_str(),
            state.as_db(),
            now,
            reason,
            related_session_id.map(ClientSessionId::as_str)
        ],
    )?;
    Ok(())
}

pub(crate) fn active_bucket_by_name(
    conn: &Connection,
    client_id: &ClientId,
    name: &str,
) -> Result<Option<BucketRecord>, AttributionError> {
    conn.query_row(
        "SELECT bucket_id, client_id, name, metadata_json, created_at_ms,
                deleted_at_ms, deletion_reason
         FROM buckets
         WHERE client_id = ?1 AND name = ?2 AND deleted_at_ms IS NULL",
        params![client_id.as_str(), name],
        bucket_from_row,
    )
    .optional()
    .map_err(AttributionError::from)
}

pub(crate) fn fetch_bucket(
    conn: &Connection,
    bucket_id: &BucketId,
) -> Result<BucketRecord, AttributionError> {
    conn.query_row(
        "SELECT bucket_id, client_id, name, metadata_json, created_at_ms,
                deleted_at_ms, deletion_reason
         FROM buckets
         WHERE bucket_id = ?1",
        params![bucket_id.as_str()],
        bucket_from_row,
    )
    .optional()?
    .ok_or(AttributionError::NotFound { entity: "bucket" })
}

fn bucket_from_row(row: &rusqlite::Row<'_>) -> Result<BucketRecord, rusqlite::Error> {
    Ok(BucketRecord {
        bucket_id: BucketId::try_from(row.get::<_, String>(0)?).map_err(sqlite_conversion_error)?,
        client_id: ClientId::try_from(row.get::<_, String>(1)?).map_err(sqlite_conversion_error)?,
        name: row.get(2)?,
        metadata_json: row.get(3)?,
        created_at_ms: row.get(4)?,
        deleted_at_ms: row.get(5)?,
        deletion_reason: row.get(6)?,
    })
}

pub(crate) fn default_bucket_for_session(
    conn: &Connection,
    session_id: &ClientSessionId,
) -> Result<BucketRecord, AttributionError> {
    conn.query_row(
        "SELECT b.bucket_id, b.client_id, b.name, b.metadata_json, b.created_at_ms,
                b.deleted_at_ms, b.deletion_reason
         FROM default_bucket_assignments d
         JOIN buckets b ON b.bucket_id = d.bucket_id
         WHERE d.client_session_id = ?1",
        params![session_id.as_str()],
        bucket_from_row,
    )
    .optional()?
    .ok_or(AttributionError::NotFound {
        entity: "default_bucket_assignment",
    })
}

pub(crate) fn default_bucket_assignment_count(
    conn: &Connection,
    bucket_id: &BucketId,
) -> Result<i64, AttributionError> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM default_bucket_assignments WHERE bucket_id = ?1",
        params![bucket_id.as_str()],
        |row| row.get(0),
    )?)
}

pub(crate) fn active_workflow_run_count(
    conn: &Connection,
    bucket_id: &BucketId,
) -> Result<i64, AttributionError> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM workflow_runs
         WHERE bucket_id = ?1 AND status = 'running'",
        params![bucket_id.as_str()],
        |row| row.get(0),
    )?)
}

pub(crate) fn lifecycle_record_from_row(
    row: &rusqlite::Row<'_>,
) -> Result<SessionLifecycleRecord, rusqlite::Error> {
    Ok(SessionLifecycleRecord {
        event_id: row.get(0)?,
        client_session_id: ClientSessionId::try_from(row.get::<_, String>(1)?)
            .map_err(sqlite_conversion_error)?,
        lifecycle_state: session_state_from_db(&row.get::<_, String>(2)?),
        occurred_at_ms: row.get(3)?,
        reason: row.get(4)?,
        related_session_id: row
            .get::<_, Option<String>>(5)?
            .map(ClientSessionId::try_from)
            .transpose()
            .map_err(sqlite_conversion_error)?,
    })
}

fn client_status_from_db(value: &str) -> ClientStatus {
    match value {
        "disabled" => ClientStatus::Disabled,
        _ => ClientStatus::Active,
    }
}

fn credential_status_from_db(value: &str) -> ClientCredentialStatus {
    match value {
        "revoked" => ClientCredentialStatus::Revoked,
        _ => ClientCredentialStatus::Active,
    }
}

fn session_state_from_db(value: &str) -> ClientSessionLifecycleState {
    match value {
        "opening" => ClientSessionLifecycleState::Opening,
        "disconnected_grace" => ClientSessionLifecycleState::DisconnectedGrace,
        "expired" => ClientSessionLifecycleState::Expired,
        "taken_over" => ClientSessionLifecycleState::TakenOver,
        "closed" => ClientSessionLifecycleState::Closed,
        _ => ClientSessionLifecycleState::Connected,
    }
}
