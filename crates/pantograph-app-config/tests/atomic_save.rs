//! Linux write-failure regression against the real persisted configuration API.
#![cfg(target_os = "linux")]

use pantograph_app_config::AppConfig;

#[tokio::test]
async fn failed_config_save_preserves_the_complete_previous_capacity_configuration() {
    let directory = tempfile::TempDir::new().unwrap();
    let config = AppConfig {
        runtime_resources: serde_json::from_str(include_str!(
            "../../pantograph-runtime-registry/tests/fixtures/startup_shared_resource_config.json"
        ))
        .unwrap(),
        ..Default::default()
    };
    config.save(&directory.path().to_path_buf()).await.unwrap();
    let config_path = directory.path().join("config.json");
    let previous = std::fs::read(&config_path).unwrap();
    assert!(previous.len() < 2048);
    // A per-child file-size limit deterministically interrupts the real save.
    // It neither changes directory permissions nor affects the parent process.
    let output = std::process::Command::new("/bin/bash")
        .args(["-c", "ulimit -f 2; trap '' XFSZ; exec \"$1\" --ignored --exact config_save_failure_child --nocapture", "config-save-failure"])
        .arg(std::env::current_exe().unwrap())
        .env("PANTOGRAPH_CONFIG_FAILURE_TEST_DIR", directory.path())
        .output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        std::fs::read(&config_path).unwrap() == previous,
        "failed publication must not truncate the previous settings"
    );
    let (restored, _) = AppConfig::load_with_runtime_registry(directory.path())
        .await
        .unwrap();
    assert_eq!(restored.runtime_resources, config.runtime_resources);
}

#[tokio::test]
#[ignore = "subprocess helper invoked by the real write-failure regression"]
async fn config_save_failure_child() {
    let directory = std::path::PathBuf::from(
        std::env::var_os("PANTOGRAPH_CONFIG_FAILURE_TEST_DIR")
            .expect("parent supplies child directory"),
    );
    let mut config = AppConfig::load(&directory).await.unwrap();
    config.models.vlm_model_path = Some("x".repeat(64 * 1024));
    assert!(
        config.save(&directory).await.is_err(),
        "the limited child must report write failure"
    );
}

#[tokio::test]
async fn successful_atomic_save_preserves_file_mode_and_publishes_complete_settings() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::TempDir::new().unwrap();
    let mut config = AppConfig::default();
    config.save(&directory.path().to_path_buf()).await.unwrap();
    let path = directory.path().join("config.json");
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    config.models.vlm_model_path = Some("/models/new-generation.gguf".into());
    config.save(&directory.path().to_path_buf()).await.unwrap();
    assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode(), mode);
    let restored = AppConfig::load(directory.path()).await.unwrap();
    assert_eq!(restored.models.vlm_model_path, config.models.vlm_model_path);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}
