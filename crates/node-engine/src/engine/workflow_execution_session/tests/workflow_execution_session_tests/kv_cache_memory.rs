use super::*;

struct KvCacheProducingTaskExecutor;

#[async_trait]
impl TaskExecutor for KvCacheProducingTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        _inputs: std::collections::HashMap<String, serde_json::Value>,
        _context: &graph_flow::Context,
        _extensions: &crate::extensions::ExecutorExtensions,
    ) -> crate::error::Result<std::collections::HashMap<String, serde_json::Value>> {
        Ok(std::collections::HashMap::from([
            ("out".to_string(), serde_json::json!(task_id)),
            (
                "kv_cache_out".to_string(),
                serde_json::json!({
                    "cache_id": format!("cache-{task_id}"),
                    "compatibility": {
                        "model_fingerprint": {
                            "model_id": format!("model-{task_id}"),
                            "config_hash": "cfg-1",
                        },
                        "runtime_fingerprint": {
                            "runtime_id": "runtime-1",
                            "backend_key": "llamacpp",
                            "tokenizer_fingerprint": "tok-1",
                            "prompt_format_fingerprint": "prompt-1",
                            "runtime_build_fingerprint": "build-1",
                        }
                    }
                }),
            ),
        ]))
    }
}

#[tokio::test]
async fn sync_bound_session_node_memory_from_cache_projects_kv_cache_reference() {
    let mut graph = crate::types::WorkflowGraph::new("graph-1", "Graph");
    graph.nodes.push(crate::types::GraphNode {
        id: "llm".to_string(),
        node_type: "llamacpp-inference".to_string(),
        data: serde_json::json!({}),
        position: (0.0, 0.0),
    });

    let executor = WorkflowExecutor::new("exec-1", graph, Arc::new(NullEventSink));
    bind_workflow_execution_session(&executor, "session-1").await;

    executor
        .demand(&"llm".to_string(), &KvCacheProducingTaskExecutor)
        .await
        .expect("run kv-producing demand");
    sync_bound_session_node_memory_from_cache(&executor).await;

    let snapshots = workflow_execution_session_node_memory_snapshots(&executor, "session-1").await;
    assert_eq!(snapshots.len(), 1);
    assert_eq!(
        snapshots[0].indirect_state_reference,
        Some(crate::engine::NodeMemoryIndirectStateReference {
            reference_kind: "kv_cache_handle".to_string(),
            reference_id: "cache-llm".to_string(),
            restore_strategy: crate::engine::NodeMemoryRestoreStrategy::RehydrateBeforeResume,
            inspection_metadata: Some(serde_json::json!({
                "source_port": "kv_cache_out",
                "backend_key": "llamacpp",
                "model_fingerprint": {
                    "model_id": "model-llm",
                    "config_hash": "cfg-1",
                },
                "runtime_fingerprint": {
                    "runtime_id": "runtime-1",
                    "backend_key": "llamacpp",
                    "tokenizer_fingerprint": "tok-1",
                    "prompt_format_fingerprint": "prompt-1",
                    "runtime_build_fingerprint": "build-1",
                }
            })),
        })
    );
}

struct KvCacheReusingTaskExecutor {
    execution_counter: AtomicUsize,
}

impl KvCacheReusingTaskExecutor {
    fn new() -> Self {
        Self {
            execution_counter: AtomicUsize::new(0),
        }
    }
}

#[derive(Default)]
struct KvSuffixReuseTaskExecutor {
    run_counts: std::sync::Mutex<std::collections::HashMap<String, usize>>,
}

impl KvSuffixReuseTaskExecutor {
    fn run_count(&self, node_id: &str) -> usize {
        self.run_counts
            .lock()
            .expect("run counts lock")
            .get(node_id)
            .copied()
            .unwrap_or(0)
    }
}

#[async_trait]
impl TaskExecutor for KvSuffixReuseTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: std::collections::HashMap<String, serde_json::Value>,
        _context: &graph_flow::Context,
        _extensions: &crate::extensions::ExecutorExtensions,
    ) -> crate::error::Result<std::collections::HashMap<String, serde_json::Value>> {
        let run_number = {
            let mut counts = self.run_counts.lock().expect("run counts lock");
            let count = counts.entry(task_id.to_string()).or_insert(0);
            *count += 1;
            *count
        };

        match task_id {
            "prefix-input" | "suffix-input" => Ok(std::collections::HashMap::from([(
                "text".to_string(),
                inputs
                    .get("_data")
                    .and_then(|data| data.get("text"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            )])),
            "prefix-llm" => Ok(std::collections::HashMap::from([
                (
                    "response".to_string(),
                    serde_json::json!({
                        "prompt": inputs.get("prompt").cloned().unwrap_or(serde_json::Value::Null),
                        "run_number": run_number,
                    }),
                ),
                (
                    "kv_cache_out".to_string(),
                    serde_json::json!({
                        "cache_id": format!("prefix-cache-run-{run_number}"),
                        "compatibility": {
                            "model_fingerprint": {
                                "model_id": "model-1",
                                "config_hash": "cfg-1",
                            },
                            "runtime_fingerprint": {
                                "runtime_id": "runtime-1",
                                "backend_key": "llamacpp",
                                "tokenizer_fingerprint": "tok-1",
                                "prompt_format_fingerprint": "prompt-1",
                                "runtime_build_fingerprint": "build-1",
                            }
                        }
                    }),
                ),
            ])),
            "suffix-llm" => Ok(std::collections::HashMap::from([
                (
                    "observed_prompt".to_string(),
                    inputs
                        .get("prompt")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                ),
                (
                    "observed_kv_cache_in".to_string(),
                    inputs
                        .get("kv_cache_in")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                ),
                (
                    "response".to_string(),
                    serde_json::json!({
                        "run_number": run_number,
                    }),
                ),
            ])),
            other => panic!("unexpected task id: {other}"),
        }
    }
}

#[async_trait]
impl TaskExecutor for KvCacheReusingTaskExecutor {
    async fn execute_task(
        &self,
        _task_id: &str,
        inputs: std::collections::HashMap<String, serde_json::Value>,
        _context: &graph_flow::Context,
        _extensions: &crate::extensions::ExecutorExtensions,
    ) -> crate::error::Result<std::collections::HashMap<String, serde_json::Value>> {
        let run_number = self.execution_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let kv_cache_in = inputs.get("kv_cache_in").cloned();
        Ok(std::collections::HashMap::from([
            (
                "out".to_string(),
                serde_json::json!({ "run_number": run_number }),
            ),
            (
                "kv_cache_out".to_string(),
                serde_json::json!({
                    "cache_id": format!("cache-run-{run_number}"),
                    "compatibility": {
                        "model_fingerprint": {
                            "model_id": "model-1",
                            "config_hash": "cfg-1",
                        },
                        "runtime_fingerprint": {
                            "runtime_id": "runtime-1",
                            "backend_key": "llamacpp",
                            "tokenizer_fingerprint": "tok-1",
                            "prompt_format_fingerprint": "prompt-1",
                            "runtime_build_fingerprint": "build-1",
                        }
                    }
                }),
            ),
            (
                "observed_kv_cache_in".to_string(),
                kv_cache_in.unwrap_or(serde_json::Value::Null),
            ),
        ]))
    }
}

#[tokio::test]
async fn rerun_projects_preserved_kv_cache_reference_back_into_inputs() {
    let executor = WorkflowExecutor::new("exec-1", single_node_graph(), Arc::new(NullEventSink));
    let task_executor = KvCacheReusingTaskExecutor::new();
    bind_workflow_execution_session(&executor, "session-1").await;

    let first_outputs = executor
        .demand(&"memory".to_string(), &task_executor)
        .await
        .expect("first run should succeed");
    assert_eq!(
        first_outputs.get("observed_kv_cache_in"),
        Some(&serde_json::Value::Null)
    );

    executor.mark_modified(&"memory".to_string()).await;
    let second_outputs = executor
        .demand(&"memory".to_string(), &task_executor)
        .await
        .expect("second run should succeed");
    assert_eq!(
        second_outputs.get("observed_kv_cache_in"),
        Some(&serde_json::json!({
            "cache_id": "cache-run-1",
            "compatibility": {
                "model_fingerprint": {
                    "model_id": "model-1",
                    "config_hash": "cfg-1",
                },
                "runtime_fingerprint": {
                    "runtime_id": "runtime-1",
                    "backend_key": "llamacpp",
                    "tokenizer_fingerprint": "tok-1",
                    "prompt_format_fingerprint": "prompt-1",
                    "runtime_build_fingerprint": "build-1",
                }
            }
        }))
    );
}

#[tokio::test]
async fn invalidated_node_memory_does_not_project_preserved_kv_cache_back_into_inputs() {
    let executor = WorkflowExecutor::new("exec-1", single_node_graph(), Arc::new(NullEventSink));
    let task_executor = KvCacheReusingTaskExecutor::new();
    bind_workflow_execution_session(&executor, "session-1").await;

    executor
        .demand(&"memory".to_string(), &task_executor)
        .await
        .expect("first run should succeed");

    reconcile_workflow_execution_session_node_memory(
        &executor,
        "session-1",
        &GraphMemoryImpactSummary {
            node_decisions: vec![NodeMemoryCompatibilitySnapshot {
                node_id: "memory".to_string(),
                compatibility: NodeMemoryCompatibility::PreserveWithInputRefresh,
                reason: Some("upstream_prefix_changed".to_string()),
            }],
            fallback_to_full_invalidation: false,
        },
    )
    .await;

    executor.mark_modified(&"memory".to_string()).await;
    let second_outputs = executor
        .demand(&"memory".to_string(), &task_executor)
        .await
        .expect("second run should succeed");
    assert_eq!(
        second_outputs.get("observed_kv_cache_in"),
        Some(&serde_json::Value::Null)
    );
}

#[tokio::test]
async fn suffix_only_rerun_reuses_graph_wired_kv_without_rerunning_prefix() {
    let executor =
        WorkflowExecutor::new("exec-1", kv_suffix_reuse_graph(), Arc::new(NullEventSink));
    let task_executor = KvSuffixReuseTaskExecutor::default();
    bind_workflow_execution_session(&executor, "session-1").await;

    let first_outputs = executor
        .demand(&"suffix-llm".to_string(), &task_executor)
        .await
        .expect("first run should succeed");
    assert_eq!(
        first_outputs.get("observed_prompt"),
        Some(&serde_json::json!("suffix-alpha"))
    );
    assert_eq!(
        first_outputs.get("observed_kv_cache_in"),
        Some(&serde_json::json!({
            "cache_id": "prefix-cache-run-1",
            "compatibility": {
                "model_fingerprint": {
                    "model_id": "model-1",
                    "config_hash": "cfg-1",
                },
                "runtime_fingerprint": {
                    "runtime_id": "runtime-1",
                    "backend_key": "llamacpp",
                    "tokenizer_fingerprint": "tok-1",
                    "prompt_format_fingerprint": "prompt-1",
                    "runtime_build_fingerprint": "build-1",
                }
            }
        }))
    );
    assert_eq!(task_executor.run_count("prefix-input"), 1);
    assert_eq!(task_executor.run_count("prefix-llm"), 1);
    assert_eq!(task_executor.run_count("suffix-input"), 1);
    assert_eq!(task_executor.run_count("suffix-llm"), 1);

    let suffix_input_node_id = "suffix-input".to_string();
    executor
        .update_node_data(
            &suffix_input_node_id,
            serde_json::json!({ "text": "suffix-beta" }),
        )
        .await
        .expect("update suffix input");
    let second_outputs = executor
        .demand(&"suffix-llm".to_string(), &task_executor)
        .await
        .expect("second run should succeed");
    assert_eq!(
        second_outputs.get("observed_prompt"),
        Some(&serde_json::json!("suffix-beta"))
    );
    assert_eq!(
        second_outputs.get("observed_kv_cache_in"),
        Some(&serde_json::json!({
            "cache_id": "prefix-cache-run-1",
            "compatibility": {
                "model_fingerprint": {
                    "model_id": "model-1",
                    "config_hash": "cfg-1",
                },
                "runtime_fingerprint": {
                    "runtime_id": "runtime-1",
                    "backend_key": "llamacpp",
                    "tokenizer_fingerprint": "tok-1",
                    "prompt_format_fingerprint": "prompt-1",
                    "runtime_build_fingerprint": "build-1",
                }
            }
        }))
    );
    assert_eq!(task_executor.run_count("prefix-input"), 1);
    assert_eq!(task_executor.run_count("prefix-llm"), 1);
    assert_eq!(task_executor.run_count("suffix-input"), 2);
    assert_eq!(task_executor.run_count("suffix-llm"), 2);
}
