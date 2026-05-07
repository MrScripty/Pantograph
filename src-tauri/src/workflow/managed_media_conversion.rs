use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_media_conversion::{
    acquire_managed_media_dependency_plan, format_managed_media_dependency_lease_holder,
    release_managed_media_dependency_plan, resolve_managed_media_dependency_executable_path,
    ConversionMediaKind, ManagedExecutablePath, ManagedMediaDependency, ManagedMediaDependencyId,
    ManagedMediaDependencyLease, ManagedMediaDependencyLeaseId, ManagedMediaDependencyPlan,
    ManagedMediaDependencyPlanRequest, ManagedMediaDependencyVersion, MediaCommandPlan,
    MediaConversionDependencyAttribution, MediaConversionError, MediaConversionExecutor,
    MediaConversionRequest, MediaConversionResult, MediaConversionStatus, ProcessRunRequest,
    ProcessRunner, StdProcessRunner,
};

#[derive(Clone)]
pub struct TauriManagedMediaConversionExecutor<R = StdProcessRunner> {
    app_data_dir: PathBuf,
    runner: Arc<R>,
}

impl TauriManagedMediaConversionExecutor<StdProcessRunner> {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self::with_runner(app_data_dir, Arc::new(StdProcessRunner))
    }
}

impl<R> TauriManagedMediaConversionExecutor<R>
where
    R: ProcessRunner,
{
    pub fn with_runner(app_data_dir: PathBuf, runner: Arc<R>) -> Self {
        Self {
            app_data_dir,
            runner,
        }
    }

    async fn convert_with_plan(
        &self,
        request: MediaConversionRequest,
        command_plan: MediaCommandPlan,
        dependency_plan: &ManagedMediaDependencyPlan,
    ) -> Result<MediaConversionResult, MediaConversionError> {
        let mut stdin = request.source.body.clone();
        let mut stderr_summaries = Vec::new();

        for step in &command_plan.steps {
            let dependency = dependency_for_step(dependency_plan, step.dependency_id)?;
            let executable_path = managed_executable_path(&dependency.dependency)?;
            let process_request = ProcessRunRequest::try_new(
                executable_path,
                step.argv.clone(),
                stdin,
                request.timeout_ms,
            )?;
            let output = self.runner.run(process_request).await?;
            if !output.successful() {
                return Err(MediaConversionError::ProcessFailed {
                    status_code: output.status_code,
                    stderr_summary: output
                        .stderr_summary
                        .unwrap_or_else(|| "no stderr captured".to_string()),
                });
            }
            if let Some(stderr_summary) = output.stderr_summary {
                stderr_summaries.push(stderr_summary);
            }
            stdin = output.stdout;
        }

        MediaConversionResult::try_new(
            request.conversion_id,
            MediaConversionStatus::Converted,
            command_plan.target.media_type.clone(),
            command_plan.target.clone(),
            command_id_for_plan(&command_plan),
            stdin,
            dependency_attributions(dependency_plan)?,
            joined_stderr_summary(stderr_summaries),
        )
    }
}

#[async_trait]
impl<R> MediaConversionExecutor for TauriManagedMediaConversionExecutor<R>
where
    R: ProcessRunner + 'static,
{
    async fn convert(
        &self,
        request: MediaConversionRequest,
    ) -> Result<MediaConversionResult, MediaConversionError> {
        let command_plan = MediaCommandPlan::try_for_target(request.kind, request.target.clone())?;
        let plan_request = ManagedMediaDependencyPlanRequest {
            kind: command_plan.kind,
            color_managed: command_plan.target.color_managed,
            holder: lease_holder_for_request(&request)?,
        };
        let dependency_plan =
            acquire_managed_media_dependency_plan(self.app_data_dir.as_path(), plan_request)
                .map_err(|reason| MediaConversionError::DependencyUnavailable {
                    dependency_id: primary_dependency_id(&command_plan),
                    reason,
                })?;
        let dependency_plan_guard =
            MediaConversionDependencyPlanGuard::new(self.app_data_dir.clone(), dependency_plan);

        let conversion_result = self
            .convert_with_plan(request, command_plan, dependency_plan_guard.plan())
            .await;
        release_after_conversion(dependency_plan_guard, conversion_result)
    }
}

struct MediaConversionDependencyPlanGuard {
    app_data_dir: PathBuf,
    dependency_plan: Option<ManagedMediaDependencyPlan>,
}

impl MediaConversionDependencyPlanGuard {
    fn new(app_data_dir: PathBuf, dependency_plan: ManagedMediaDependencyPlan) -> Self {
        Self {
            app_data_dir,
            dependency_plan: Some(dependency_plan),
        }
    }

    fn plan(&self) -> &ManagedMediaDependencyPlan {
        self.dependency_plan
            .as_ref()
            .expect("dependency plan guard should hold a plan until release")
    }

    fn release(mut self) -> Result<(), String> {
        self.release_inner()
    }

    fn release_inner(&mut self) -> Result<(), String> {
        let Some(dependency_plan) = self.dependency_plan.take() else {
            return Ok(());
        };
        release_managed_media_dependency_plan(self.app_data_dir.as_path(), &dependency_plan)
    }
}

impl Drop for MediaConversionDependencyPlanGuard {
    fn drop(&mut self) {
        let _ = self.release_inner();
    }
}

fn release_after_conversion(
    dependency_plan_guard: MediaConversionDependencyPlanGuard,
    conversion_result: Result<MediaConversionResult, MediaConversionError>,
) -> Result<MediaConversionResult, MediaConversionError> {
    let release_result = dependency_plan_guard.release();
    match (conversion_result, release_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Ok(_), Err(release_error)) => Err(MediaConversionError::Io {
            message: format!("failed to release media conversion dependency leases: {release_error}"),
        }),
        (Err(conversion_error), Ok(())) => Err(conversion_error),
        (Err(conversion_error), Err(release_error)) => Err(MediaConversionError::Io {
            message: format!(
                "media conversion failed: {conversion_error}; additionally failed to release dependency leases: {release_error}"
            ),
        }),
    }
}

fn managed_executable_path(
    dependency: &ManagedMediaDependency,
) -> Result<ManagedExecutablePath, MediaConversionError> {
    let executable_path =
        resolve_managed_media_dependency_executable_path(dependency).map_err(|reason| {
            MediaConversionError::DependencyUnavailable {
                dependency_id: dependency.id,
                reason,
            }
        })?;
    ManagedExecutablePath::try_new(executable_path)
}

fn dependency_for_step(
    plan: &ManagedMediaDependencyPlan,
    dependency_id: ManagedMediaDependencyId,
) -> Result<&ManagedMediaDependencyLease, MediaConversionError> {
    plan.leases
        .iter()
        .find(|lease| lease.token.id == dependency_id)
        .ok_or_else(|| MediaConversionError::DependencyUnavailable {
            dependency_id,
            reason: "managed dependency lease was not included in the acquired plan".to_string(),
        })
}

fn dependency_attributions(
    plan: &ManagedMediaDependencyPlan,
) -> Result<Vec<MediaConversionDependencyAttribution>, MediaConversionError> {
    plan.leases
        .iter()
        .map(|lease| {
            Ok(MediaConversionDependencyAttribution {
                dependency_id: lease.token.id,
                version: ManagedMediaDependencyVersion::try_from(lease.token.version.clone())?,
                lease_id: ManagedMediaDependencyLeaseId::try_from(lease.token.lease_id.clone())?,
                lease_holder: lease.token.holder.clone(),
            })
        })
        .collect()
}

fn lease_holder_for_request(
    request: &MediaConversionRequest,
) -> Result<String, MediaConversionError> {
    let node_id = request
        .attribution
        .node_id
        .as_ref()
        .map(|node_id| node_id.as_str())
        .unwrap_or("workflow");
    let port_id = request
        .attribution
        .port_id
        .as_ref()
        .map(|port_id| port_id.as_str())
        .unwrap_or("output");
    format_managed_media_dependency_lease_holder(
        request.attribution.workflow_run_id.as_str(),
        node_id,
        port_id,
        request.conversion_id.as_str(),
    )
    .map_err(|reason| MediaConversionError::Io { message: reason })
}

fn command_id_for_plan(command_plan: &MediaCommandPlan) -> String {
    let dependencies = command_plan
        .steps
        .iter()
        .map(|step| step.dependency_id.to_string())
        .collect::<Vec<_>>()
        .join("_");
    format!("{}_{}", media_kind_key(command_plan.kind), dependencies)
}

fn joined_stderr_summary(stderr_summaries: Vec<String>) -> Option<String> {
    if stderr_summaries.is_empty() {
        None
    } else {
        Some(stderr_summaries.join("; "))
    }
}

fn primary_dependency_id(command_plan: &MediaCommandPlan) -> ManagedMediaDependencyId {
    command_plan
        .required_dependency_ids
        .first()
        .copied()
        .unwrap_or(ManagedMediaDependencyId::Ffmpeg)
}

fn media_kind_key(kind: ConversionMediaKind) -> &'static str {
    match kind {
        ConversionMediaKind::Image => "image",
        ConversionMediaKind::Audio => "audio",
        ConversionMediaKind::Video => "video",
        ConversionMediaKind::ThreeD => "3d",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::fs;
    use std::path::Path;
    use std::sync::Mutex;

    use pantograph_managed_dependencies::{
        activate_managed_redistributable_version, install_managed_redistributable_from_staging,
        load_managed_redistributable_state, managed_redistributable_catalog_entry,
        remove_managed_redistributable_version, ManagedRedistributableId,
    };
    use pantograph_media_conversion::{
        ArtifactId, FormatField, GraphNodeId, MediaConversionAttribution, MediaConversionId,
        MediaConversionSource, MediaConversionTarget, MediaType, PortId, ProcessRunOutput,
        WorkflowRunId,
    };
    use tokio::sync::Notify;

    use super::*;

    struct FakeProcessRunner {
        calls: Mutex<Vec<ProcessRunRequest>>,
        outputs: Mutex<VecDeque<ProcessRunOutput>>,
        run_started: Notify,
        wait_forever: bool,
    }

    impl FakeProcessRunner {
        fn with_outputs(outputs: Vec<ProcessRunOutput>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                outputs: Mutex::new(outputs.into()),
                run_started: Notify::new(),
                wait_forever: false,
            }
        }

        fn pending() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                outputs: Mutex::new(VecDeque::new()),
                run_started: Notify::new(),
                wait_forever: true,
            }
        }

        fn calls(&self) -> Vec<ProcessRunRequest> {
            self.calls.lock().expect("calls lock").clone()
        }

        async fn wait_for_run_started(&self) {
            self.run_started.notified().await;
        }
    }

    #[async_trait]
    impl ProcessRunner for FakeProcessRunner {
        async fn run(
            &self,
            request: ProcessRunRequest,
        ) -> Result<ProcessRunOutput, MediaConversionError> {
            self.calls.lock().expect("calls lock").push(request);
            self.run_started.notify_one();
            if self.wait_forever {
                std::future::pending::<()>().await;
            }
            self.outputs
                .lock()
                .expect("outputs lock")
                .pop_front()
                .ok_or_else(|| MediaConversionError::Io {
                    message: "fake runner has no queued output".to_string(),
                })
        }
    }

    #[tokio::test]
    async fn managed_executor_acquires_releases_and_attributes_audio_dependency() {
        let app_data_dir = tempfile::tempdir().expect("app data dir");
        install_active_dependency(app_data_dir.path(), ManagedRedistributableId::Ffmpeg);
        let runner = Arc::new(FakeProcessRunner::with_outputs(vec![
            ProcessRunOutput::new(Some(0), b"converted audio".to_vec(), b"ffmpeg stderr"),
        ]));
        let executor = TauriManagedMediaConversionExecutor::with_runner(
            app_data_dir.path().to_path_buf(),
            runner.clone(),
        );

        let result = executor
            .convert(media_conversion_request(
                ConversionMediaKind::Audio,
                "audio/wav",
                "ogg",
                "audio/ogg",
                Some("opus"),
                false,
            ))
            .await
            .expect("convert audio");

        assert_eq!(result.body, b"converted audio");
        assert_eq!(result.dependencies.len(), 1);
        assert_eq!(
            result.dependencies[0].dependency_id,
            ManagedMediaDependencyId::Ffmpeg
        );
        assert!(result.dependencies[0]
            .lease_holder
            .contains("workflow_run:run_test"));
        assert!(runner.calls()[0]
            .executable_path
            .as_path()
            .ends_with("ffmpeg"));
        assert_active_leases_released(app_data_dir.path());
    }

    #[tokio::test]
    async fn managed_executor_pipelines_color_managed_image_dependencies() {
        let app_data_dir = tempfile::tempdir().expect("app data dir");
        install_active_dependency(app_data_dir.path(), ManagedRedistributableId::Oiiotool);
        install_active_dependency(app_data_dir.path(), ManagedRedistributableId::Ocioconvert);
        install_active_dependency(app_data_dir.path(), ManagedRedistributableId::OpenColorIo);
        let runner = Arc::new(FakeProcessRunner::with_outputs(vec![
            ProcessRunOutput::new(Some(0), b"color managed".to_vec(), b"ocio"),
            ProcessRunOutput::new(Some(0), b"encoded image".to_vec(), b"oiio"),
        ]));
        let executor = TauriManagedMediaConversionExecutor::with_runner(
            app_data_dir.path().to_path_buf(),
            runner.clone(),
        );

        let result = executor
            .convert(media_conversion_request(
                ConversionMediaKind::Image,
                "image/png",
                "jpg",
                "image/jpeg",
                None,
                true,
            ))
            .await
            .expect("convert image");

        let calls = runner.calls();
        assert_eq!(calls.len(), 2);
        assert!(calls[0].executable_path.as_path().ends_with("ocioconvert"));
        assert_eq!(calls[1].stdin, b"color managed");
        assert!(calls[1].executable_path.as_path().ends_with("oiiotool"));
        assert_eq!(result.body, b"encoded image");
        assert_eq!(result.dependencies.len(), 3);
        assert!(result
            .dependencies
            .iter()
            .any(|dependency| dependency.dependency_id == ManagedMediaDependencyId::OpenColorIo));
        assert_active_leases_released(app_data_dir.path());
    }

    #[tokio::test]
    async fn managed_executor_releases_dependency_when_conversion_future_is_aborted() {
        let app_data_dir = tempfile::tempdir().expect("app data dir");
        install_active_dependency(app_data_dir.path(), ManagedRedistributableId::Ffmpeg);
        let runner = Arc::new(FakeProcessRunner::pending());
        let executor = TauriManagedMediaConversionExecutor::with_runner(
            app_data_dir.path().to_path_buf(),
            runner.clone(),
        );

        let convert_task = tokio::spawn(async move {
            executor
                .convert(media_conversion_request(
                    ConversionMediaKind::Audio,
                    "audio/wav",
                    "ogg",
                    "audio/ogg",
                    Some("opus"),
                    false,
                ))
                .await
        });

        runner.wait_for_run_started().await;
        assert_eq!(runner.calls().len(), 1);
        assert_active_leases_present(app_data_dir.path());

        convert_task.abort();
        assert!(convert_task
            .await
            .expect_err("convert task should abort")
            .is_cancelled());
        assert_active_leases_released(app_data_dir.path());
    }

    #[tokio::test]
    async fn managed_executor_blocks_dependency_removal_while_conversion_lease_is_active() {
        let app_data_dir = tempfile::tempdir().expect("app data dir");
        let version =
            install_active_dependency(app_data_dir.path(), ManagedRedistributableId::Ffmpeg);
        let runner = Arc::new(FakeProcessRunner::pending());
        let executor = TauriManagedMediaConversionExecutor::with_runner(
            app_data_dir.path().to_path_buf(),
            runner.clone(),
        );

        let convert_task = tokio::spawn(async move {
            executor
                .convert(media_conversion_request(
                    ConversionMediaKind::Audio,
                    "audio/wav",
                    "ogg",
                    "audio/ogg",
                    Some("opus"),
                    false,
                ))
                .await
        });

        runner.wait_for_run_started().await;
        let removal_error = remove_managed_redistributable_version(
            app_data_dir.path(),
            ManagedRedistributableId::Ffmpeg,
            &version,
        )
        .expect_err("active leased dependency removal should fail");
        assert!(removal_error.contains("while 1 lease(s) exist"));

        convert_task.abort();
        assert!(convert_task
            .await
            .expect_err("convert task should abort")
            .is_cancelled());
        assert_active_leases_released(app_data_dir.path());
        remove_managed_redistributable_version(
            app_data_dir.path(),
            ManagedRedistributableId::Ffmpeg,
            &version,
        )
        .expect("dependency removal succeeds after lease release");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn managed_executor_runs_unix_managed_executable_fixture() {
        let app_data_dir = tempfile::tempdir().expect("app data dir");
        install_active_executable_dependency(
            app_data_dir.path(),
            ManagedRedistributableId::Ffmpeg,
            "#!/bin/sh\ncat >/dev/null\nprintf fixture-converted\n",
        );
        let executor = TauriManagedMediaConversionExecutor::new(app_data_dir.path().to_path_buf());

        let result = executor
            .convert(media_conversion_request(
                ConversionMediaKind::Audio,
                "audio/wav",
                "ogg",
                "audio/ogg",
                Some("opus"),
                false,
            ))
            .await
            .expect("convert through executable fixture");

        assert_eq!(result.body, b"fixture-converted");
        assert_eq!(result.dependencies.len(), 1);
        assert_eq!(
            result.dependencies[0].dependency_id,
            ManagedMediaDependencyId::Ffmpeg
        );
        assert_active_leases_released(app_data_dir.path());
    }

    fn media_conversion_request(
        kind: ConversionMediaKind,
        source_media_type: &str,
        format_id: &str,
        target_media_type: &str,
        codec_id: Option<&str>,
        color_managed: bool,
    ) -> MediaConversionRequest {
        MediaConversionRequest::try_new(
            MediaConversionId::try_from("conversion_test".to_string()).expect("conversion id"),
            kind,
            MediaConversionAttribution {
                workflow_run_id: WorkflowRunId::try_from("run_test".to_string())
                    .expect("workflow run id"),
                source_artifact_id: ArtifactId::try_from("artifact_source".to_string())
                    .expect("artifact id"),
                node_id: Some(GraphNodeId::try_from("node_image".to_string()).expect("node id")),
                port_id: Some(PortId::try_from("port_output".to_string()).expect("port id")),
            },
            MediaConversionSource::try_new(
                ArtifactId::try_from("artifact_source".to_string()).expect("artifact id"),
                MediaType::try_from(source_media_type.to_string()).expect("source media type"),
                b"source bytes".to_vec(),
            )
            .expect("source"),
            MediaConversionTarget::try_new(
                FormatField::try_from(format_id.to_string()).expect("format id"),
                MediaType::try_from(target_media_type.to_string()).expect("target media type"),
                codec_id.map(|codec| FormatField::try_from(codec.to_string()).expect("codec id")),
                Some(75),
                Some(96),
                None,
                None,
                Some(FormatField::try_from("srgb".to_string()).expect("color profile")),
                color_managed,
            )
            .expect("target"),
            Some(60_000),
        )
        .expect("request")
    }

    fn install_active_dependency(app_data_dir: &Path, id: ManagedRedistributableId) -> String {
        let staging_dir = tempfile::tempdir().expect("staging dir");
        let catalog = managed_redistributable_catalog_entry(id);
        for expected_file in &catalog.expected_files {
            let path = staging_dir.path().join(expected_file);
            fs::create_dir_all(path.parent().expect("expected file parent"))
                .expect("create expected file parent");
            fs::write(path, b"stub").expect("write expected file");
        }
        install_managed_redistributable_from_staging(
            app_data_dir,
            id,
            &catalog.version,
            staging_dir.path(),
        )
        .expect("install dependency");
        activate_managed_redistributable_version(app_data_dir, id, &catalog.version)
            .expect("activate dependency");
        catalog.version
    }

    #[cfg(unix)]
    fn install_active_executable_dependency(
        app_data_dir: &Path,
        id: ManagedRedistributableId,
        executable_contents: &str,
    ) -> String {
        use std::os::unix::fs::PermissionsExt;

        let staging_dir = tempfile::tempdir().expect("staging dir");
        let catalog = managed_redistributable_catalog_entry(id);
        for expected_file in &catalog.expected_files {
            let path = staging_dir.path().join(expected_file);
            fs::create_dir_all(path.parent().expect("expected file parent"))
                .expect("create expected file parent");
            fs::write(&path, executable_contents).expect("write executable fixture");
            let mut permissions = fs::metadata(&path).expect("fixture metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).expect("mark executable fixture");
        }
        install_managed_redistributable_from_staging(
            app_data_dir,
            id,
            &catalog.version,
            staging_dir.path(),
        )
        .expect("install executable dependency");
        activate_managed_redistributable_version(app_data_dir, id, &catalog.version)
            .expect("activate executable dependency");
        catalog.version
    }

    fn assert_active_leases_released(app_data_dir: &Path) {
        let state = load_managed_redistributable_state(app_data_dir).expect("load state");
        for dependency in state.dependencies {
            assert!(
                dependency.active_leases.is_empty(),
                "dependency {:?} should not retain active leases",
                dependency.id
            );
        }
    }

    fn assert_active_leases_present(app_data_dir: &Path) {
        let state = load_managed_redistributable_state(app_data_dir).expect("load state");
        assert!(
            state
                .dependencies
                .iter()
                .any(|dependency| !dependency.active_leases.is_empty()),
            "at least one dependency should retain an active lease"
        );
    }
}
