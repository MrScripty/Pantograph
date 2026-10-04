//! Candidate loading precedes retirement; custody survives loss of the caller.
use super::*;
use futures_util::{future::BoxFuture, future::Shared, FutureExt};
use std::sync::{atomic::AtomicBool, Mutex};
use tokio::sync::{oneshot, OwnedRwLockWriteGuard};

type Completion = Shared<BoxFuture<'static, Result<(), String>>>;
#[cfg(test)]
type PublicationHook = Arc<dyn Fn() -> BoxFuture<'static, ()> + Send + Sync>;

type BackendGuard = OwnedRwLockWriteGuard<Box<dyn InferenceBackend>>;

#[derive(Default)]
pub(super) struct ReplacementCustody {
    completion: Mutex<Option<Completion>>,
    #[cfg(test)]
    pub(super) after_publication: Mutex<Option<PublicationHook>>,
}

impl ReplacementCustody {
    pub(super) async fn drain(&self) -> Result<(), GatewayError> {
        let completion = self.completion.lock().unwrap().clone();
        if let Some(completion) = completion {
            let outcome = completion.clone().await;
            self.retire_completed(&completion);
            // Drain establishes termination for a new operation. The failed
            // replacement reports its JoinError to its own requesting caller.
            if let Err(error) = outcome {
                log::warn!("drained terminated replacement: {error}");
            }
        }
        Ok(())
    }

    fn retire_completed(&self, completion: &Completion) {
        let mut current = self.completion.lock().unwrap();
        // A delayed observer of an older join must never remove new live custody.
        if completion.peek().is_some()
            && current
                .as_ref()
                .is_some_and(|current| current.ptr_eq(completion))
        {
            *current = None;
        }
    }
}

struct CallerCancellation {
    dropped: Arc<AtomicBool>,
    host: InferenceExecutionCancellationHandle,
}

impl crate::InferenceExecutionCancellationSignal for CallerCancellation {
    fn snapshot(&self) -> crate::InferenceExecutionCancellationSnapshot {
        if self.dropped.load(Ordering::Acquire) {
            crate::InferenceExecutionCancellationSnapshot::cancellation_requested(Some(
                "embedding replacement caller dropped".into(),
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
    pub(super) async fn replace_selected_embedding(
        &self,
        mut backend: BackendGuard,
        request: InferenceExecutionRequest,
        target: crate::PumasArtifactLoadTarget,
        decision: crate::BackendExecutionDecision,
        host: InferenceExecutionCancellationHandle,
        config: BackendConfig,
    ) -> Result<BackendGuard, GatewayError> {
        let mut candidate = self.registry.create("candle")?;
        let dropped = Arc::new(AtomicBool::new(false));
        let _cancel_on_drop = CancelOnDrop(dropped.clone());
        let cancellation =
            InferenceExecutionCancellationHandle::with_signal(Arc::new(CallerCancellation {
                dropped,
                host,
            }));
        let name = self.current_backend_name.clone();
        let current_config = self.current_runtime_config.clone();
        let embedding = self.embedding_mode.clone();
        let reranking = self.reranking_mode.clone();
        let external = self.external_mode.clone();
        let lifecycle = self.runtime_lifecycle.clone();
        let sequence = self.runtime_instance_sequence.clone();
        #[cfg(test)]
        let after_publication = self
            .embedding_replacement
            .after_publication
            .lock()
            .unwrap()
            .clone();
        let (sender, receiver) = oneshot::channel();
        let task = tokio::spawn(async move {
            let result = async {
                let outcome = candidate
                    .load_selected_embedding_with_cancellation(
                        &request,
                        &target,
                        &decision,
                        cancellation.clone(),
                    )
                    .await?;
                // Acquire every publication guard before the irreversible retirement.
                // Until admission below, cancellation/failure leaves the resident intact.
                let mut name = name.write_owned().await;
                let mut current_config = current_config.write_owned().await;
                let mut embedding = embedding.write_owned().await;
                let mut reranking = reranking.write_owned().await;
                let mut external = external.write_owned().await;
                let mut lifecycle = lifecycle.write_owned().await;
                reject_cancelled_execution_handle(
                    "embedding replacement admission",
                    &cancellation,
                )?;
                // Retirement and publication are supervised to completion, even if the
                // caller disappears or the host cancels after this admission point.
                backend.stop().await?;
                std::mem::swap(&mut *backend, &mut candidate);
                *name = backend.name().to_owned();
                *current_config = Some(config);
                *embedding = true;
                *reranking = false;
                *external = false;
                *lifecycle = RuntimeLifecycleSnapshot {
                    runtime_id: Some("candle".into()),
                    runtime_instance_id: Some(format!(
                        "candle-{}",
                        sequence.fetch_add(1, Ordering::Relaxed)
                    )),
                    runtime_reused: outcome.runtime_reused,
                    lifecycle_decision_reason: outcome.lifecycle_decision_reason,
                    active: backend.is_ready(),
                    ..Default::default()
                };
                #[cfg(test)]
                if let Some(hook) = after_publication {
                    hook().await;
                }
                Ok::<(), GatewayError>(())
            }
            .await;
            // In particular, this observes Candle's actual blocking load join on
            // candidate failure/cancellation before releasing exclusive residency.
            let cleanup = if result.is_err() {
                candidate.stop().await
            } else {
                Ok(())
            };
            let result = result
                .and_then(|()| cleanup.map_err(GatewayError::from))
                .map(|()| backend);
            let _ = sender.send(result);
        });
        let completion = async move {
            task.await
                .map_err(|error| format!("embedding replacement supervisor: {error}"))
        }
        .boxed()
        .shared();
        *self.embedding_replacement.completion.lock().unwrap() = Some(completion.clone());
        let outcome = completion.clone().await;
        self.embedding_replacement.retire_completed(&completion);
        outcome.map_err(GatewayError::SwitchFailed)?;
        receiver
            .await
            .map_err(|error| GatewayError::SwitchFailed(error.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stale_observer_cannot_retire_new_live_completion() {
        let custody = ReplacementCustody::default();
        let older: Completion = async { Ok(()) }.boxed().shared();
        older.clone().await.unwrap();
        let (release, pending) = oneshot::channel();
        let live: Completion = async move { pending.await.map_err(|error| error.to_string()) }
            .boxed()
            .shared();
        *custody.completion.lock().unwrap() = Some(live.clone());
        custody.retire_completed(&older);
        custody.retire_completed(&live);
        assert!(custody
            .completion
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .ptr_eq(&live));
        release.send(()).unwrap();
        custody.drain().await.unwrap();
        assert!(custody.completion.lock().unwrap().is_none());
    }
}
