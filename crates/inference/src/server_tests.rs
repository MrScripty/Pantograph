use super::{parse_sidecar_pid, LlamaServer, ServerMode};
use crate::config::DeviceConfig;
use crate::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc;

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

struct ErroringProcessHandle {
    killed: Arc<AtomicBool>,
}

impl ProcessHandle for ErroringProcessHandle {
    fn pid(&self) -> u32 {
        1234
    }

    fn kill(&self) -> Result<(), String> {
        self.killed.store(true, Ordering::SeqCst);
        Ok(())
    }
}

struct ErroringProcessSpawner {
    app_data_dir: PathBuf,
    killed: Arc<AtomicBool>,
}

#[async_trait]
impl ProcessSpawner for ErroringProcessSpawner {
    async fn spawn_sidecar(
        &self,
        _sidecar_name: &str,
        args: &[&str],
    ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
        if let Some(pid_path) = pid_path_arg(args) {
            std::fs::write(pid_path, "1234\n").expect("write pid file");
        }

        let (tx, rx) = mpsc::channel(1);
        tx.send(ProcessEvent::Error("mock startup error".to_string()))
            .await
            .expect("send startup error");

        Ok((
            rx,
            Box::new(ErroringProcessHandle {
                killed: self.killed.clone(),
            }),
        ))
    }

    fn app_data_dir(&self) -> Result<PathBuf, String> {
        Ok(self.app_data_dir.clone())
    }

    fn binaries_dir(&self) -> Result<PathBuf, String> {
        Ok(self.app_data_dir.clone())
    }
}

fn pid_path_arg(args: &[&str]) -> Option<PathBuf> {
    args.windows(2)
        .find(|window| window[0] == "--pid-file")
        .map(|window| PathBuf::from(window[1]))
}

#[tokio::test]
async fn start_sidecar_inference_cleans_process_and_pid_file_on_start_error() {
    let temp = tempfile::tempdir().expect("temp dir");
    let pid_file = temp.path().join(super::SIDECAR_PID_FILE);
    let killed = Arc::new(AtomicBool::new(false));
    let mut server = LlamaServer::new();

    let result = server
        .start_sidecar_inference(
            Arc::new(ErroringProcessSpawner {
                app_data_dir: temp.path().to_path_buf(),
                killed: killed.clone(),
            }),
            "/models/main.gguf",
            None,
            &DeviceConfig {
                device: "auto".to_string(),
                gpu_layers: -1,
            },
            Some(18080),
        )
        .await;

    assert_eq!(
        result,
        Err("llama-server error: mock startup error".to_string())
    );
    assert!(killed.load(Ordering::SeqCst));
    assert!(!pid_file.exists());
    assert!(!server.is_ready());
    assert_eq!(server.mode_info().mode, "none");

    server.stop();
    assert_eq!(server.mode_info().mode, "none");
}
