use std::sync::Mutex;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PythonRuntimeExecutionMetadata {
    pub snapshot: inference::RuntimeLifecycleSnapshot,
    pub model_target: Option<String>,
}

#[derive(Debug, Default)]
pub struct PythonRuntimeExecutionRecorder {
    state: Mutex<Vec<PythonRuntimeExecutionMetadata>>,
}

impl PythonRuntimeExecutionRecorder {
    pub fn record(&self, metadata: PythonRuntimeExecutionMetadata) {
        self.state
            .lock()
            .expect("python runtime recorder lock")
            .push(metadata);
    }

    pub fn snapshot(&self) -> Option<PythonRuntimeExecutionMetadata> {
        self.state
            .lock()
            .expect("python runtime recorder lock")
            .last()
            .cloned()
    }

    pub fn snapshots(&self) -> Vec<PythonRuntimeExecutionMetadata> {
        self.state
            .lock()
            .expect("python runtime recorder lock")
            .clone()
    }
}
