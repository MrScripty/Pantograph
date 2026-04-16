//! Backend-owned recovery restart planning helpers.
//!
//! Hosts may own event emission and app-state wiring, but restart-plan
//! decisions belong in backend Rust so recovery orchestration does not drift
//! across wrappers.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Recovery strategy to use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStrategy {
    Restart,
    AlternatePort,
    CleanRestart,
    Abandon,
}

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
    let mut restart_config =
        restart_config.ok_or(RecoveryRestartPlanError::MissingRuntimeConfig)?;
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

pub fn recovery_backoff(base_ms: u64, max_ms: u64, attempt: u32) -> Duration {
    let delay_ms = base_ms.saturating_mul(1u64 << attempt.min(10));
    Duration::from_millis(delay_ms.min(max_ms))
}

pub fn recovery_strategy_for_attempt(attempt: u32, try_alternate_port: bool) -> RecoveryStrategy {
    if attempt == 0 {
        RecoveryStrategy::Restart
    } else if attempt == 1 && try_alternate_port {
        RecoveryStrategy::AlternatePort
    } else if attempt >= 2 {
        RecoveryStrategy::CleanRestart
    } else {
        RecoveryStrategy::Restart
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        build_recovery_restart_plan, recovery_backoff, recovery_strategy_for_attempt,
        RecoveryRestartPlanError, RecoveryStrategy,
    };

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

    #[test]
    fn recovery_backoff_uses_exponential_growth_with_cap() {
        assert_eq!(
            recovery_backoff(1_000, 30_000, 0),
            Duration::from_millis(1_000)
        );
        assert_eq!(
            recovery_backoff(1_000, 30_000, 1),
            Duration::from_millis(2_000)
        );
        assert_eq!(
            recovery_backoff(1_000, 30_000, 6),
            Duration::from_millis(30_000)
        );
    }

    #[test]
    fn recovery_strategy_for_attempt_prefers_alternate_port_on_second_try() {
        assert_eq!(
            recovery_strategy_for_attempt(0, true),
            RecoveryStrategy::Restart
        );
        assert_eq!(
            recovery_strategy_for_attempt(1, true),
            RecoveryStrategy::AlternatePort
        );
        assert_eq!(
            recovery_strategy_for_attempt(2, true),
            RecoveryStrategy::CleanRestart
        );
        assert_eq!(
            recovery_strategy_for_attempt(1, false),
            RecoveryStrategy::Restart
        );
    }
}
