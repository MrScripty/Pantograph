use super::{parse_sidecar_pid, LlamaServer, ServerMode};
use crate::config::DeviceConfig;

#[test]
fn parse_sidecar_pid_accepts_legacy_plain_pid() {
    assert_eq!(parse_sidecar_pid("12345\n"), Some(12345));
}

#[test]
fn parse_sidecar_pid_accepts_structured_pid_record() {
    let record = r#"{
        "schema_version": 1,
        "pid": 12345,
        "started_at_ms": 1710000000000,
        "owner": "pantograph-tauri",
        "owner_version": "0.0.0",
        "mode": "llama.cpp.inference",
        "executable": "/tmp/llama-server"
    }"#;

    assert_eq!(parse_sidecar_pid(record), Some(12345));
}

#[test]
fn base_url_reflects_sidecar_port_override() {
    let mut server = LlamaServer::new();
    server.set_test_runtime_state(
        ServerMode::SidecarInference {
            port: 18080,
            model_path: "/models/main.gguf".to_string(),
            mmproj_path: None,
            device: DeviceConfig {
                device: "auto".to_string(),
                gpu_layers: -1,
            },
        },
        true,
    );

    assert_eq!(server.base_url().as_deref(), Some("http://127.0.0.1:18080"));
}

#[test]
fn kv_slot_save_dir_is_scoped_under_app_data_dir() {
    let dir = super::kv_slot_save_dir(std::path::Path::new("/tmp/pantograph"));
    assert_eq!(
        dir,
        std::path::PathBuf::from("/tmp/pantograph").join("llama-kv-slots")
    );
}

#[test]
fn inference_runtime_matcher_requires_matching_port() {
    let mut server = LlamaServer::new();
    let device = DeviceConfig {
        device: "Vulkan0".to_string(),
        gpu_layers: 40,
    };
    server.set_test_runtime_state(
        ServerMode::SidecarInference {
            port: 11434,
            model_path: "/models/main.gguf".to_string(),
            mmproj_path: Some("/models/vision.mmproj".to_string()),
            device: device.clone(),
        },
        true,
    );

    assert!(server.matches_inference_runtime(
        "/models/main.gguf",
        Some("/models/vision.mmproj"),
        &device,
        Some(11434),
    ));
    assert!(!server.matches_inference_runtime(
        "/models/main.gguf",
        Some("/models/vision.mmproj"),
        &device,
        Some(18080),
    ));
}
