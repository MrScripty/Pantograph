//! Process Execution Task
//!
//! Executes an external process/command and captures its output.
//! Uses `tokio::process::Command` for async execution with timeout support.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use tokio::process::Command;

/// Default timeout in seconds for process execution
const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Process Execution Task
///
/// Spawns an external process, captures stdout/stderr, and returns
/// the exit code and output streams.
///
/// # Inputs (from context)
/// - `{task_id}.input.command` (required) - Command to execute
/// - `{task_id}.input.args` (optional) - JSON array of string arguments
/// - `{task_id}.input.cwd` (optional) - Working directory
/// - `{task_id}.input.env` (optional) - JSON object of environment variables
/// - `{task_id}.input.stdin` (optional) - String to pipe to stdin
/// - `{task_id}.input.timeout_secs` (optional) - Timeout in seconds (default: 300)
///
/// # Outputs (to context)
/// - `{task_id}.output.exit_code` - Process exit code (or -1 if killed)
/// - `{task_id}.output.stdout` - Captured stdout
/// - `{task_id}.output.stderr` - Captured stderr
/// - `{task_id}.output.success` - Whether exit code was 0
#[derive(Clone)]
pub struct ProcessTask {
    task_id: String,
}

impl ProcessTask {
    // Input ports
    pub const PORT_COMMAND: &'static str = "command";
    pub const PORT_ARGS: &'static str = "args";
    pub const PORT_CWD: &'static str = "cwd";
    pub const PORT_ENV: &'static str = "env";
    pub const PORT_STDIN: &'static str = "stdin";
    pub const PORT_TIMEOUT: &'static str = "timeout_secs";

    // Output ports
    pub const PORT_EXIT_CODE: &'static str = "exit_code";
    pub const PORT_STDOUT: &'static str = "stdout";
    pub const PORT_STDERR: &'static str = "stderr";
    pub const PORT_SUCCESS: &'static str = "success";

    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

impl TaskDescriptor for ProcessTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "process".to_string(),
            category: NodeCategory::Processing,
            label: "Process".to_string(),
            description: "Execute an external process/command".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_COMMAND, "Command", PortDataType::String),
                PortMetadata::optional(Self::PORT_ARGS, "Arguments", PortDataType::Json),
                PortMetadata::optional(Self::PORT_CWD, "Working Dir", PortDataType::String),
                PortMetadata::optional(Self::PORT_ENV, "Environment", PortDataType::Json),
                PortMetadata::optional(Self::PORT_STDIN, "Stdin", PortDataType::String),
                PortMetadata::optional(Self::PORT_TIMEOUT, "Timeout (s)", PortDataType::Number),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_EXIT_CODE, "Exit Code", PortDataType::Number),
                PortMetadata::optional(Self::PORT_STDOUT, "Stdout", PortDataType::String),
                PortMetadata::optional(Self::PORT_STDERR, "Stderr", PortDataType::String),
                PortMetadata::optional(Self::PORT_SUCCESS, "Success", PortDataType::Boolean),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(ProcessTask::descriptor));

#[async_trait]
impl Task for ProcessTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: command
        let cmd_key = ContextKeys::input(&self.task_id, Self::PORT_COMMAND);
        let command: String = context.get(&cmd_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'command' at key '{}'",
                cmd_key
            ))
        })?;

        // Get optional args
        let args_key = ContextKeys::input(&self.task_id, Self::PORT_ARGS);
        let args: Vec<String> = context
            .get::<serde_json::Value>(&args_key)
            .await
            .and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|item| item.as_str().map(String::from))
                        .collect()
                })
            })
            .unwrap_or_default();

        // Get optional working directory
        let cwd_key = ContextKeys::input(&self.task_id, Self::PORT_CWD);
        let cwd: Option<String> = context.get(&cwd_key).await;

        // Get optional environment variables
        let env_key = ContextKeys::input(&self.task_id, Self::PORT_ENV);
        let env_vars: HashMap<String, String> = context
            .get::<serde_json::Value>(&env_key)
            .await
            .and_then(|v| {
                v.as_object().map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
            })
            .unwrap_or_default();

        // Get optional stdin
        let stdin_key = ContextKeys::input(&self.task_id, Self::PORT_STDIN);
        let stdin_data: Option<String> = context.get(&stdin_key).await;

        // Get optional timeout
        let timeout_key = ContextKeys::input(&self.task_id, Self::PORT_TIMEOUT);
        let timeout_secs: u64 = context
            .get::<f64>(&timeout_key)
            .await
            .map(|v| v as u64)
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        log::debug!(
            "ProcessTask {}: executing '{}' with {} args, timeout {}s",
            self.task_id,
            command,
            args.len(),
            timeout_secs
        );

        // Build the command
        let mut cmd = Command::new(&command);
        cmd.args(&args);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(ref dir) = cwd {
            cmd.current_dir(dir);
        }

        if stdin_data.is_some() {
            cmd.stdin(std::process::Stdio::piped());
        } else {
            cmd.stdin(std::process::Stdio::null());
        }

        for (k, v) in &env_vars {
            cmd.env(k, v);
        }

        // Spawn and run with timeout
        let result = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
            let mut child = cmd.spawn().map_err(|e| {
                GraphError::TaskExecutionFailed(format!(
                    "Failed to spawn process '{}': {}",
                    command, e
                ))
            })?;

            // Write stdin if provided
            if let Some(ref data) = stdin_data {
                if let Some(mut stdin) = child.stdin.take() {
                    use tokio::io::AsyncWriteExt;
                    let _ = stdin.write_all(data.as_bytes()).await;
                    drop(stdin);
                }
            }

            let output = child.wait_with_output().await.map_err(|e| {
                GraphError::TaskExecutionFailed(format!(
                    "Failed to wait for process '{}': {}",
                    command, e
                ))
            })?;

            Ok::<_, GraphError>(output)
        })
        .await;

        let (exit_code, stdout, stderr, success) = match result {
            Ok(Ok(output)) => {
                let code = output.status.code().unwrap_or(-1);
                let out = String::from_utf8_lossy(&output.stdout).to_string();
                let err = String::from_utf8_lossy(&output.stderr).to_string();
                (code, out, err, output.status.success())
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // Timeout - report as failure
                (-1i32, String::new(), "Process timed out".to_string(), false)
            }
        };

        // Store outputs
        let exit_key = ContextKeys::output(&self.task_id, Self::PORT_EXIT_CODE);
        context.set(&exit_key, exit_code as f64).await;

        let stdout_key = ContextKeys::output(&self.task_id, Self::PORT_STDOUT);
        context.set(&stdout_key, stdout.clone()).await;

        let stderr_key = ContextKeys::output(&self.task_id, Self::PORT_STDERR);
        context.set(&stderr_key, stderr.clone()).await;

        let success_key = ContextKeys::output(&self.task_id, Self::PORT_SUCCESS);
        context.set(&success_key, success).await;

        log::debug!(
            "ProcessTask {}: exit_code={}, stdout={} bytes, stderr={} bytes",
            self.task_id,
            exit_code,
            stdout.len(),
            stderr.len()
        );

        Ok(TaskResult::new(
            Some(format!("Process '{}' exited with code {}", command, exit_code)),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor() {
        let meta = ProcessTask::descriptor();
        assert_eq!(meta.node_type, "process");
        assert_eq!(meta.category, NodeCategory::Processing);
        assert_eq!(meta.inputs.len(), 6);
        assert_eq!(meta.outputs.len(), 4);

        // Check required input
        let cmd_port = meta.inputs.iter().find(|p| p.id == "command").unwrap();
        assert!(cmd_port.required);

        // Check optional inputs
        let args_port = meta.inputs.iter().find(|p| p.id == "args").unwrap();
        assert!(!args_port.required);
    }

    #[test]
    fn test_task_id() {
        let task = ProcessTask::new("proc-1");
        assert_eq!(task.id(), "proc-1");
    }

    #[tokio::test]
    async fn test_missing_command_error() {
        let task = ProcessTask::new("test_proc");
        let context = Context::new();

        let result = task.run(context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_echo_command() {
        let task = ProcessTask::new("test_echo");
        let context = Context::new();

        // Set command input
        let cmd_key = ContextKeys::input("test_echo", ProcessTask::PORT_COMMAND);
        context.set(&cmd_key, "echo".to_string()).await;

        // Set args
        let args_key = ContextKeys::input("test_echo", ProcessTask::PORT_ARGS);
        context
            .set(&args_key, serde_json::json!(["hello", "world"]))
            .await;

        let _result = task.run(context.clone()).await.unwrap();

        // Check outputs
        let stdout_key = ContextKeys::output("test_echo", ProcessTask::PORT_STDOUT);
        let stdout: String = context.get(&stdout_key).await.unwrap();
        assert_eq!(stdout.trim(), "hello world");

        let success_key = ContextKeys::output("test_echo", ProcessTask::PORT_SUCCESS);
        let success: bool = context.get(&success_key).await.unwrap();
        assert!(success);

        let exit_key = ContextKeys::output("test_echo", ProcessTask::PORT_EXIT_CODE);
        let exit_code: f64 = context.get(&exit_key).await.unwrap();
        assert_eq!(exit_code, 0.0);
    }

    #[tokio::test]
    async fn test_failing_command() {
        let task = ProcessTask::new("test_fail");
        let context = Context::new();

        let cmd_key = ContextKeys::input("test_fail", ProcessTask::PORT_COMMAND);
        context.set(&cmd_key, "false".to_string()).await;

        let _result = task.run(context.clone()).await.unwrap();

        let success_key = ContextKeys::output("test_fail", ProcessTask::PORT_SUCCESS);
        let success: bool = context.get(&success_key).await.unwrap();
        assert!(!success);

        let exit_key = ContextKeys::output("test_fail", ProcessTask::PORT_EXIT_CODE);
        let exit_code: f64 = context.get(&exit_key).await.unwrap();
        assert_ne!(exit_code, 0.0);
    }

    #[tokio::test]
    async fn test_stdin_pipe() {
        let task = ProcessTask::new("test_stdin");
        let context = Context::new();

        let cmd_key = ContextKeys::input("test_stdin", ProcessTask::PORT_COMMAND);
        context.set(&cmd_key, "cat".to_string()).await;

        let stdin_key = ContextKeys::input("test_stdin", ProcessTask::PORT_STDIN);
        context.set(&stdin_key, "piped input".to_string()).await;

        let _result = task.run(context.clone()).await.unwrap();

        let stdout_key = ContextKeys::output("test_stdin", ProcessTask::PORT_STDOUT);
        let stdout: String = context.get(&stdout_key).await.unwrap();
        assert_eq!(stdout, "piped input");
    }

    #[tokio::test]
    async fn test_env_vars() {
        let task = ProcessTask::new("test_env");
        let context = Context::new();

        let cmd_key = ContextKeys::input("test_env", ProcessTask::PORT_COMMAND);
        context.set(&cmd_key, "env".to_string()).await;

        let env_key = ContextKeys::input("test_env", ProcessTask::PORT_ENV);
        context
            .set(&env_key, serde_json::json!({"TEST_VAR": "test_value"}))
            .await;

        let _result = task.run(context.clone()).await.unwrap();

        let stdout_key = ContextKeys::output("test_env", ProcessTask::PORT_STDOUT);
        let stdout: String = context.get(&stdout_key).await.unwrap();
        assert!(stdout.contains("TEST_VAR=test_value"));
    }

    #[tokio::test]
    async fn test_cwd() {
        let task = ProcessTask::new("test_cwd");
        let context = Context::new();

        let cmd_key = ContextKeys::input("test_cwd", ProcessTask::PORT_COMMAND);
        context.set(&cmd_key, "pwd".to_string()).await;

        let cwd_key = ContextKeys::input("test_cwd", ProcessTask::PORT_CWD);
        context.set(&cwd_key, "/tmp".to_string()).await;

        let _result = task.run(context.clone()).await.unwrap();

        let stdout_key = ContextKeys::output("test_cwd", ProcessTask::PORT_STDOUT);
        let stdout: String = context.get(&stdout_key).await.unwrap();
        // /tmp might resolve to a real path on some systems
        assert!(stdout.trim() == "/tmp" || stdout.trim().ends_with("/tmp"));
    }
}
