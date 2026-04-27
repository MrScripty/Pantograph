use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::ids::{
    validate_bucket_name, validate_optional_text, DEFAULT_BUCKET_NAME, MAX_NAME_LEN, MAX_REASON_LEN,
};
use crate::schema::{apply_schema, current_schema_version, SCHEMA_VERSION};
use crate::sqlite_rows::{
    active_bucket_by_name, active_session_for_client, active_workflow_run_count,
    default_bucket_assignment_count, default_bucket_for_session, fetch_bucket, fetch_credential,
    fetch_session, insert_lifecycle_record, lifecycle_record_from_row, update_session_state,
};
use crate::util::{credential_digest, now_ms};
use crate::{
    AttributionError, AttributionRepository, BucketCreateRequest, BucketDeleteRequest,
    BucketRecord, BucketSelection, ClientCredential, ClientCredentialId, ClientCredentialStatus,
    ClientId, ClientRegistrationRequest, ClientRegistrationResponse,
    ClientSessionDisconnectRequest, ClientSessionExpireRequest, ClientSessionId,
    ClientSessionLifecycleState, ClientSessionOpenRequest, ClientSessionOpenResponse,
    ClientSessionRecord, ClientSessionResumeRequest, ClientStatus, CredentialProofRequest,
    CredentialSecret, DefaultBucketAssignment, SessionLifecycleRecord, WorkflowRunRecord,
    WorkflowRunSnapshotRecord, WorkflowRunSnapshotRequest, WorkflowRunStartRequest,
    WorkflowRunStatus, WorkflowVersionRecord, WorkflowVersionResolveRequest,
};

const MAX_SEMANTIC_VERSION_LEN: usize = 64;
const MAX_EXECUTION_FINGERPRINT_LEN: usize = 256;
const MAX_EXECUTABLE_TOPOLOGY_JSON_LEN: usize = 262_144;
const MAX_WORKFLOW_EXECUTION_SESSION_ID_LEN: usize = 128;
const MAX_RUN_SNAPSHOT_JSON_LEN: usize = 262_144;

pub struct SqliteAttributionStore {
    pub(crate) conn: Connection,
}

impl SqliteAttributionStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AttributionError> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    pub fn open_in_memory() -> Result<Self, AttributionError> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    pub fn from_connection(conn: Connection) -> Result<Self, AttributionError> {
        let mut store = Self { conn };
        store.initialize()?;
        Ok(store)
    }

    pub fn from_existing_without_migration(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn initialize(&mut self) -> Result<(), AttributionError> {
        self.conn.pragma_update(None, "foreign_keys", "ON")?;
        let version = current_schema_version(&self.conn)?;
        if let Some(found) = version {
            if found != SCHEMA_VERSION {
                return Err(AttributionError::UnsupportedSchemaVersion { found });
            }
            return Ok(());
        }

        let tx = self.conn.transaction()?;
        apply_schema(&tx)?;
        tx.commit()?;
        Ok(())
    }

    pub fn lifecycle_records(
        &self,
        session_id: &ClientSessionId,
    ) -> Result<Vec<SessionLifecycleRecord>, AttributionError> {
        let mut stmt = self.conn.prepare(
            "SELECT event_id, client_session_id, lifecycle_state, occurred_at_ms, reason,
                    related_session_id
             FROM session_lifecycle_records
             WHERE client_session_id = ?1
             ORDER BY occurred_at_ms, event_id",
        )?;
        let rows = stmt
            .query_map(params![session_id.as_str()], lifecycle_record_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn workflow_run_snapshot(
        &self,
        workflow_run_id: &crate::WorkflowRunId,
    ) -> Result<Option<WorkflowRunSnapshotRecord>, AttributionError> {
        workflow_run_snapshot_by_run_id(&self.conn, workflow_run_id)
    }
}

impl AttributionRepository for SqliteAttributionStore {
    fn register_client(
        &mut self,
        request: ClientRegistrationRequest,
    ) -> Result<ClientRegistrationResponse, AttributionError> {
        let now = now_ms();
        validate_optional_text(
            "display_name",
            request.display_name.as_deref(),
            MAX_NAME_LEN,
        )?;
        validate_optional_text("metadata_json", request.metadata_json.as_deref(), 8192)?;

        let client_id = ClientId::generate();
        let credential_id = ClientCredentialId::generate();
        let credential_secret = CredentialSecret::generate();
        let salt = Uuid::new_v4().as_bytes().to_vec();
        let digest = credential_digest(&salt, &credential_secret);

        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO clients (client_id, display_name, metadata_json, status, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                client_id.as_str(),
                request.display_name,
                request.metadata_json,
                ClientStatus::Active.as_db(),
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO client_credentials
                (client_credential_id, client_id, salt, digest, status, created_at_ms, revoked_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
            params![
                credential_id.as_str(),
                client_id.as_str(),
                salt,
                digest,
                ClientCredentialStatus::Active.as_db(),
                now
            ],
        )?;
        tx.commit()?;

        Ok(ClientRegistrationResponse {
            client: crate::records::ClientRecord {
                client_id: client_id.clone(),
                display_name: request.display_name,
                metadata_json: request.metadata_json,
                status: ClientStatus::Active,
                created_at_ms: now,
            },
            credential: ClientCredential {
                client_credential_id: credential_id,
                client_id,
                status: ClientCredentialStatus::Active,
                created_at_ms: now,
                revoked_at_ms: None,
            },
            credential_secret,
        })
    }

    fn verify_credential(
        &self,
        request: &CredentialProofRequest,
    ) -> Result<ClientCredential, AttributionError> {
        let credential = fetch_credential(&self.conn, &request.credential_id)?;
        if credential.client_status != ClientStatus::Active {
            return Err(AttributionError::ClientDisabled);
        }
        if credential.record.status != ClientCredentialStatus::Active {
            return Err(AttributionError::CredentialRevoked);
        }

        let actual = credential_digest(&credential.salt, &request.secret);
        if actual != credential.digest {
            return Err(AttributionError::CredentialMismatch);
        }
        Ok(credential.record)
    }

    fn open_session(
        &mut self,
        request: ClientSessionOpenRequest,
    ) -> Result<ClientSessionOpenResponse, AttributionError> {
        validate_optional_text("reason", request.reason.as_deref(), MAX_REASON_LEN)?;
        let credential = self.verify_credential(&request.credential)?;
        let now = now_ms();
        let session_id = ClientSessionId::generate();
        let tx = self.conn.transaction()?;

        let existing_active = active_session_for_client(&tx, &credential.client_id)?;
        if let Some(existing) = existing_active.as_ref() {
            if !request.takeover {
                return Err(AttributionError::DuplicateActiveSession {
                    client_id: credential.client_id,
                });
            }
            update_session_state(
                &tx,
                &existing.client_session_id,
                ClientSessionLifecycleState::TakenOver,
                None,
                None,
                now,
                request.reason.as_deref(),
            )?;
        }

        tx.execute(
            "INSERT INTO client_sessions
                (client_session_id, client_id, opened_at_ms, latest_lifecycle_state,
                 grace_deadline_ms, superseded_by_session_id)
             VALUES (?1, ?2, ?3, ?4, NULL, NULL)",
            params![
                session_id.as_str(),
                credential.client_id.as_str(),
                now,
                ClientSessionLifecycleState::Connected.as_db()
            ],
        )?;
        insert_lifecycle_record(
            &tx,
            &session_id,
            ClientSessionLifecycleState::Connected,
            now,
            request.reason.as_deref(),
            None,
        )?;
        let default_bucket = ensure_default_bucket(&tx, &credential.client_id, now)?;
        tx.execute(
            "INSERT INTO default_bucket_assignments
                (client_session_id, bucket_id, assigned_at_ms)
             VALUES (?1, ?2, ?3)",
            params![session_id.as_str(), default_bucket.bucket_id.as_str(), now],
        )?;
        if let Some(existing) = existing_active.as_ref() {
            tx.execute(
                "UPDATE client_sessions
                 SET superseded_by_session_id = ?2
                 WHERE client_session_id = ?1",
                params![existing.client_session_id.as_str(), session_id.as_str()],
            )?;
        }
        tx.commit()?;

        Ok(ClientSessionOpenResponse {
            session: ClientSessionRecord {
                client_session_id: session_id.clone(),
                client_id: credential.client_id,
                opened_at_ms: now,
                latest_lifecycle_state: ClientSessionLifecycleState::Connected,
                grace_deadline_ms: None,
                superseded_by_session_id: None,
            },
            default_bucket_assignment: DefaultBucketAssignment {
                client_session_id: session_id,
                bucket_id: default_bucket.bucket_id.clone(),
                assigned_at_ms: now,
            },
            default_bucket,
            superseded_session_id: existing_active.map(|session| session.client_session_id),
        })
    }

    fn resume_session(
        &mut self,
        request: ClientSessionResumeRequest,
    ) -> Result<ClientSessionRecord, AttributionError> {
        validate_optional_text("reason", request.reason.as_deref(), MAX_REASON_LEN)?;
        let credential = self.verify_credential(&request.credential)?;
        let tx = self.conn.transaction()?;
        let session = fetch_session(&tx, &request.client_session_id)?;
        if session.client_id != credential.client_id {
            return Err(AttributionError::SessionClientMismatch);
        }
        match session.latest_lifecycle_state {
            ClientSessionLifecycleState::Connected => Ok(session),
            ClientSessionLifecycleState::DisconnectedGrace => {
                let now = now_ms();
                update_session_state(
                    &tx,
                    &request.client_session_id,
                    ClientSessionLifecycleState::Connected,
                    None,
                    None,
                    now,
                    request.reason.as_deref(),
                )?;
                tx.commit()?;
                fetch_session(&self.conn, &request.client_session_id)
            }
            state => Err(AttributionError::SessionNotResumable { state }),
        }
    }

    fn disconnect_session(
        &mut self,
        request: ClientSessionDisconnectRequest,
    ) -> Result<ClientSessionRecord, AttributionError> {
        validate_optional_text("reason", request.reason.as_deref(), MAX_REASON_LEN)?;
        let credential = self.verify_credential(&request.credential)?;
        let tx = self.conn.transaction()?;
        let session = fetch_session(&tx, &request.client_session_id)?;
        if session.client_id != credential.client_id {
            return Err(AttributionError::SessionClientMismatch);
        }
        if session.latest_lifecycle_state != ClientSessionLifecycleState::Connected {
            return Err(AttributionError::InvalidSessionTransition {
                from: session.latest_lifecycle_state,
                to: ClientSessionLifecycleState::DisconnectedGrace,
            });
        }
        update_session_state(
            &tx,
            &request.client_session_id,
            ClientSessionLifecycleState::DisconnectedGrace,
            Some(request.grace_deadline_ms),
            None,
            now_ms(),
            request.reason.as_deref(),
        )?;
        tx.commit()?;
        fetch_session(&self.conn, &request.client_session_id)
    }

    fn expire_session(
        &mut self,
        request: ClientSessionExpireRequest,
    ) -> Result<ClientSessionRecord, AttributionError> {
        validate_optional_text("reason", request.reason.as_deref(), MAX_REASON_LEN)?;
        let tx = self.conn.transaction()?;
        let session = fetch_session(&tx, &request.client_session_id)?;
        if session.latest_lifecycle_state != ClientSessionLifecycleState::DisconnectedGrace {
            return Err(AttributionError::InvalidSessionTransition {
                from: session.latest_lifecycle_state,
                to: ClientSessionLifecycleState::Expired,
            });
        }
        update_session_state(
            &tx,
            &request.client_session_id,
            ClientSessionLifecycleState::Expired,
            None,
            None,
            now_ms(),
            request.reason.as_deref(),
        )?;
        tx.commit()?;
        fetch_session(&self.conn, &request.client_session_id)
    }

    fn create_bucket(
        &mut self,
        request: BucketCreateRequest,
    ) -> Result<BucketRecord, AttributionError> {
        let name = validate_bucket_name(&request.name)?;
        validate_optional_text("metadata_json", request.metadata_json.as_deref(), 8192)?;
        if name == DEFAULT_BUCKET_NAME {
            return Err(AttributionError::BucketNameReserved { name });
        }
        let credential = self.verify_credential(&request.credential)?;
        let now = now_ms();
        let bucket_id = crate::BucketId::generate();
        let tx = self.conn.transaction()?;

        if active_bucket_by_name(&tx, &credential.client_id, &name)?.is_some() {
            return Err(AttributionError::BucketNameCollision { name });
        }

        tx.execute(
            "INSERT INTO buckets
                (bucket_id, client_id, name, metadata_json, created_at_ms,
                 deleted_at_ms, deletion_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL)",
            params![
                bucket_id.as_str(),
                credential.client_id.as_str(),
                name,
                request.metadata_json,
                now
            ],
        )?;
        tx.commit()?;
        fetch_bucket(&self.conn, &bucket_id)
    }

    fn delete_bucket(
        &mut self,
        request: BucketDeleteRequest,
    ) -> Result<BucketRecord, AttributionError> {
        validate_optional_text("reason", request.reason.as_deref(), MAX_REASON_LEN)?;
        let credential = self.verify_credential(&request.credential)?;
        let tx = self.conn.transaction()?;
        let bucket = fetch_bucket(&tx, &request.bucket_id)?;
        if bucket.client_id != credential.client_id {
            return Err(AttributionError::BucketClientMismatch);
        }
        if bucket.name == DEFAULT_BUCKET_NAME {
            return Err(AttributionError::DefaultBucketProtected);
        }
        if default_bucket_assignment_count(&tx, &bucket.bucket_id)? > 0 {
            return Err(AttributionError::DefaultBucketProtected);
        }
        if active_workflow_run_count(&tx, &bucket.bucket_id)? > 0 {
            return Err(AttributionError::BucketDeletionProtected {
                bucket_id: bucket.bucket_id,
            });
        }
        tx.execute(
            "UPDATE buckets
             SET deleted_at_ms = ?2, deletion_reason = ?3
             WHERE bucket_id = ?1",
            params![request.bucket_id.as_str(), now_ms(), request.reason],
        )?;
        tx.commit()?;
        fetch_bucket(&self.conn, &request.bucket_id)
    }

    fn start_workflow_run(
        &mut self,
        request: WorkflowRunStartRequest,
    ) -> Result<WorkflowRunRecord, AttributionError> {
        let credential = self.verify_credential(&request.credential)?;
        let tx = self.conn.transaction()?;
        let session = fetch_session(&tx, &request.client_session_id)?;
        if session.client_id != credential.client_id {
            return Err(AttributionError::SessionClientMismatch);
        }
        if !session.latest_lifecycle_state.is_active() {
            return Err(AttributionError::SessionNotActive {
                state: session.latest_lifecycle_state,
            });
        }

        let bucket = match request.bucket_selection {
            BucketSelection::Default => {
                default_bucket_for_session(&tx, &session.client_session_id)?
            }
            BucketSelection::Explicit(bucket_id) => fetch_bucket(&tx, &bucket_id)?,
        };
        if bucket.client_id != session.client_id {
            return Err(AttributionError::BucketClientMismatch);
        }
        if bucket.deleted_at_ms.is_some() {
            return Err(AttributionError::NotFound { entity: "bucket" });
        }

        let now = now_ms();
        let run_id = crate::WorkflowRunId::generate();
        tx.execute(
            "INSERT INTO workflow_runs
                (workflow_run_id, workflow_id, client_id, client_session_id,
                 bucket_id, status, started_at_ms, completed_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            params![
                run_id.as_str(),
                request.workflow_id.as_str(),
                session.client_id.as_str(),
                session.client_session_id.as_str(),
                bucket.bucket_id.as_str(),
                WorkflowRunStatus::Running.as_db(),
                now
            ],
        )?;
        tx.commit()?;

        Ok(WorkflowRunRecord {
            workflow_run_id: run_id,
            workflow_id: request.workflow_id,
            client_id: session.client_id,
            client_session_id: session.client_session_id,
            bucket_id: bucket.bucket_id,
            status: WorkflowRunStatus::Running,
            started_at_ms: now,
            completed_at_ms: None,
        })
    }

    fn resolve_workflow_version(
        &mut self,
        request: WorkflowVersionResolveRequest,
    ) -> Result<WorkflowVersionRecord, AttributionError> {
        let semantic_version = validate_semantic_version(request.semantic_version)?;
        let execution_fingerprint = validate_required_boundary_text(
            "execution_fingerprint",
            request.execution_fingerprint,
            MAX_EXECUTION_FINGERPRINT_LEN,
        )?;
        let executable_topology_json =
            validate_executable_topology_json(request.executable_topology_json)?;

        let tx = self.conn.transaction()?;
        let semantic_match =
            workflow_version_by_semantic_version(&tx, &request.workflow_id, &semantic_version)?;
        if let Some(record) = semantic_match {
            if record.execution_fingerprint != execution_fingerprint {
                return Err(AttributionError::WorkflowSemanticVersionConflict {
                    workflow_id: request.workflow_id,
                    semantic_version,
                });
            }
            return Ok(record);
        }

        let fingerprint_match = workflow_version_by_execution_fingerprint(
            &tx,
            &request.workflow_id,
            &execution_fingerprint,
        )?;
        if let Some(record) = fingerprint_match {
            if record.semantic_version != semantic_version {
                return Err(AttributionError::WorkflowFingerprintVersionConflict {
                    workflow_id: request.workflow_id,
                    execution_fingerprint,
                });
            }
            return Ok(record);
        }

        let now = now_ms();
        let workflow_version_id = crate::WorkflowVersionId::generate();
        tx.execute(
            "INSERT INTO workflow_versions
                (workflow_version_id, workflow_id, semantic_version, execution_fingerprint,
                 executable_topology_json, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                workflow_version_id.as_str(),
                request.workflow_id.as_str(),
                semantic_version.as_str(),
                execution_fingerprint.as_str(),
                executable_topology_json.as_str(),
                now
            ],
        )?;
        tx.commit()?;

        Ok(WorkflowVersionRecord {
            workflow_version_id,
            workflow_id: request.workflow_id,
            semantic_version,
            execution_fingerprint,
            executable_topology_json,
            created_at_ms: now,
        })
    }

    fn create_workflow_run_snapshot(
        &mut self,
        request: WorkflowRunSnapshotRequest,
    ) -> Result<WorkflowRunSnapshotRecord, AttributionError> {
        let workflow_semantic_version =
            validate_semantic_version(request.workflow_semantic_version)?;
        let workflow_execution_fingerprint = validate_required_boundary_text(
            "workflow_execution_fingerprint",
            request.workflow_execution_fingerprint,
            MAX_EXECUTION_FINGERPRINT_LEN,
        )?;
        let workflow_execution_session_id = validate_required_boundary_text(
            "workflow_execution_session_id",
            request.workflow_execution_session_id,
            MAX_WORKFLOW_EXECUTION_SESSION_ID_LEN,
        )?;
        let inputs_json = validate_json_text("inputs_json", request.inputs_json)?;
        let output_targets_json =
            validate_optional_json_text("output_targets_json", request.output_targets_json)?;
        let override_selection_json = validate_optional_json_text(
            "override_selection_json",
            request.override_selection_json,
        )?;
        let timeout_ms = request
            .timeout_ms
            .map(|value| i64::try_from(value).unwrap_or(i64::MAX));

        let tx = self.conn.transaction()?;
        let version = workflow_version_by_id(&tx, &request.workflow_version_id)?.ok_or(
            AttributionError::NotFound {
                entity: "workflow_version",
            },
        )?;
        if version.workflow_id != request.workflow_id
            || version.semantic_version != workflow_semantic_version
            || version.execution_fingerprint != workflow_execution_fingerprint
        {
            return Err(AttributionError::WorkflowFingerprintVersionConflict {
                workflow_id: request.workflow_id,
                execution_fingerprint: workflow_execution_fingerprint,
            });
        }

        let now = now_ms();
        let workflow_run_snapshot_id = crate::WorkflowRunSnapshotId::generate();
        tx.execute(
            "INSERT INTO workflow_run_snapshots
                (workflow_run_snapshot_id, workflow_run_id, workflow_id, workflow_version_id,
                 workflow_semantic_version, workflow_execution_fingerprint,
                 workflow_execution_session_id, priority, timeout_ms, inputs_json,
                 output_targets_json, override_selection_json, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                workflow_run_snapshot_id.as_str(),
                request.workflow_run_id.as_str(),
                request.workflow_id.as_str(),
                request.workflow_version_id.as_str(),
                workflow_semantic_version.as_str(),
                workflow_execution_fingerprint.as_str(),
                workflow_execution_session_id.as_str(),
                request.priority,
                timeout_ms,
                inputs_json.as_str(),
                output_targets_json.as_deref(),
                override_selection_json.as_deref(),
                now
            ],
        )?;
        tx.commit()?;

        Ok(WorkflowRunSnapshotRecord {
            workflow_run_snapshot_id,
            workflow_run_id: request.workflow_run_id,
            workflow_id: request.workflow_id,
            workflow_version_id: request.workflow_version_id,
            workflow_semantic_version,
            workflow_execution_fingerprint,
            workflow_execution_session_id,
            priority: request.priority,
            timeout_ms: request.timeout_ms,
            inputs_json,
            output_targets_json,
            override_selection_json,
            created_at_ms: now,
        })
    }
}

fn workflow_version_by_semantic_version(
    conn: &rusqlite::Connection,
    workflow_id: &crate::WorkflowId,
    semantic_version: &str,
) -> Result<Option<WorkflowVersionRecord>, AttributionError> {
    let mut stmt = conn.prepare(
        "SELECT workflow_version_id, workflow_id, semantic_version, execution_fingerprint,
                executable_topology_json, created_at_ms
         FROM workflow_versions
         WHERE workflow_id = ?1 AND semantic_version = ?2",
    )?;
    let record = stmt
        .query_row(
            params![workflow_id.as_str(), semantic_version],
            workflow_version_from_row,
        )
        .optional()?;
    Ok(record)
}

fn workflow_version_by_execution_fingerprint(
    conn: &rusqlite::Connection,
    workflow_id: &crate::WorkflowId,
    execution_fingerprint: &str,
) -> Result<Option<WorkflowVersionRecord>, AttributionError> {
    let mut stmt = conn.prepare(
        "SELECT workflow_version_id, workflow_id, semantic_version, execution_fingerprint,
                executable_topology_json, created_at_ms
         FROM workflow_versions
         WHERE workflow_id = ?1 AND execution_fingerprint = ?2",
    )?;
    let record = stmt
        .query_row(
            params![workflow_id.as_str(), execution_fingerprint],
            workflow_version_from_row,
        )
        .optional()?;
    Ok(record)
}

fn workflow_version_by_id(
    conn: &rusqlite::Connection,
    workflow_version_id: &crate::WorkflowVersionId,
) -> Result<Option<WorkflowVersionRecord>, AttributionError> {
    let mut stmt = conn.prepare(
        "SELECT workflow_version_id, workflow_id, semantic_version, execution_fingerprint,
                executable_topology_json, created_at_ms
         FROM workflow_versions
         WHERE workflow_version_id = ?1",
    )?;
    let record = stmt
        .query_row(
            params![workflow_version_id.as_str()],
            workflow_version_from_row,
        )
        .optional()?;
    Ok(record)
}

fn workflow_version_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkflowVersionRecord> {
    Ok(WorkflowVersionRecord {
        workflow_version_id: row
            .get::<_, String>(0)
            .and_then(parse_workflow_version_id)?,
        workflow_id: row.get::<_, String>(1).and_then(parse_workflow_id)?,
        semantic_version: row.get(2)?,
        execution_fingerprint: row.get(3)?,
        executable_topology_json: row.get(4)?,
        created_at_ms: row.get(5)?,
    })
}

fn workflow_run_snapshot_by_run_id(
    conn: &rusqlite::Connection,
    workflow_run_id: &crate::WorkflowRunId,
) -> Result<Option<WorkflowRunSnapshotRecord>, AttributionError> {
    let mut stmt = conn.prepare(
        "SELECT workflow_run_snapshot_id, workflow_run_id, workflow_id, workflow_version_id,
                workflow_semantic_version, workflow_execution_fingerprint,
                workflow_execution_session_id, priority, timeout_ms, inputs_json,
                output_targets_json, override_selection_json, created_at_ms
         FROM workflow_run_snapshots
         WHERE workflow_run_id = ?1",
    )?;
    let record = stmt
        .query_row(
            params![workflow_run_id.as_str()],
            workflow_run_snapshot_from_row,
        )
        .optional()?;
    Ok(record)
}

fn workflow_run_snapshot_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<WorkflowRunSnapshotRecord> {
    let timeout_ms = row
        .get::<_, Option<i64>>(8)?
        .map(|value| u64::try_from(value).unwrap_or(u64::MAX));
    Ok(WorkflowRunSnapshotRecord {
        workflow_run_snapshot_id: row
            .get::<_, String>(0)
            .and_then(parse_workflow_run_snapshot_id)?,
        workflow_run_id: row.get::<_, String>(1).and_then(parse_workflow_run_id)?,
        workflow_id: row.get::<_, String>(2).and_then(parse_workflow_id)?,
        workflow_version_id: row
            .get::<_, String>(3)
            .and_then(parse_workflow_version_id)?,
        workflow_semantic_version: row.get(4)?,
        workflow_execution_fingerprint: row.get(5)?,
        workflow_execution_session_id: row.get(6)?,
        priority: row.get(7)?,
        timeout_ms,
        inputs_json: row.get(9)?,
        output_targets_json: row.get(10)?,
        override_selection_json: row.get(11)?,
        created_at_ms: row.get(12)?,
    })
}

fn parse_workflow_run_snapshot_id(value: String) -> rusqlite::Result<crate::WorkflowRunSnapshotId> {
    crate::WorkflowRunSnapshotId::try_from(value)
        .map_err(crate::sqlite_rows::sqlite_conversion_error)
}

fn parse_workflow_run_id(value: String) -> rusqlite::Result<crate::WorkflowRunId> {
    crate::WorkflowRunId::try_from(value).map_err(crate::sqlite_rows::sqlite_conversion_error)
}

fn parse_workflow_version_id(value: String) -> rusqlite::Result<crate::WorkflowVersionId> {
    crate::WorkflowVersionId::try_from(value).map_err(crate::sqlite_rows::sqlite_conversion_error)
}

fn parse_workflow_id(value: String) -> rusqlite::Result<crate::WorkflowId> {
    crate::WorkflowId::try_from(value).map_err(crate::sqlite_rows::sqlite_conversion_error)
}

fn validate_semantic_version(value: String) -> Result<String, AttributionError> {
    let value =
        validate_required_boundary_text("semantic_version", value, MAX_SEMANTIC_VERSION_LEN)?;
    let mut parts = value.split('.');
    let valid = parts.next().is_some_and(is_numeric_semver_part)
        && parts.next().is_some_and(is_numeric_semver_part)
        && parts.next().is_some_and(is_numeric_semver_part)
        && parts.next().is_none();
    if !valid {
        return Err(AttributionError::InvalidWorkflowSemanticVersion { value });
    }
    Ok(value)
}

fn is_numeric_semver_part(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
}

fn validate_executable_topology_json(value: String) -> Result<String, AttributionError> {
    validate_json_text_with_limit(
        "executable_topology_json",
        value,
        MAX_EXECUTABLE_TOPOLOGY_JSON_LEN,
    )
}

fn validate_json_text(field: &'static str, value: String) -> Result<String, AttributionError> {
    validate_json_text_with_limit(field, value, MAX_RUN_SNAPSHOT_JSON_LEN)
}

fn validate_optional_json_text(
    field: &'static str,
    value: Option<String>,
) -> Result<Option<String>, AttributionError> {
    value
        .map(|json| validate_json_text(field, json))
        .transpose()
}

fn validate_json_text_with_limit(
    field: &'static str,
    value: String,
    max_len: usize,
) -> Result<String, AttributionError> {
    let value = validate_required_boundary_text(field, value, max_len)?;
    serde_json::from_str::<serde_json::Value>(&value)
        .map_err(|_| AttributionError::InvalidField { field })?;
    Ok(value)
}

fn validate_required_boundary_text(
    field: &'static str,
    value: String,
    max_len: usize,
) -> Result<String, AttributionError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AttributionError::MissingField { field });
    }
    if trimmed.len() > max_len {
        return Err(AttributionError::FieldTooLong { field, max_len });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(AttributionError::InvalidField { field });
    }
    Ok(trimmed.to_string())
}

fn ensure_default_bucket(
    tx: &rusqlite::Transaction<'_>,
    client_id: &ClientId,
    now: i64,
) -> Result<BucketRecord, AttributionError> {
    if let Some(bucket) = active_bucket_by_name(tx, client_id, DEFAULT_BUCKET_NAME)? {
        return Ok(bucket);
    }
    let bucket_id = crate::BucketId::generate();
    tx.execute(
        "INSERT INTO buckets
            (bucket_id, client_id, name, metadata_json, created_at_ms,
             deleted_at_ms, deletion_reason)
         VALUES (?1, ?2, ?3, NULL, ?4, NULL, NULL)",
        params![
            bucket_id.as_str(),
            client_id.as_str(),
            DEFAULT_BUCKET_NAME,
            now
        ],
    )?;
    fetch_bucket(tx, &bucket_id)
}
