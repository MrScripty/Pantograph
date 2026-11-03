use std::path::Path;

use pantograph_workflow_service::{ArtifactPolicy, ArtifactStore, ArtifactStoreError};

pub(super) fn open_artifact_store(root_dir: &Path) -> Result<ArtifactStore, ArtifactStoreError> {
    ArtifactStore::open_with_default_policy(root_dir, default_artifact_policy())
}

fn default_artifact_policy() -> ArtifactPolicy {
    ArtifactPolicy {
        policy_id: "artifact-global-default".to_string(),
        policy_version: 1,
        ttl_seconds: None,
        max_disk_bytes: None,
        max_memory_bytes: Some(256 * 1024 * 1024),
        max_single_artifact_bytes: Some(128 * 1024 * 1024),
        spill_threshold_bytes: Some(8 * 1024 * 1024),
        delete_on_consume: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_workflow_service::{
        ArtifactAttribution, ArtifactConsumeAcknowledgementRequest, ArtifactPayloadKind,
        ArtifactReadRequest, ArtifactWriteRequest, WorkflowErrorCode, WorkflowService,
    };

    fn saved_policy() -> ArtifactPolicy {
        ArtifactPolicy {
            policy_id: "user-retention".to_string(),
            policy_version: 7,
            ttl_seconds: Some(60),
            max_disk_bytes: Some(10),
            max_memory_bytes: Some(4),
            max_single_artifact_bytes: Some(8),
            spill_threshold_bytes: Some(4),
            delete_on_consume: true,
        }
    }

    fn write_request(artifact_id: &str, body: &[u8]) -> ArtifactWriteRequest {
        ArtifactWriteRequest {
            artifact_id: Some(artifact_id.to_string()),
            payload_kind: ArtifactPayloadKind::GenericBinary,
            media_type: "application/octet-stream".to_string(),
            format: None,
            attribution: ArtifactAttribution {
                workflow_run_id: "retention-run".to_string(),
                workflow_id: None,
                workflow_version_id: None,
                node_id: None,
                port_id: None,
                model_id: None,
                runtime_id: None,
            },
            artifact_role: None,
            parent_artifact_id: None,
            revision_index: None,
            body: body.to_vec(),
        }
    }

    fn open_service(root_dir: &Path) -> WorkflowService {
        WorkflowService::new()
            .with_artifact_store(open_artifact_store(root_dir).expect("desktop artifact startup"))
    }

    #[test]
    fn desktop_reopen_preserves_saved_policy_and_enforces_its_limits() {
        let temp = tempfile::tempdir().expect("temp dir");
        let service = open_service(temp.path());
        assert_eq!(
            service.artifact_policy().expect("default policy"),
            default_artifact_policy()
        );
        let saved = saved_policy();
        assert_eq!(
            service
                .update_artifact_policy(saved.clone())
                .expect("save settings through service"),
            saved
        );
        service
            .write_artifact(write_request("retained", b"123456"))
            .expect("retain artifact");
        drop(service);

        let reopened = open_service(temp.path());
        assert_eq!(reopened.artifact_policy().expect("saved policy"), saved);
        let stats = reopened.artifact_store_stats().expect("reopened stats");
        assert_eq!(stats.retained_body_bytes, 6);
        assert_eq!(stats.memory_cache_body_bytes, 0);
        let read = reopened
            .read_artifact_body(ArtifactReadRequest {
                artifact_id: "retained".to_string(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .expect("read retained body");
        assert_eq!(read.body, b"123456");
        assert!(read.response.complete);

        let disk_limit = reopened
            .write_artifact(write_request("over-disk", b"12345"))
            .expect_err("saved disk cap must still apply");
        assert_eq!(disk_limit.code(), WorkflowErrorCode::InvalidRequest);
        assert!(disk_limit.to_string().contains("disk"));
        let single_limit = reopened
            .write_artifact(write_request("over-single", b"123456789"))
            .expect_err("saved single-artifact cap must still apply");
        assert_eq!(single_limit.code(), WorkflowErrorCode::InvalidRequest);
        assert!(single_limit
            .to_string()
            .contains("max_single_artifact_bytes"));
        let consumed = reopened
            .acknowledge_artifact_consumed(ArtifactConsumeAcknowledgementRequest {
                artifact_id: "retained".to_string(),
                consumer_id: "inspector".to_string(),
            })
            .expect("consume retained artifact");
        assert!(!consumed.retained_after_consume);
        drop(reopened);

        let reopened_again = open_service(temp.path());
        assert_eq!(
            reopened_again.artifact_policy().expect("saved policy"),
            saved
        );
        assert_eq!(
            reopened_again
                .artifact_store_stats()
                .expect("stats")
                .retained_body_count,
            0
        );
    }

    #[test]
    fn desktop_reopen_preserves_ttl_cleanup() {
        let temp = tempfile::tempdir().expect("temp dir");
        let service = open_service(temp.path());
        service
            .update_artifact_policy(saved_policy())
            .expect("save retention settings");
        service
            .write_artifact(write_request("expires", b"body"))
            .expect("retain artifact");
        service
            .write_artifact(write_request("also-expires", b"body"))
            .expect("retain second artifact");
        drop(service);

        let reopened = open_service(temp.path());
        assert_eq!(
            reopened
                .artifact_store_stats()
                .expect("stats")
                .memory_cache_body_bytes,
            4
        );
        assert_eq!(
            reopened
                .apply_artifact_store_retention_cleanup(u64::MAX)
                .expect("cleanup at a time beyond TTL"),
            2
        );
        assert_eq!(
            reopened
                .artifact_store_stats()
                .expect("stats")
                .retained_body_count,
            0
        );
    }
}
