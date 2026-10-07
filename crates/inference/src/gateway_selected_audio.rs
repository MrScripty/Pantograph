//! Audio custody survives caller loss; the selected owner observes completion.
use super::*;

struct CallerCancellation {
    dropped: Arc<AtomicBool>,
    host: InferenceExecutionCancellationHandle,
}
impl crate::InferenceExecutionCancellationSignal for CallerCancellation {
    fn snapshot(&self) -> crate::InferenceExecutionCancellationSnapshot {
        if self.dropped.load(Ordering::Acquire) {
            crate::InferenceExecutionCancellationSnapshot::cancellation_requested(Some(
                "selected audio caller dropped".into(),
            ))
        } else {
            self.host.snapshot()
        }
    }
}
struct CancelOnDrop(Arc<AtomicBool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

impl InferenceGateway {
    /// Execute only the scheduler-selected bounded inline WAV CPU ASR route.
    /// No generic typed execution, prompt fallback or runtime acquisition is used.
    pub async fn execute_selected_audio_with_cancellation(
        &self,
        request: InferenceExecutionRequest,
        target: crate::PumasArtifactLoadTarget,
        decision: crate::BackendExecutionDecision,
        host: InferenceExecutionCancellationHandle,
    ) -> Result<InferenceExecutionResult, GatewayError> {
        crate::selected_audio_execution::SelectedAudioLoad::validate(&request, &target, &decision)
            .await?;
        reject_cancelled_execution_handle("selected audio", &host)?;
        let InferenceExecutionInput::AudioTranscription { request: audio } = &request.input else {
            unreachable!("validated audio input")
        };
        let mut execution = audio.clone();
        execution.extra_options = serde_json::Value::Null;
        let option_diagnostics = typed_request_option_diagnostics(&request, Some("pytorch"));
        self.embedding_replacement.drain().await?;
        let backend = self.backend.clone().write_owned().await;
        reject_cancelled_execution_handle("selected audio", &host)?;
        let replacement = if canonical_backend_key(backend.name()) == "pytorch" {
            None
        } else {
            Some(self.registry.create("pytorch")?)
        };
        let spawner = self.spawner.read().await.clone();
        let runtime_config = self.current_runtime_config.clone();
        let backend_name = self.current_backend_name.clone();
        let lifecycle = self.runtime_lifecycle.clone();
        let embedding_mode = self.embedding_mode.clone();
        let reranking_mode = self.reranking_mode.clone();
        let external_mode = self.external_mode.clone();
        let old_pytorch_release = self.pytorch_release_confirmed.clone();
        let old_llamacpp_release = self.llamacpp_release_confirmed.clone();
        let pytorch_release = self.pytorch_release_confirmed.clone();
        let pytorch_owned = self.pytorch_ever_owned.clone();
        let instance = self.allocate_runtime_instance_id("pytorch");
        let dropped = Arc::new(AtomicBool::new(false));
        let _cancel_on_drop = CancelOnDrop(dropped.clone());
        let cancellation =
            InferenceExecutionCancellationHandle::with_signal(Arc::new(CallerCancellation {
                dropped,
                host,
            }));
        // Ownership of the write guard moves into the operation. Dropping this
        // caller cannot admit another load until the actual request completes.
        let owner = tokio::spawn(async move {
            let mut backend = backend;
            reject_cancelled_execution_handle("selected audio", &cancellation)?;
            if let Some(replacement) = replacement {
                let old = canonical_backend_key(backend.name());
                if let Err(error) = backend.stop().await {
                    lifecycle.write().await.last_error = Some(error.to_string());
                    return Err(GatewayError::Backend(error));
                }
                if old == "pytorch" {
                    old_pytorch_release.store(true, Ordering::Relaxed);
                }
                if old == "llama_cpp" {
                    old_llamacpp_release.store(true, Ordering::Relaxed);
                }
                *backend = replacement;
                *backend_name.write().await = backend.name().to_owned();
                *runtime_config.write().await = None;
                *lifecycle.write().await = RuntimeLifecycleSnapshot {
                    runtime_id: Some("pytorch".into()),
                    ..Default::default()
                };
            }
            reject_cancelled_execution_handle("selected audio", &cancellation)?;
            pytorch_owned.store(true, Ordering::Relaxed);
            pytorch_release.store(false, Ordering::Relaxed);
            let outcome = match backend
                .load_selected_audio(&request, &target, &decision, spawner, cancellation.clone())
                .await
            {
                Ok(outcome) => outcome,
                Err(error) => {
                    let ready = backend.is_ready();
                    if !ready {
                        *runtime_config.write().await = None;
                    }
                    let mut snapshot = lifecycle.write().await;
                    snapshot.active = ready;
                    snapshot.last_error = Some(error.to_string());
                    if !ready {
                        snapshot.runtime_instance_id = None;
                    }
                    return Err(GatewayError::Backend(error));
                }
            };
            *runtime_config.write().await = Some(BackendConfig {
                model_path: Some(PathBuf::from(&target.local_load_path)),
                model_name: Some(target.model_ref.model_id.clone()),
                device: Some(BackendStartupDeviceIntent::CanonicalDevice(
                    crate::InferenceDeviceId::parse("cpu").expect("canonical CPU"),
                )),
                ..Default::default()
            });
            *embedding_mode.write().await = false;
            *reranking_mode.write().await = false;
            *external_mode.write().await = false;
            let mut snapshot = lifecycle.write().await;
            let runtime_instance_id = if outcome.runtime_reused == Some(true) {
                snapshot.runtime_instance_id.clone().unwrap_or(instance)
            } else {
                instance
            };
            *snapshot = RuntimeLifecycleSnapshot {
                runtime_id: Some("pytorch".into()),
                runtime_instance_id: Some(runtime_instance_id),
                runtime_reused: outcome.runtime_reused,
                lifecycle_decision_reason: Some("scheduler_selected_audio_package_loaded".into()),
                active: backend.is_ready(),
                ..Default::default()
            };
            drop(snapshot);
            reject_cancelled_execution_handle("selected audio", &cancellation)?;
            let result = backend
                .selected_audio(
                    execution,
                    request.request_id.as_deref().expect("validated request id"),
                    &target,
                    &decision,
                    cancellation.clone(),
                )
                .await?;
            reject_cancelled_execution_handle("selected audio", &cancellation)?;
            if result
                .duration_seconds
                .is_some_and(|v| !v.is_finite() || v < 0.0)
                || result.segments.iter().any(|s| {
                    s.start_seconds.is_some_and(|v| !v.is_finite() || v < 0.0)
                        || s.end_seconds.is_some_and(|v| !v.is_finite() || v < 0.0)
                })
            {
                return Err(GatewayError::Backend(BackendError::Inference(
                    "selected audio returned invalid timing".into(),
                )));
            }
            Ok(InferenceExecutionResult::AudioTranscription {
                result,
                option_diagnostics,
            })
        });
        owner.await.map_err(|error| {
            GatewayError::Backend(BackendError::Inference(format!(
                "selected audio owner terminated: {error}"
            )))
        })?
    }
}
