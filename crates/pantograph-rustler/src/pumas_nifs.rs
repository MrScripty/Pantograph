use std::path::PathBuf;
use std::sync::Arc;

use rustler::{Atom, NifResult, ResourceArc};

use crate::atoms;
use crate::resources::{PumasApiResource, WorkflowExecutorResource};

pub(crate) fn api_discover() -> NifResult<ResourceArc<PumasApiResource>> {
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Runtime error: {}", e))))?;

    let api = runtime
        .block_on(async { pumas_library::PumasApi::discover().await })
        .map_err(|e| rustler::Error::Term(Box::new(format!("PumasApi discover error: {}", e))))?;

    Ok(ResourceArc::new(PumasApiResource {
        api: Arc::new(api),
        runtime: Arc::new(runtime),
    }))
}

pub(crate) fn api_new(launcher_root_path: String) -> NifResult<ResourceArc<PumasApiResource>> {
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Runtime error: {}", e))))?;

    let api = runtime
        .block_on(async {
            pumas_library::PumasApi::builder(&launcher_root_path)
                .auto_create_dirs(true)
                .with_hf_client(true)
                .with_process_manager(false)
                .build()
                .await
        })
        .map_err(|e| rustler::Error::Term(Box::new(format!("PumasApi init error: {}", e))))?;

    Ok(ResourceArc::new(PumasApiResource {
        api: Arc::new(api),
        runtime: Arc::new(runtime),
    }))
}

pub(crate) fn executor_set_pumas_api(
    executor_resource: ResourceArc<WorkflowExecutorResource>,
    pumas_resource: ResourceArc<PumasApiResource>,
) -> NifResult<Atom> {
    let rt = &executor_resource.runtime;

    rt.block_on(async {
        let mut exec = executor_resource.executor.write().await;
        exec.extensions_mut().set(
            node_engine::extension_keys::PUMAS_API,
            pumas_resource.api.clone(),
        );
    });

    Ok(atoms::ok())
}

pub(crate) fn executor_set_kv_cache_store(
    executor_resource: ResourceArc<WorkflowExecutorResource>,
    cache_dir: String,
) -> NifResult<Atom> {
    let rt = &executor_resource.runtime;
    rt.block_on(async {
        let mut exec = executor_resource.executor.write().await;
        let store = Arc::new(inference::kv_cache::KvCacheStore::new(
            PathBuf::from(&cache_dir),
            inference::kv_cache::StoragePolicy::MemoryAndDisk,
        ));
        exec.extensions_mut()
            .set(node_engine::extension_keys::KV_CACHE_STORE, store);
    });
    Ok(atoms::ok())
}

pub(crate) fn list_models(resource: ResourceArc<PumasApiResource>) -> NifResult<String> {
    resource
        .runtime
        .block_on(async { resource.api.list_models().await })
        .map_err(|e| rustler::Error::Term(Box::new(format!("list_models error: {}", e))))
        .and_then(|models| {
            serde_json::to_string(&models)
                .map_err(|e| rustler::Error::Term(Box::new(format!("JSON error: {}", e))))
        })
}

pub(crate) fn search_models(
    resource: ResourceArc<PumasApiResource>,
    query: String,
    limit: usize,
    offset: usize,
) -> NifResult<String> {
    resource
        .runtime
        .block_on(async { resource.api.search_models(&query, limit, offset).await })
        .map_err(|e| rustler::Error::Term(Box::new(format!("search_models error: {}", e))))
        .and_then(|result| {
            serde_json::to_string(&result)
                .map_err(|e| rustler::Error::Term(Box::new(format!("JSON error: {}", e))))
        })
}

pub(crate) fn get_model(
    resource: ResourceArc<PumasApiResource>,
    model_id: String,
) -> NifResult<Option<String>> {
    let model = resource
        .runtime
        .block_on(async { resource.api.get_model(&model_id).await })
        .map_err(|e| rustler::Error::Term(Box::new(format!("get_model error: {}", e))))?;

    match model {
        Some(m) => {
            let json = serde_json::to_string(&m)
                .map_err(|e| rustler::Error::Term(Box::new(format!("JSON error: {}", e))))?;
            Ok(Some(json))
        }
        None => Ok(None),
    }
}

pub(crate) fn rebuild_index(resource: ResourceArc<PumasApiResource>) -> NifResult<usize> {
    resource
        .runtime
        .block_on(async { resource.api.rebuild_model_index().await })
        .map_err(|e| rustler::Error::Term(Box::new(format!("rebuild_index error: {}", e))))
}

pub(crate) fn search_hf(
    resource: ResourceArc<PumasApiResource>,
    query: String,
    kind: Option<String>,
    limit: usize,
) -> NifResult<String> {
    resource
        .runtime
        .block_on(async {
            resource
                .api
                .search_hf_models(&query, kind.as_deref(), limit)
                .await
        })
        .map_err(|e| rustler::Error::Term(Box::new(format!("search_hf error: {}", e))))
        .and_then(|models| {
            serde_json::to_string(&models)
                .map_err(|e| rustler::Error::Term(Box::new(format!("JSON error: {}", e))))
        })
}

pub(crate) fn get_repo_files(
    resource: ResourceArc<PumasApiResource>,
    repo_id: String,
) -> NifResult<String> {
    resource
        .runtime
        .block_on(async { resource.api.get_hf_repo_files(&repo_id).await })
        .map_err(|e| rustler::Error::Term(Box::new(format!("get_repo_files error: {}", e))))
        .and_then(|tree| {
            serde_json::to_string(&tree)
                .map_err(|e| rustler::Error::Term(Box::new(format!("JSON error: {}", e))))
        })
}

pub(crate) fn start_download(
    resource: ResourceArc<PumasApiResource>,
    request_json: String,
) -> NifResult<String> {
    let request: pumas_library::model_library::DownloadRequest =
        serde_json::from_str(&request_json)
            .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    resource
        .runtime
        .block_on(async { resource.api.start_hf_download(&request).await })
        .map_err(|e| rustler::Error::Term(Box::new(format!("start_download error: {}", e))))
}

pub(crate) fn get_download_progress(
    resource: ResourceArc<PumasApiResource>,
    download_id: String,
) -> NifResult<Option<String>> {
    let progress = resource
        .runtime
        .block_on(async { resource.api.get_hf_download_progress(&download_id).await });

    match progress {
        Some(p) => {
            let json = serde_json::to_string(&p)
                .map_err(|e| rustler::Error::Term(Box::new(format!("JSON error: {}", e))))?;
            Ok(Some(json))
        }
        None => Ok(None),
    }
}

pub(crate) fn cancel_download(
    resource: ResourceArc<PumasApiResource>,
    download_id: String,
) -> NifResult<bool> {
    resource
        .runtime
        .block_on(async { resource.api.cancel_hf_download(&download_id).await })
        .map_err(|e| rustler::Error::Term(Box::new(format!("cancel_download error: {}", e))))
}

pub(crate) fn import_model(
    resource: ResourceArc<PumasApiResource>,
    spec_json: String,
) -> NifResult<String> {
    let spec: pumas_library::model_library::ModelImportSpec = serde_json::from_str(&spec_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let result = resource
        .runtime
        .block_on(async { resource.api.import_model(&spec).await })
        .map_err(|e| rustler::Error::Term(Box::new(format!("import_model error: {}", e))))?;

    serde_json::to_string(&result)
        .map_err(|e| rustler::Error::Term(Box::new(format!("JSON error: {}", e))))
}

pub(crate) fn import_batch(
    resource: ResourceArc<PumasApiResource>,
    specs_json: String,
) -> NifResult<String> {
    let specs: Vec<pumas_library::model_library::ModelImportSpec> =
        serde_json::from_str(&specs_json)
            .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let results = resource
        .runtime
        .block_on(async { resource.api.import_models_batch(specs).await });

    serde_json::to_string(&results)
        .map_err(|e| rustler::Error::Term(Box::new(format!("JSON error: {}", e))))
}

pub(crate) fn get_disk_space(resource: ResourceArc<PumasApiResource>) -> NifResult<String> {
    resource
        .runtime
        .block_on(async { resource.api.get_disk_space().await })
        .map_err(|e| rustler::Error::Term(Box::new(format!("get_disk_space error: {}", e))))
        .and_then(|info| {
            serde_json::to_string(&info)
                .map_err(|e| rustler::Error::Term(Box::new(format!("JSON error: {}", e))))
        })
}

pub(crate) fn get_system_resources(resource: ResourceArc<PumasApiResource>) -> NifResult<String> {
    resource
        .runtime
        .block_on(async { resource.api.get_system_resources().await })
        .map_err(|e| rustler::Error::Term(Box::new(format!("get_system_resources error: {}", e))))
        .and_then(|info| {
            serde_json::to_string(&info)
                .map_err(|e| rustler::Error::Term(Box::new(format!("JSON error: {}", e))))
        })
}

pub(crate) fn is_ollama_running(resource: ResourceArc<PumasApiResource>) -> bool {
    resource
        .runtime
        .block_on(async { resource.api.is_ollama_running().await })
}
