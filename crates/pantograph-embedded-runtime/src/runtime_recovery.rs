//! Backend-owned recovery restart planning helpers.
//!
//! Hosts may own event emission and app-state wiring, but restart-plan
//! decisions belong in backend Rust so recovery orchestration does not drift
//! across wrappers.

#[derive(Debug, Clone)]
pub struct RecoveryRestartPlan {
    pub restart_config: inference::BackendConfig,
    pub restart_embedding: bool,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum RecoveryRestartPlanError {
    #[error("No active runtime configuration available for restart")]
    MissingRuntimeConfig,
}

pub fn build_recovery_restart_plan(
    restart_config: Option<inference::BackendConfig>,
    port_override: Option<u16>,
    has_embedding_model: bool,
    embedding_runs_parallel: bool,
) -> Result<RecoveryRestartPlan, RecoveryRestartPlanError> {
    let mut restart_config = restart_config.ok_or(RecoveryRestartPlanError::MissingRuntimeConfig)?;
    if let Some(port_override) = port_override {
        restart_config.port_override = Some(port_override);
    }

    let restart_embedding = has_embedding_model
        && embedding_runs_parallel
        && restart_config.external_url.is_none()
        && !restart_config.embedding_mode
        && !restart_config.reranking_mode;

    Ok(RecoveryRestartPlan {
        restart_config,
        restart_embedding,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_recovery_restart_plan, RecoveryRestartPlanError};

    #[test]
    fn build_recovery_restart_plan_sets_backend_owned_port_contract() {
        let plan = build_recovery_restart_plan(
            Some(inference::BackendConfig {
                model_name: Some("llava:13b".to_string()),
                ..inference::BackendConfig::default()
            }),
            Some(18080),
            true,
            true,
        )
        .expect("restart plan should build");

        assert_eq!(plan.restart_config.port_override, Some(18080));
        assert_eq!(plan.restart_config.model_name.as_deref(), Some("llava:13b"));
        assert!(plan.restart_embedding);
    }

    #[test]
    fn build_recovery_restart_plan_requires_runtime_config() {
        let error = build_recovery_restart_plan(None, None, true, true)
            .expect_err("missing restart config should fail");

        assert_eq!(error, RecoveryRestartPlanError::MissingRuntimeConfig);
    }

    #[test]
    fn build_recovery_restart_plan_skips_embedding_restart_for_external_runtime() {
        let plan = build_recovery_restart_plan(
            Some(inference::BackendConfig {
                external_url: Some("http://127.0.0.1:1234".to_string()),
                ..inference::BackendConfig::default()
            }),
            None,
            true,
            true,
        )
        .expect("restart plan should build");

        assert!(!plan.restart_embedding);
    }

    #[test]
    fn build_recovery_restart_plan_skips_embedding_restart_for_sequential_mode() {
        let plan = build_recovery_restart_plan(
            Some(inference::BackendConfig {
                model_name: Some("llava:13b".to_string()),
                ..inference::BackendConfig::default()
            }),
            None,
            true,
            false,
        )
        .expect("restart plan should build");

        assert!(!plan.restart_embedding);
    }

    #[test]
    fn build_recovery_restart_plan_skips_embedding_restart_for_embedding_runtime() {
        let plan = build_recovery_restart_plan(
            Some(inference::BackendConfig {
                embedding_mode: true,
                ..inference::BackendConfig::default()
            }),
            None,
            true,
            true,
        )
        .expect("restart plan should build");

        assert!(!plan.restart_embedding);
    }
}
