//! Real gateway, llama.cpp backend and host ports; only process/HTTP I/O is controlled.
use super::resident_resources::{requirements, task};
use super::*;
use crate::runtime_registry::{
    reclaim_runtime_and_reconcile_runtime_registry, sync_runtime_registry,
};
use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use pantograph_runtime_registry::{
    RuntimeAdmissionResourceKind, RuntimeModelResidentEstimate, RuntimeReclaimAction,
    RuntimeRegistration, RuntimeRegistryError, RuntimeResourceDomain, RuntimeResourceDomainBinding,
};
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

struct ControlledProcess {
    events: mpsc::Sender<ProcessEvent>,
    acknowledge: Arc<AtomicBool>,
    kills: Arc<AtomicUsize>,
}
impl ProcessHandle for ControlledProcess {
    fn pid(&self) -> u32 {
        1234
    }
    fn kill(&self) -> Result<(), String> {
        self.kills.fetch_add(1, Ordering::SeqCst);
        if !self.acknowledge.load(Ordering::SeqCst) {
            return Err("controlled kill failure".into());
        }
        // A signal alone is insufficient: explicitly emit owned termination evidence.
        let events = self.events.clone();
        tokio::spawn(async move {
            let _ = events.send(ProcessEvent::Terminated(Some(0))).await;
        });
        Ok(())
    }
}
struct ControlledSpawner {
    dir: tempfile::TempDir,
    acknowledge: Arc<AtomicBool>,
    kills: Arc<AtomicUsize>,
    fail_load: AtomicBool,
    events: Mutex<Option<mpsc::Sender<ProcessEvent>>>,
}
#[async_trait]
impl ProcessSpawner for ControlledSpawner {
    async fn spawn_sidecar(
        &self,
        _: &str,
        _: &[&str],
    ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
        let (tx, rx) = mpsc::channel(8);
        tx.send(if self.fail_load.load(Ordering::SeqCst) {
            ProcessEvent::Error("effectful load failure".into())
        } else {
            ProcessEvent::Stdout(b"HTTP server listening".to_vec())
        })
        .await
        .unwrap();
        *self.events.lock().unwrap() = Some(tx.clone());
        Ok((
            rx,
            Box::new(ControlledProcess {
                events: tx,
                acknowledge: self.acknowledge.clone(),
                kills: self.kills.clone(),
            }),
        ))
    }
    fn app_data_dir(&self) -> Result<std::path::PathBuf, String> {
        Ok(self.dir.path().into())
    }
    fn binaries_dir(&self) -> Result<std::path::PathBuf, String> {
        self.app_data_dir()
    }
}

struct Fixture {
    gateway: Arc<inference::InferenceGateway>,
    spawner: Arc<ControlledSpawner>,
    config: inference::BackendConfig,
    registry: Arc<RuntimeRegistry>,
    http: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.http.abort();
    }
}
impl Fixture {
    async fn new(bytes: Option<u64>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let http = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut request = [0; 4096];
                let _ = stream.read(&mut request).await;
                let _ = stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                    )
                    .await;
            }
        });
        let spawner = Arc::new(ControlledSpawner {
            dir: tempfile::tempdir().unwrap(),
            acknowledge: Arc::new(AtomicBool::new(true)),
            kills: Arc::new(AtomicUsize::new(0)),
            fail_load: AtomicBool::new(false),
            events: Mutex::new(None),
        });
        let gateway = Arc::new(inference::InferenceGateway::new());
        gateway.set_spawner(spawner.clone()).await;
        let registry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(RuntimeRegistration::new("llama_cpp", "llama.cpp"));
        registry.register_runtime(RuntimeRegistration::new("candle", "Candle"));
        registry
            .configure_resource_domain(RuntimeResourceDomain {
                domain_id: "pool".into(),
                total_bytes: 100,
                safety_margin_bytes: 0,
                bindings: ["llama_cpp", "candle"]
                    .into_iter()
                    .map(|id| RuntimeResourceDomainBinding {
                        runtime_id: id.into(),
                        resource_kind: RuntimeAdmissionResourceKind::RamBytes,
                    })
                    .collect(),
            })
            .unwrap();
        if let Some(bytes) = bytes {
            registry
                .configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
                    runtime_id: "llama.cpp".into(),
                    model_id: "model-a".into(),
                    requirements: requirements(bytes),
                }])
                .unwrap();
        }
        let config = inference::BackendConfig {
            model_path: Some("model-a".into()),
            port_override: Some(port),
            ..Default::default()
        };
        Self {
            gateway,
            spawner,
            config,
            registry,
            http,
        }
    }
    fn available(&self) -> u64 {
        self.registry
            .evaluate_reservation(task("candle", "probe", 0))
            .unwrap()
            .observation()
            .resource_domains[0]
            .available_bytes
    }
    async fn reclaim(
        &self,
    ) -> Result<
        pantograph_runtime_registry::RuntimeReclaimDisposition,
        crate::runtime_registry::RuntimeLifecycleCoordinationError,
    > {
        reclaim_runtime_and_reconcile_runtime_registry(
            self.gateway.as_ref(),
            &self.registry,
            "llama_cpp",
        )
        .await
    }
}

#[tokio::test]
async fn llama_host_cold_load_keeps_full_peak_claims_and_idle_resident_estimate() {
    let f = Fixture::new(Some(40)).await;
    let first = f
        .registry
        .acquire_reservation(task("llama_cpp", "first", 50))
        .unwrap();
    let second = f
        .registry
        .acquire_reservation(task("llama_cpp", "second", 50))
        .unwrap();
    f.gateway.start(&f.config).await.unwrap();
    let events = f.spawner.events.lock().unwrap().clone().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        for _ in 0..128 {
            events
                .send(ProcessEvent::Stdout(b"idle log".to_vec()))
                .await
                .unwrap();
        }
    })
    .await
    .expect("the existing output stream must keep draining while idle");
    let port = EmbeddedReservationLifecyclePort::new(f.registry.clone(), f.gateway.clone());
    port.apply_reservation_lifecycle(event(
        first.reservation_id,
        ReservationLifecycleOutcome::RuntimeHostCompleted,
        vec![],
    ))
    .await
    .unwrap();
    assert_eq!(f.available(), 10);
    assert!(f
        .registry
        .acquire_reservation(task("candle", "competing", 20))
        .is_err());
    assert_eq!(f.spawner.kills.load(Ordering::SeqCst), 0);
    // Terminal Retain above kept the other full peak lease. Release that
    // remaining custody explicitly, then reconcile the now idle owner.
    f.registry
        .release_reservation(second.reservation_id)
        .unwrap();
    sync_runtime_registry(f.gateway.as_ref(), &f.registry).await;
    assert_eq!(f.available(), 60);
    assert!(f.gateway.is_ready().await);
    assert_eq!(f.spawner.kills.load(Ordering::SeqCst), 0);
    assert_eq!(
        f.reclaim().await.unwrap().action,
        RuntimeReclaimAction::StopProducer
    );
    assert_eq!(f.available(), 100);
}

#[tokio::test]
async fn llama_host_failed_load_and_failed_stop_keep_unknown_until_owned_termination() {
    let f = Fixture::new(Some(40)).await;
    let lease = f
        .registry
        .acquire_reservation(task("llama_cpp", "failed", 50))
        .unwrap();
    f.spawner.fail_load.store(true, Ordering::SeqCst);
    f.spawner.acknowledge.store(false, Ordering::SeqCst);
    assert!(f.gateway.start(&f.config).await.is_err());
    let port = EmbeddedReservationLifecyclePort::new(f.registry.clone(), f.gateway.clone());
    port.apply_reservation_lifecycle(event(
        lease.reservation_id,
        ReservationLifecycleOutcome::RuntimeHostFailed,
        vec![diagnostic(
            ReservationLifecycleDiagnosticSeverity::Error,
            ReservationLifecycleDiagnosticCode::RuntimeHostFailed,
            "effectful load failed",
        )],
    ))
    .await
    .unwrap_err();
    assert!(f
        .registry
        .acquire_reservation(task("candle", "unknown", 1))
        .is_err());
    let before = f.spawner.kills.load(Ordering::SeqCst);
    assert!(f.reclaim().await.is_err());
    assert!(f.spawner.kills.load(Ordering::SeqCst) > before);
    assert!(f
        .registry
        .snapshot()
        .runtimes
        .iter()
        .any(|r| r.runtime_id == "llama_cpp" && r.resident_resources_uncertain));
    f.spawner.acknowledge.store(true, Ordering::SeqCst);
    assert_eq!(
        f.reclaim().await.unwrap().action,
        RuntimeReclaimAction::StopProducer
    );
    assert_eq!(f.available(), 100);
}

#[tokio::test]
async fn llama_host_delayed_stop_cannot_release_new_generation() {
    let f = Fixture::new(Some(40)).await;
    f.gateway.start(&f.config).await.unwrap();
    sync_runtime_registry(f.gateway.as_ref(), &f.registry).await;
    f.gateway.stop().await.unwrap();
    let stale = f.gateway.resident_lifecycle_snapshots().await.remove(0);
    f.gateway.start(&f.config).await.unwrap();
    sync_runtime_registry(f.gateway.as_ref(), &f.registry).await;
    assert!(matches!(
        stale.publish(&f.registry),
        Err(RuntimeRegistryError::ModelResidencyObservationChanged(_))
    ));
    assert_eq!(f.available(), 60);
    f.reclaim().await.unwrap();
    assert_eq!(f.available(), 100);
}

#[tokio::test]
async fn llama_host_known_zero_is_distinct_from_missing_estimate_and_external_ownership() {
    let zero = Fixture::new(Some(0)).await;
    zero.gateway.start(&zero.config).await.unwrap();
    sync_runtime_registry(zero.gateway.as_ref(), &zero.registry).await;
    assert_eq!(zero.available(), 100);
    zero.reclaim().await.unwrap();
    let missing = Fixture::new(None).await;
    missing.gateway.start(&missing.config).await.unwrap();
    sync_runtime_registry(missing.gateway.as_ref(), &missing.registry).await;
    assert!(missing
        .registry
        .acquire_reservation(task("candle", "unknown", 1))
        .is_err());
    missing.reclaim().await.unwrap();
    assert_eq!(missing.available(), 100);
    let external = inference::BackendConfig {
        external_url: Some(format!(
            "http://127.0.0.1:{}",
            missing.config.port_override.unwrap()
        )),
        ..Default::default()
    };
    missing.gateway.start(&external).await.unwrap();
    sync_runtime_registry(missing.gateway.as_ref(), &missing.registry).await;
    assert_eq!(missing.available(), 100);
    assert!(missing.gateway.is_ready().await);
    assert_eq!(missing.spawner.kills.load(Ordering::SeqCst), 1);
}
