use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use node_engine::{EventSink, TaskExecutor};
use rustler::{Atom, Encoder, NifResult, OwnedEnv};
use tokio::sync::oneshot;

use crate::atoms;
use crate::workflow_event_contract::serialize_workflow_event_json;

type PendingCallbackSender = oneshot::Sender<Result<String, String>>;
type PendingCallbackMap = HashMap<String, PendingCallbackSender>;

/// Pending callback channels for bridging node execution to BEAM.
static PENDING_CALLBACKS: LazyLock<Mutex<PendingCallbackMap>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Counter for generating unique callback IDs.
static CALLBACK_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// TaskExecutor that bridges node execution to Elixir via callback NIFs.
pub(crate) struct ElixirCallbackTaskExecutor {
    pid: rustler::LocalPid,
    owned_env: Arc<Mutex<OwnedEnv>>,
    timeout_secs: u64,
}

impl ElixirCallbackTaskExecutor {
    pub(crate) fn new(pid: rustler::LocalPid) -> Self {
        Self {
            pid,
            owned_env: Arc::new(Mutex::new(OwnedEnv::new())),
            timeout_secs: 300,
        }
    }

    pub(crate) fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }
}

#[async_trait::async_trait]
impl TaskExecutor for ElixirCallbackTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &graph_flow::Context,
        _extensions: &node_engine::ExecutorExtensions,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        let callback_id = format!(
            "cb-{}",
            CALLBACK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );

        let (tx, rx) = oneshot::channel();

        {
            let mut callbacks = PENDING_CALLBACKS.lock().map_err(|e| {
                node_engine::NodeEngineError::ExecutionFailed(format!("Lock poisoned: {}", e))
            })?;
            callbacks.insert(callback_id.clone(), tx);
        }

        let inputs_json = serde_json::to_string(&inputs)?;

        let pid = self.pid;
        let cb_id = callback_id.clone();
        let t_id = task_id.to_string();
        let owned_env = self.owned_env.clone();
        tokio::task::spawn_blocking(move || {
            let mut env = owned_env
                .lock()
                .map_err(|e| format!("Env lock poisoned: {}", e))?;
            env.send_and_clear(&pid, |env| {
                let msg = (
                    atoms::node_execute().encode(env),
                    cb_id.encode(env),
                    t_id.encode(env),
                    inputs_json.encode(env),
                );
                msg.encode(env)
            })
            .map_err(|_| "Failed to send to Elixir PID".to_string())
        })
        .await
        .map_err(|e| {
            node_engine::NodeEngineError::ExecutionFailed(format!("Send thread error: {}", e))
        })?
        .map_err(node_engine::NodeEngineError::ExecutionFailed)?;

        let result = tokio::time::timeout(std::time::Duration::from_secs(self.timeout_secs), rx)
            .await
            .map_err(|_| {
                let mut callbacks = PENDING_CALLBACKS.lock().unwrap_or_else(|e| e.into_inner());
                callbacks.remove(&callback_id);
                node_engine::NodeEngineError::ExecutionFailed(format!(
                    "Callback timeout for task '{}'",
                    task_id
                ))
            })?
            .map_err(|_| {
                node_engine::NodeEngineError::ExecutionFailed(format!(
                    "Callback channel dropped for task '{}'",
                    task_id
                ))
            })?;

        match result {
            Ok(json_str) => {
                let outputs: HashMap<String, serde_json::Value> = serde_json::from_str(&json_str)?;
                Ok(outputs)
            }
            Err(err_msg) => Err(node_engine::NodeEngineError::ExecutionFailed(err_msg)),
        }
    }
}

/// Task executor that tries CoreTaskExecutor first, then falls back to Elixir.
pub(crate) struct CoreFirstExecutor {
    core: Arc<node_engine::CoreTaskExecutor>,
    elixir: Arc<ElixirCallbackTaskExecutor>,
}

impl CoreFirstExecutor {
    pub(crate) fn new(
        core: node_engine::CoreTaskExecutor,
        elixir: ElixirCallbackTaskExecutor,
    ) -> Self {
        Self {
            core: Arc::new(core),
            elixir: Arc::new(elixir),
        }
    }
}

#[async_trait::async_trait]
impl TaskExecutor for CoreFirstExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        context: &graph_flow::Context,
        extensions: &node_engine::ExecutorExtensions,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        match self
            .core
            .execute_task(task_id, inputs.clone(), context, extensions)
            .await
        {
            Err(node_engine::NodeEngineError::ExecutionFailed(ref msg))
                if msg.contains("requires host-specific executor") =>
            {
                self.elixir
                    .execute_task(task_id, inputs, context, extensions)
                    .await
            }
            other => other,
        }
    }
}

/// EventSink that sends events to an Elixir PID.
pub(crate) struct BeamEventSink {
    pid: rustler::LocalPid,
    owned_env: Arc<Mutex<OwnedEnv>>,
}

impl BeamEventSink {
    pub(crate) fn new(pid: rustler::LocalPid) -> Self {
        Self {
            pid,
            owned_env: Arc::new(Mutex::new(OwnedEnv::new())),
        }
    }
}

impl EventSink for BeamEventSink {
    fn send(
        &self,
        event: node_engine::WorkflowEvent,
    ) -> std::result::Result<(), node_engine::EventError> {
        let json = serialize_workflow_event_json(&event)?;

        let pid = self.pid;
        let owned_env = self.owned_env.clone();
        std::thread::spawn(move || {
            let mut env = owned_env.lock().unwrap();
            let _ = env.send_and_clear(&pid, |env| {
                (atoms::workflow_event().encode(env), json.encode(env)).encode(env)
            });
        })
        .join()
        .map_err(|_| node_engine::EventError {
            message: "Event send thread panicked".to_string(),
        })?;

        Ok(())
    }
}

pub(crate) fn callback_respond(callback_id: String, outputs_json: String) -> NifResult<Atom> {
    complete_callback(callback_id, Ok(outputs_json))
}

pub(crate) fn callback_error(callback_id: String, error_message: String) -> NifResult<Atom> {
    complete_callback(callback_id, Err(error_message))
}

fn complete_callback(callback_id: String, result: Result<String, String>) -> NifResult<Atom> {
    let mut callbacks = PENDING_CALLBACKS
        .lock()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Lock poisoned: {}", e))))?;

    if let Some(sender) = callbacks.remove(&callback_id) {
        let _ = sender.send(result);
        Ok(atoms::ok())
    } else {
        Err(rustler::Error::Term(Box::new(format!(
            "Unknown callback: {}",
            callback_id
        ))))
    }
}

#[cfg(test)]
pub(crate) fn insert_pending_callback_for_test(
    callback_id: String,
    sender: oneshot::Sender<Result<String, String>>,
) {
    let mut callbacks = PENDING_CALLBACKS.lock().unwrap();
    callbacks.insert(callback_id, sender);
}
