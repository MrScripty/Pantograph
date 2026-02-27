use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use node_engine::{
    extension_keys, DependencyState, ExecutorExtensions, ModelDependencyInstallResult,
    ModelDependencyPlan, ModelDependencyRequest, ModelDependencyStatus,
};
use pumas_library::index::ModelRecord;
use tokio::sync::RwLock;

#[path = "../workflow/model_dependencies.rs"]
mod model_dependencies;

#[derive(Debug, Clone)]
struct ScenarioModel {
    label: String,
    record: ModelRecord,
}

#[derive(Debug, Clone)]
struct DependencyHints {
    model_dir_exists: bool,
    requirements_files: Vec<String>,
    import_hints: Vec<String>,
    has_auto_map: bool,
    has_modeling_py: bool,
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find_map(|window| {
        if window[0] == flag {
            Some(window[1].clone())
        } else {
            None
        }
    })
}

fn arg_values(args: &[String], flag: &str) -> Vec<String> {
    args.windows(2)
        .filter_map(|window| {
            if window[0] == flag {
                let value = window[1].trim();
                if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            } else {
                None
            }
        })
        .collect()
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn platform_context() -> serde_json::Value {
    serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    })
}

fn platform_key() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn infer_request_fields(record: &ModelRecord) -> (String, Option<String>, Option<String>) {
    let model_type = record.model_type.to_lowercase();
    if model_type == "audio" {
        return (
            "audio-generation".to_string(),
            Some("stable_audio".to_string()),
            Some("text-to-audio".to_string()),
        );
    }
    if model_type == "llm" {
        let has_gguf = record
            .metadata
            .get("files")
            .and_then(|v| v.as_array())
            .is_some_and(|files| {
                files.iter().any(|f| {
                    f.get("name")
                        .and_then(|v| v.as_str())
                        .is_some_and(|name| name.to_lowercase().ends_with(".gguf"))
                })
            });
        if has_gguf {
            return (
                "llamacpp-inference".to_string(),
                Some("llamacpp".to_string()),
                Some("text-generation".to_string()),
            );
        }
        return (
            "pytorch-inference".to_string(),
            Some("pytorch".to_string()),
            Some("text-generation".to_string()),
        );
    }

    let task_type_primary = record
        .metadata
        .as_object()
        .and_then(|meta| {
            meta.get("task_type_primary")
                .and_then(|v| v.as_str())
                .or_else(|| meta.get("taskTypePrimary").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
        })
        .or_else(|| Some("text-generation".to_string()));

    (
        "pytorch-inference".to_string(),
        Some("pytorch".to_string()),
        task_type_primary,
    )
}

async fn has_bindings_for_backend(
    api: &pumas_library::PumasApi,
    model: &ModelRecord,
    platform: &str,
    backend_key: Option<&str>,
) -> bool {
    api.resolve_model_dependency_plan(&model.id, platform, backend_key)
        .await
        .map(|plan| !plan.bindings.is_empty())
        .unwrap_or(false)
}

async fn model_selection(
    api: &pumas_library::PumasApi,
    models: &[ModelRecord],
    platform: &str,
) -> Vec<ScenarioModel> {
    let mut out = Vec::new();

    let mut audio_with_bindings = None;
    for model in models
        .iter()
        .filter(|m| m.model_type.eq_ignore_ascii_case("audio"))
    {
        if has_bindings_for_backend(api, model, platform, Some("stable_audio")).await
            || has_bindings_for_backend(api, model, platform, None).await
        {
            audio_with_bindings = Some(model.clone());
            break;
        }
    }
    if let Some(audio) = audio_with_bindings {
        out.push(ScenarioModel {
            label: "audio-with-bindings".to_string(),
            record: audio,
        });
    } else if let Some(audio) = models
        .iter()
        .find(|m| m.model_type.eq_ignore_ascii_case("audio"))
    {
        out.push(ScenarioModel {
            label: "audio".to_string(),
            record: audio.clone(),
        });
    }

    let mut non_audio_with_bindings = None;
    for model in models
        .iter()
        .filter(|m| !m.model_type.eq_ignore_ascii_case("audio"))
    {
        let (_, backend, _) = infer_request_fields(model);
        if has_bindings_for_backend(api, model, platform, backend.as_deref()).await
            || has_bindings_for_backend(api, model, platform, None).await
        {
            non_audio_with_bindings = Some(model.clone());
            break;
        }
    }
    if let Some(non_audio) = non_audio_with_bindings {
        out.push(ScenarioModel {
            label: "non-audio-with-bindings".to_string(),
            record: non_audio,
        });
    } else if let Some(non_audio) = models
        .iter()
        .find(|m| !m.model_type.eq_ignore_ascii_case("audio"))
    {
        out.push(ScenarioModel {
            label: "non-audio".to_string(),
            record: non_audio.clone(),
        });
    }

    if out.is_empty() {
        for model in models.iter().take(2) {
            out.push(ScenarioModel {
                label: "fallback".to_string(),
                record: model.clone(),
            });
        }
    }

    out
}

fn model_dependency_hints(model_path: &str) -> DependencyHints {
    let path = Path::new(model_path);
    if !path.exists() || !path.is_dir() {
        return DependencyHints {
            model_dir_exists: false,
            requirements_files: Vec::new(),
            import_hints: Vec::new(),
            has_auto_map: false,
            has_modeling_py: false,
        };
    }

    let mut requirements_files = Vec::new();
    let mut import_hints = BTreeSet::new();
    let mut has_auto_map = false;
    let mut has_modeling_py = false;

    let known_requirements = [
        "requirements.txt",
        "requirements-dev.txt",
        "environment.yml",
        "environment.yaml",
        "pyproject.toml",
    ];
    for file in known_requirements {
        if path.join(file).exists() {
            requirements_files.push(file.to_string());
        }
    }

    if let Ok(raw) = std::fs::read_to_string(path.join("config.json")) {
        if let Ok(config_json) = serde_json::from_str::<serde_json::Value>(&raw) {
            has_auto_map = config_json
                .get("auto_map")
                .is_some_and(|value| value.is_object());
        }
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            let Some(name) = file_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with("modeling_") && name.ends_with(".py") {
                has_modeling_py = true;
            }
            if !name.ends_with(".py") {
                continue;
            }

            let Ok(content) = std::fs::read_to_string(&file_path) else {
                continue;
            };
            for needle in [
                "flash_attn",
                "liger_kernel",
                "transformers",
                "torch",
                "diffusers",
                "torchaudio",
                "audiocraft",
                "sentencepiece",
                "bitsandbytes",
            ] {
                if content.contains(needle) {
                    import_hints.insert(needle.to_string());
                }
            }
        }
    }

    DependencyHints {
        model_dir_exists: true,
        requirements_files,
        import_hints: import_hints.into_iter().collect(),
        has_auto_map,
        has_modeling_py,
    }
}

async fn backend_binding_matrix(
    api: &pumas_library::PumasApi,
    model_id: &str,
    platform_key: &str,
    model_type: &str,
) -> Vec<serde_json::Value> {
    let mut candidates = vec![
        None,
        Some("stable_audio"),
        Some("pytorch"),
        Some("transformers"),
        Some("llamacpp"),
        Some("ollama"),
    ];

    if model_type.eq_ignore_ascii_case("audio") {
        candidates.sort_by_key(|b| if b == &Some("stable_audio") { 0 } else { 1 });
    } else if model_type.eq_ignore_ascii_case("llm") {
        candidates.sort_by_key(|b| {
            if b == &Some("pytorch") || b == &Some("transformers") {
                0
            } else {
                1
            }
        });
    }
    candidates.dedup();

    let mut out = Vec::new();
    for backend in candidates {
        let key = backend.unwrap_or("unspecified");
        match api
            .resolve_model_dependency_plan(model_id, platform_key, backend)
            .await
        {
            Ok(plan) => out.push(serde_json::json!({
                "backend_key": key,
                "state": plan.state,
                "code": plan.error_code,
                "binding_count": plan.bindings.len(),
            })),
            Err(err) => out.push(serde_json::json!({
                "backend_key": key,
                "error": err.to_string(),
            })),
        }
    }
    out
}

async fn scan_all_binding_hits(
    api: &pumas_library::PumasApi,
    models: &[ModelRecord],
    platform_key: &str,
) -> Vec<serde_json::Value> {
    let mut out = Vec::new();

    for model in models {
        let matrix = backend_binding_matrix(api, &model.id, platform_key, &model.model_type).await;
        for row in matrix {
            let Some(binding_count) = row.get("binding_count").and_then(|v| v.as_u64()) else {
                continue;
            };
            if binding_count == 0 {
                continue;
            }

            out.push(serde_json::json!({
                "model_id": model.id,
                "model_type": model.model_type,
                "backend_key": row.get("backend_key").and_then(|v| v.as_str()).unwrap_or("unknown"),
                "state": row.get("state").cloned().unwrap_or(serde_json::Value::Null),
                "code": row.get("code").cloned().unwrap_or(serde_json::Value::Null),
                "binding_count": binding_count,
            }));
        }
    }

    out.sort_by(|left, right| {
        let l_model = left
            .get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let l_backend = left
            .get("backend_key")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let r_model = right
            .get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let r_backend = right
            .get("backend_key")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        l_model.cmp(r_model).then_with(|| l_backend.cmp(r_backend))
    });

    out
}

async fn run_flow(
    resolver: &model_dependencies::TauriModelDependencyResolver,
    request: ModelDependencyRequest,
    run_install: bool,
) -> Result<
    (
        ModelDependencyPlan,
        ModelDependencyStatus,
        Option<ModelDependencyInstallResult>,
        ModelDependencyStatus,
    ),
    String,
> {
    let t0 = Instant::now();
    let plan = resolver.resolve_plan_request(request.clone()).await?;
    println!(
        "  resolve: state={:?} code={:?} bindings={} elapsed_ms={}",
        plan.state,
        plan.code,
        plan.bindings.len(),
        t0.elapsed().as_millis()
    );

    let t1 = Instant::now();
    let status_before = resolver.check_request(request.clone()).await?;
    println!(
        "  check-before: state={:?} code={:?} bindings={} elapsed_ms={}",
        status_before.state,
        status_before.code,
        status_before.bindings.len(),
        t1.elapsed().as_millis()
    );

    let mut install_result = None;
    if run_install {
        let t2 = Instant::now();
        let install = tokio::time::timeout(
            Duration::from_secs(180),
            resolver.install_request(request.clone()),
        )
        .await
        .map_err(|_| "install timed out after 180s".to_string())??;
        println!(
            "  install: state={:?} code={:?} bindings={} elapsed_ms={}",
            install.state,
            install.code,
            install.bindings.len(),
            t2.elapsed().as_millis()
        );
        install_result = Some(install);
    }

    let t3 = Instant::now();
    let status_after = resolver.check_request(request).await?;
    println!(
        "  check-after: state={:?} code={:?} bindings={} elapsed_ms={}",
        status_after.state,
        status_after.code,
        status_after.bindings.len(),
        t3.elapsed().as_millis()
    );

    Ok((plan, status_before, install_result, status_after))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let launcher_root = arg_value(&args, "--launcher-root")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/media/jeremy/OrangeCream/Linux Software/Pumas-Library"));
    let run_install = has_flag(&args, "--install");
    let output_json = has_flag(&args, "--json");
    let scan_all = has_flag(&args, "--scan-all");
    let selected_model_ids = arg_values(&args, "--model-id");

    println!("Pumas dependency runtime probe");
    println!("launcher_root={}", launcher_root.display());
    println!("install_enabled={}", run_install);
    println!("scan_all_enabled={}", scan_all);
    println!("model_ids_requested={}", selected_model_ids.len());
    println!(
        "platform_context={}",
        serde_json::to_string(&platform_context())?
    );
    let platform = platform_key();
    println!("platform_key={}", platform);

    let api = Arc::new(pumas_library::PumasApi::new(&launcher_root).await?);
    let models = api.list_models().await?;
    println!("models_total={}", models.len());

    if models.is_empty() {
        println!("No models found; aborting runtime probe.");
        return Ok(());
    }

    let mut ext = ExecutorExtensions::default();
    ext.set(extension_keys::PUMAS_API, api.clone());
    let shared_extensions = Arc::new(RwLock::new(ext));
    let resolver = model_dependencies::TauriModelDependencyResolver::new(
        shared_extensions,
        std::env::current_dir()?,
    );

    let selected = if selected_model_ids.is_empty() {
        model_selection(api.as_ref(), &models, &platform).await
    } else {
        let mut out = Vec::new();
        for model_id in &selected_model_ids {
            match models.iter().find(|m| m.id == *model_id) {
                Some(record) => out.push(ScenarioModel {
                    label: format!("explicit:{model_id}"),
                    record: record.clone(),
                }),
                None => {
                    println!("warning: requested model_id not found: {model_id}");
                }
            }
        }
        out
    };
    println!("scenarios_selected={}", selected.len());

    let mut summary = Vec::new();

    for scenario in selected {
        let (node_type, backend_key, task_type_primary) = infer_request_fields(&scenario.record);
        let request = ModelDependencyRequest {
            node_type,
            model_path: scenario.record.path.clone(),
            model_id: Some(scenario.record.id.clone()),
            model_type: Some(scenario.record.model_type.clone()),
            task_type_primary,
            backend_key,
            platform_context: Some(platform_context()),
            selected_binding_ids: Vec::new(),
        };

        println!("\n== scenario: {} ==", scenario.label);
        println!("model_id={}", scenario.record.id);
        println!("model_type={}", scenario.record.model_type);
        println!("model_path={}", scenario.record.path);
        println!("node_type={}", request.node_type);
        println!("backend_key={:?}", request.backend_key);
        let hints = model_dependency_hints(&scenario.record.path);
        println!(
            "dependency_hints: model_dir_exists={} has_modeling_py={} has_auto_map={} requirements_files={} imports={}",
            hints.model_dir_exists,
            hints.has_modeling_py,
            hints.has_auto_map,
            hints.requirements_files.len(),
            hints.import_hints.join(",")
        );
        let backend_matrix = backend_binding_matrix(
            api.as_ref(),
            &scenario.record.id,
            &platform,
            &scenario.record.model_type,
        )
        .await;
        for row in &backend_matrix {
            let backend = row
                .get("backend_key")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let count = row
                .get("binding_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let state = row.get("state").and_then(|v| v.as_str()).unwrap_or("error");
            let code = row.get("code").and_then(|v| v.as_str()).unwrap_or("-");
            println!(
                "  backend_probe: backend={backend} state={state} code={code} bindings={count}"
            );
        }

        match run_flow(&resolver, request.clone(), run_install).await {
            Ok((plan, before, install, after)) => {
                let install_state = install.as_ref().map(|r| r.state.clone());
                summary.push(serde_json::json!({
                    "label": scenario.label,
                    "model_id": scenario.record.id,
                    "node_type": request.node_type,
                    "backend_key": request.backend_key,
                    "plan_state": plan.state,
                    "plan_code": plan.code,
                    "status_before_state": before.state,
                    "status_before_code": before.code,
                    "install_state": install_state,
                    "status_after_state": after.state,
                    "status_after_code": after.code,
                    "binding_count": plan.bindings.len(),
                    "required_binding_count": plan.required_binding_ids.len(),
                    "dependency_hints": {
                        "model_dir_exists": hints.model_dir_exists,
                        "has_modeling_py": hints.has_modeling_py,
                        "has_auto_map": hints.has_auto_map,
                        "requirements_files": hints.requirements_files,
                        "import_hints": hints.import_hints,
                    },
                    "backend_binding_matrix": backend_matrix,
                }));
            }
            Err(err) => {
                println!("  flow-error={}", err);
                summary.push(serde_json::json!({
                    "label": scenario.label,
                    "model_id": scenario.record.id,
                    "dependency_hints": {
                        "model_dir_exists": hints.model_dir_exists,
                        "has_modeling_py": hints.has_modeling_py,
                        "has_auto_map": hints.has_auto_map,
                        "requirements_files": hints.requirements_files,
                        "import_hints": hints.import_hints,
                    },
                    "backend_binding_matrix": backend_matrix,
                    "error": err,
                }));
            }
        }
    }

    let blocked = summary
        .iter()
        .filter(|row| {
            row.get("status_after_state")
                .and_then(|v| serde_json::from_value::<DependencyState>(v.clone()).ok())
                .is_some_and(|state| state != DependencyState::Ready)
        })
        .count();
    println!(
        "\nsummary_rows={} blocked_after_check={}",
        summary.len(),
        blocked
    );

    let scan_all_hits = if scan_all {
        let t_scan = Instant::now();
        let hits = scan_all_binding_hits(api.as_ref(), &models, &platform).await;
        println!(
            "scan_all_models={} scan_binding_hits={} elapsed_ms={}",
            models.len(),
            hits.len(),
            t_scan.elapsed().as_millis()
        );
        if hits.is_empty() {
            println!("scan_result=no_non_empty_bindings_found");
        } else {
            for row in &hits {
                let model_id = row
                    .get("model_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let backend_key = row
                    .get("backend_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let binding_count = row
                    .get("binding_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                println!(
                    "  scan_hit: model_id={} backend_key={} binding_count={}",
                    model_id, backend_key, binding_count
                );
            }
        }
        Some(hits)
    } else {
        None
    };

    if output_json {
        if let Some(hits) = scan_all_hits {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "summary": summary,
                    "scan_all_binding_hits": hits,
                }))?
            );
        } else {
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
    }

    Ok(())
}
