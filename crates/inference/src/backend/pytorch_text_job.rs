//! One retained producer completion, shared by the stream and residency owner.
use super::{BackendError, ChatChunk};
use futures_util::{
    future::{BoxFuture, Shared},
    FutureExt, Stream,
};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tokio::sync::{mpsc, watch};

type OwnedTextStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>;

type Completion = Shared<BoxFuture<'static, Arc<Result<(), BackendError>>>>;
#[derive(Default)]
pub(super) struct TextJobs(Mutex<Option<TextJob>>);
#[derive(Clone)]
struct TextJob {
    cancel: watch::Sender<bool>,
    completion: Completion,
}

fn copy_error(error: &BackendError) -> BackendError {
    match error {
        BackendError::NotReady => BackendError::NotReady,
        BackendError::NotRunning(s) => BackendError::NotRunning(s.clone()),
        BackendError::StartupFailed(s) => BackendError::StartupFailed(s.clone()),
        BackendError::Config(s) => BackendError::Config(s.clone()),
        BackendError::Inference(s) => BackendError::Inference(s.clone()),
        BackendError::Cancelled(s) => BackendError::Cancelled(s.clone()),
        BackendError::OutOfMemory(s) => BackendError::OutOfMemory(s.clone()),
        BackendError::ManagedBinary(s) => BackendError::ManagedBinary(s.clone()),
        BackendError::Unknown(s) => BackendError::Unknown(s.clone()),
        BackendError::Http(e) => BackendError::Inference(e.to_string()),
    }
}
pub(super) struct Producer {
    tx: mpsc::Sender<ChatChunk>,
    cancel: watch::Receiver<bool>,
}
impl Producer {
    pub(super) fn check(&self) -> Result<(), BackendError> {
        if *self.cancel.borrow() {
            Err(BackendError::Cancelled("PyTorch text job cancelled".into()))
        } else {
            Ok(())
        }
    }
    pub(super) fn send(&mut self, chunk: ChatChunk) -> Result<(), BackendError> {
        self.check()?;
        tokio::runtime::Handle::current().block_on(async {
            tokio::select! {
                biased;
                _ = self.cancel.changed() => Err(BackendError::Cancelled("PyTorch text job cancelled".into())),
                result = self.tx.send(chunk) => result.map_err(|_| BackendError::Cancelled("PyTorch text receiver dropped".into())),
            }
        })
    }
}
impl TextJobs {
    pub(super) fn spawn(
        &self,
        run: impl FnOnce(Producer) -> Result<(), BackendError> + Send + 'static,
    ) -> Result<OwnedTextStream, BackendError> {
        let mut slot = self.0.lock().unwrap();
        if slot
            .as_ref()
            .is_some_and(|job| job.completion.peek().is_none())
        {
            return Err(BackendError::Inference(
                "PyTorch text job slot occupied".into(),
            ));
        }
        let (tx, rx) = mpsc::channel(32);
        let (cancel, cancellation) = watch::channel(false);
        let handle = tokio::task::spawn_blocking(move || {
            run(Producer {
                tx,
                cancel: cancellation,
            })
        });
        let completion = async move {
            Arc::new(handle.await.unwrap_or_else(|e| {
                Err(BackendError::Inference(format!(
                    "PyTorch text producer join failed: {e}"
                )))
            }))
        }
        .boxed()
        .shared();
        let job = TextJob { cancel, completion };
        *slot = Some(job.clone());
        Ok(Box::pin(TextStream {
            rx,
            job,
            finished: false,
        }))
    }
    pub(super) async fn drain(&self, cancel: bool) -> Result<(), BackendError> {
        let job = self.0.lock().unwrap().clone();
        if let Some(job) = job {
            if cancel {
                job.cancel.send_replace(true);
            }
            let result = job.completion.await;
            // Cancellation is expected for explicit stop and receiver loss. Other
            // producer failures remain visible to lifecycle callers as well.
            match result.as_ref() {
                Ok(()) | Err(BackendError::Cancelled(_)) => Ok(()),
                Err(error) => Err(copy_error(error)),
            }
        } else {
            Ok(())
        }
    }
}
struct TextStream {
    rx: mpsc::Receiver<ChatChunk>,
    job: TextJob,
    finished: bool,
}
impl Drop for TextStream {
    fn drop(&mut self) {
        if !self.finished {
            self.job.cancel.send_replace(true);
        }
    }
}
impl Stream for TextStream {
    type Item = Result<ChatChunk, BackendError>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.finished {
            return Poll::Ready(None);
        }
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => return Poll::Ready(Some(Ok(chunk))),
            Poll::Pending => return Poll::Pending,
            Poll::Ready(None) => {}
        }
        match std::future::Future::poll(Pin::new(&mut self.job.completion), cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.finished = true;
                Poll::Ready(Some(match result.as_ref() {
                    Ok(()) if *self.job.cancel.borrow() => {
                        Err(BackendError::Cancelled("PyTorch text job cancelled".into()))
                    }
                    Ok(()) => Ok(ChatChunk {
                        content: None,
                        done: true,
                        usage: None,
                        cache_handle_id: None,
                    }),
                    Err(error) => Err(copy_error(error)),
                }))
            }
        }
    }
}
