//! Process Execution Task
//!
//! Executes an external process/command and captures its output.
//! Uses `tokio::process::Command` for async execution with timeout support.

use std::collections::HashMap;
use std::env;
use std::time::Duration;

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

/// Default timeout in seconds for process execution
const DEFAULT_TIMEOUT_SECS: u64 = 300;
const ENABLE_PROCESS_NODE_ENV: &str = "PANTOGRAPH_ENABLE_PROCESS_NODE";
const PROCESS_NODE_ALLOWLIST_ENV: &str = "PANTOGRAPH_PROCESS_NODE_ALLOWLIST";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProcessExecutionPolicy {
    allowed_commands: Vec<String>,
}

impl ProcessExecutionPolicy {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn allow_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut allowed_commands: Vec<String> = commands
            .into_iter()
            .map(Into::into)
            .map(|command| command.trim().to_string())
            .filter(|command| !command.is_empty())
            .collect();
        allowed_commands.sort();
        allowed_commands.dedup();
        Self { allowed_commands }
    }

    pub fn from_environment() -> Self {
        let enabled = env::var(ENABLE_PROCESS_NODE_ENV).ok();
        let allowlist = env::var(PROCESS_NODE_ALLOWLIST_ENV).ok();
        Self::from_environment_values(enabled.as_deref(), allowlist.as_deref())
    }

    pub fn from_environment_values(enabled: Option<&str>, allowlist: Option<&str>) -> Self {
        if !enabled.is_some_and(is_truthy) {
            return Self::disabled();
        }
        let Some(allowlist) = allowlist else {
            return Self::disabled();
        };
        Self::allow_commands(allowlist.split(','))
    }

    fn authorize(&self, command: &str) -> graph_flow::Result<()> {
        if self
            .allowed_commands
            .iter()
            .any(|allowed| allowed == command)
        {
            return Ok(());
        }
        Err(GraphError::TaskExecutionFailed(format!(
            "Process execution for '{}' is not allowed by host policy",
            command
        )))
    }
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

async fn collect_pipe_output(handle: tokio::task::JoinHandle<Vec<u8>>) -> Vec<u8> {
    match tokio::time::timeout(Duration::from_secs(1), handle).await {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(err)) => {
            log::warn!("Pipe reader task failed: {}", err);
            Vec::new()
        }
        Err(_) => Vec::new(),
    }
}

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
    execution_policy: ProcessExecutionPolicy,
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
            execution_policy: ProcessExecutionPolicy::disabled(),
        }
    }

    pub fn new_with_policy(
        task_id: impl Into<String>,
        execution_policy: ProcessExecutionPolicy,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            execution_policy,
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
        self.execution_policy.authorize(&command)?;

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
        cmd.kill_on_drop(true);
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

        let mut child = cmd.spawn().map_err(|e| {
            GraphError::TaskExecutionFailed(format!("Failed to spawn process '{}': {}", command, e))
        })?;

        let stdout_pipe = child.stdout.take().ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Failed to capture stdout for process '{}'",
                command
            ))
        })?;
        let stderr_pipe = child.stderr.take().ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Failed to capture stderr for process '{}'",
                command
            ))
        })?;

        let stdout_reader = tokio::spawn(async move {
            let mut reader = stdout_pipe;
            let mut buf = Vec::new();
            let _ = reader.read_to_end(&mut buf).await;
            buf
        });
        let stderr_reader = tokio::spawn(async move {
            let mut reader = stderr_pipe;
            let mut buf = Vec::new();
            let _ = reader.read_to_end(&mut buf).await;
            buf
        });

        // Write stdin if provided.
        if let Some(ref data) = stdin_data {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(data.as_bytes()).await.map_err(|e| {
                    GraphError::TaskExecutionFailed(format!(
                        "Failed to write stdin for process '{}': {}",
                        command, e
                    ))
                })?;
            }
        }

        let wait_result =
            tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait()).await;

        let (exit_code, stdout, stderr, success) = match wait_result {
            Ok(Ok(status)) => {
                let out = collect_pipe_output(stdout_reader).await;
                let err = collect_pipe_output(stderr_reader).await;
                (
                    status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&out).to_string(),
                    String::from_utf8_lossy(&err).to_string(),
                    status.success(),
                )
            }
            Ok(Err(e)) => {
                return Err(GraphError::TaskExecutionFailed(format!(
                    "Failed to wait for process '{}': {}",
                    command, e
                )));
            }
            Err(_) => {
                let timeout_msg = match child.kill().await {
                    Ok(_) => "Process timed out and was terminated".to_string(),
                    Err(e) => format!("Process timed out; failed to terminate child: {}", e),
                };
                let _ = child.wait().await;
                let out = collect_pipe_output(stdout_reader).await;
                let err = collect_pipe_output(stderr_reader).await;
                let mut stderr_msg = String::from_utf8_lossy(&err).to_string();
                if !stderr_msg.is_empty() {
                    stderr_msg.push('\n');
                }
                stderr_msg.push_str(&timeout_msg);
                (
                    -1i32,
                    String::from_utf8_lossy(&out).to_string(),
                    stderr_msg,
                    false,
                )
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
            Some(format!(
                "Process '{}' exited with code {}",
                command, exit_code
            )),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn allowed_process_task(task_id: &str, commands: &[&str]) -> ProcessTask {
        ProcessTask::new_with_policy(
            task_id,
            ProcessExecutionPolicy::allow_commands(commands.iter().copied()),
        )
    }

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
    async fn test_default_policy_denies_process_execution() {
        let task = ProcessTask::new("test_denied");
        let context = Context::new();

        let cmd_key = ContextKeys::input("test_denied", ProcessTask::PORT_COMMAND);
        context.set(&cmd_key, "echo".to_string()).await;

        let result = task.run(context).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not allowed by host policy")
        );
    }

    #[test]
    fn test_environment_policy_requires_enable_and_allowlist() {
        assert_eq!(
            ProcessExecutionPolicy::from_environment_values(None, Some("echo")),
            ProcessExecutionPolicy::disabled()
        );
        assert_eq!(
            ProcessExecutionPolicy::from_environment_values(Some("true"), None),
            ProcessExecutionPolicy::disabled()
        );
        assert_eq!(
            ProcessExecutionPolicy::from_environment_values(Some("true"), Some("echo, pwd")),
            ProcessExecutionPolicy::allow_commands(["echo", "pwd"])
        );
    }

    #[tokio::test]
    async fn test_echo_command() {
        let task = allowed_process_task("test_echo", &["echo"]);
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
        let task = allowed_process_task("test_fail", &["false"]);
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
        let task = allowed_process_task("test_stdin", &["cat"]);
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
        let task = allowed_process_task("test_env", &["env"]);
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
        let task = allowed_process_task("test_cwd", &["pwd"]);
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

    #[cfg(unix)]
    #[tokio::test]
    async fn test_timeout_terminates_process() {
        let dir = tempdir().unwrap();
        let marker = dir.path().join("should_not_exist.txt");

        let task = allowed_process_task("test_timeout", &["sh"]);
        let context = Context::new();

        let cmd_key = ContextKeys::input("test_timeout", ProcessTask::PORT_COMMAND);
        context.set(&cmd_key, "sh".to_string()).await;

        let args_key = ContextKeys::input("test_timeout", ProcessTask::PORT_ARGS);
        let script = format!("sleep 2; echo orphan > \"{}\"", marker.display());
        context
            .set(&args_key, serde_json::json!(["-c", script]))
            .await;

        let timeout_key = ContextKeys::input("test_timeout", ProcessTask::PORT_TIMEOUT);
        context.set(&timeout_key, 1.0f64).await;

        let _result = task.run(context.clone()).await.unwrap();

        let success_key = ContextKeys::output("test_timeout", ProcessTask::PORT_SUCCESS);
        let success: bool = context.get(&success_key).await.unwrap();
        assert!(!success);

        tokio::time::sleep(Duration::from_millis(1500)).await;
        assert!(
            !marker.exists(),
            "Timed-out process continued running after timeout"
        );
    }
}
