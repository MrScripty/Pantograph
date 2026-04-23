use super::*;

impl TauriModelDependencyResolver {
    pub(super) async fn pip_show_version(
        python: &Path,
        package_name: &str,
    ) -> Result<Option<String>, String> {
        let output = Command::new(python)
            .arg("-m")
            .arg("pip")
            .arg("show")
            .arg(package_name)
            .output()
            .await
            .map_err(|err| format!("Failed to run pip show for '{package_name}': {err}"))?;

        if !output.status.success() {
            return Ok(None);
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(version) = line.strip_prefix("Version:") {
                let trimmed = version.trim();
                if !trimmed.is_empty() {
                    return Ok(Some(trimmed.to_string()));
                }
            }
        }
        Ok(None)
    }

    pub(super) async fn is_requirement_satisfied(
        python: &Path,
        requirement: &ModelDependencyRequirement,
    ) -> Result<bool, String> {
        let installed = Self::pip_show_version(python, &requirement.name).await?;
        let Some(installed_version) = installed else {
            return Ok(false);
        };
        Ok(requirements::normalize_exact_pin(&requirement.exact_pin) == installed_version.trim())
    }

    pub(super) async fn consume_install_stream<R>(
        reader: R,
        stream_name: &'static str,
        emitter: Option<DependencyActivityEmitter>,
        context: DependencyActivityContext,
        binding_id: String,
        requirement_name: String,
    ) -> Vec<String>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut captured = Vec::new();
        let mut lines = BufReader::new(reader).lines();
        while let Ok(next) = lines.next_line().await {
            let Some(line) = next else {
                break;
            };
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            captured.push(trimmed.clone());
            Self::emit_activity_with_emitter(
                emitter.as_ref(),
                &context,
                "install_stream",
                trimmed,
                Some(&binding_id),
                Some(&requirement_name),
                Some(stream_name),
            );
        }
        captured
    }

    pub(super) async fn pip_install_requirement(
        &self,
        python: &Path,
        requirement: &ModelDependencyRequirement,
        context: Option<&DependencyActivityContext>,
        binding_id: &str,
    ) -> Result<(), String> {
        let spec = Self::requirement_install_target(requirement);
        if let Some(context) = context {
            self.emit_activity(
                context,
                "install",
                format!("pip install {}", spec),
                Some(binding_id),
                Some(&requirement.name),
                None,
            );
        }

        let mut command = Command::new(python);
        command
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg("--disable-pip-version-check")
            .arg(spec)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(index_url) = requirement.index_url.as_deref() {
            let trimmed = index_url.trim();
            if !trimmed.is_empty() {
                command.arg("--index-url").arg(trimmed);
            }
        }
        for extra in &requirement.extra_index_urls {
            let trimmed = extra.trim();
            if !trimmed.is_empty() {
                command.arg("--extra-index-url").arg(trimmed);
            }
        }

        let mut child = command.spawn().map_err(|err| {
            format!(
                "Failed to run pip install for '{}': {}",
                requirement.name, err
            )
        })?;

        let emitter = self.current_activity_emitter();
        let context_value = context
            .cloned()
            .unwrap_or_else(DependencyActivityContext::unknown);
        let stdout_task = child.stdout.take().map(|stdout| {
            tokio::spawn(Self::consume_install_stream(
                stdout,
                "stdout",
                emitter.clone(),
                context_value.clone(),
                binding_id.to_string(),
                requirement.name.clone(),
            ))
        });
        let stderr_task = child.stderr.take().map(|stderr| {
            tokio::spawn(Self::consume_install_stream(
                stderr,
                "stderr",
                emitter.clone(),
                context_value.clone(),
                binding_id.to_string(),
                requirement.name.clone(),
            ))
        });

        let status = child.wait().await.map_err(|err| {
            format!(
                "Failed waiting for pip install process for '{}': {}",
                requirement.name, err
            )
        })?;

        let stdout_lines = match stdout_task {
            Some(handle) => handle.await.unwrap_or_default(),
            None => Vec::new(),
        };
        let stderr_lines = match stderr_task {
            Some(handle) => handle.await.unwrap_or_default(),
            None => Vec::new(),
        };

        if status.success() {
            if let Some(context) = context {
                self.emit_activity(
                    context,
                    "install",
                    "pip install completed",
                    Some(binding_id),
                    Some(&requirement.name),
                    None,
                );
            }
            return Ok(());
        }

        let details = if !stderr_lines.is_empty() {
            stderr_lines.join(" | ")
        } else {
            stdout_lines.join(" | ")
        };
        let message = format!("pip install failed for '{}': {}", requirement.name, details);
        if let Some(context) = context {
            self.emit_activity(
                context,
                "install",
                message.clone(),
                Some(binding_id),
                Some(&requirement.name),
                None,
            );
        }
        Err(message)
    }

    pub(super) async fn check_binding_with_python(
        &self,
        binding: &ModelDependencyBinding,
        python_override: Option<&Path>,
        context: Option<&DependencyActivityContext>,
    ) -> ModelDependencyBindingStatus {
        if let Some(context) = context {
            self.emit_activity(
                context,
                "check",
                "checking binding requirements",
                Some(&binding.binding_id),
                None,
                None,
            );
        }

        if binding.validation_state != DependencyValidationState::Resolved {
            let state =
                requirements::runtime_state_from_validation(binding.validation_state.clone());
            let code = binding.validation_errors.first().map(|e| e.code.clone());
            let message = binding.validation_errors.first().map(|e| e.message.clone());
            let row = ModelDependencyBindingStatus {
                binding_id: binding.binding_id.clone(),
                env_id: binding.env_id.clone(),
                state,
                code,
                message,
                missing_requirements: Vec::new(),
                installed_requirements: Vec::new(),
                failed_requirements: Vec::new(),
            };
            if let Some(context) = context {
                self.emit_activity(
                    context,
                    "check",
                    format!(
                        "binding state={} code={}",
                        serde_json::to_value(&row.state)
                            .ok()
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| "unknown".to_string()),
                        row.code.clone().unwrap_or_else(|| "none".to_string())
                    ),
                    Some(&binding.binding_id),
                    None,
                    None,
                );
            }
            return row;
        }

        if binding.env_id.as_deref().unwrap_or("").trim().is_empty() {
            let row = ModelDependencyBindingStatus {
                binding_id: binding.binding_id.clone(),
                env_id: binding.env_id.clone(),
                state: DependencyState::Unresolved,
                code: Some("env_id_missing".to_string()),
                message: Some("Dependency binding has no env_id".to_string()),
                missing_requirements: Vec::new(),
                installed_requirements: Vec::new(),
                failed_requirements: Vec::new(),
            };
            if let Some(context) = context {
                self.emit_activity(
                    context,
                    "check",
                    "binding has no env_id",
                    Some(&binding.binding_id),
                    None,
                    None,
                );
            }
            return row;
        }

        let environment_kind = binding
            .environment_kind
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if environment_kind != "python" && environment_kind != "python-venv" {
            let row = ModelDependencyBindingStatus {
                binding_id: binding.binding_id.clone(),
                env_id: binding.env_id.clone(),
                state: DependencyState::Failed,
                code: Some("unsupported_environment_kind".to_string()),
                message: Some(format!(
                    "Unsupported environment_kind '{}'",
                    environment_kind
                )),
                missing_requirements: Vec::new(),
                installed_requirements: Vec::new(),
                failed_requirements: Vec::new(),
            };
            if let Some(context) = context {
                self.emit_activity(
                    context,
                    "check",
                    row.message
                        .clone()
                        .unwrap_or_else(|| "unsupported environment".to_string()),
                    Some(&binding.binding_id),
                    None,
                    None,
                );
            }
            return row;
        }

        let python = if let Some(existing) = python_override {
            existing.to_path_buf()
        } else {
            let env_ids = binding.env_id.clone().into_iter().collect::<Vec<_>>();
            match crate::python_runtime::resolve_python_executable_for_env_ids(&env_ids) {
                Ok(path) => path,
                Err(err) => {
                    let row = ModelDependencyBindingStatus {
                        binding_id: binding.binding_id.clone(),
                        env_id: binding.env_id.clone(),
                        state: DependencyState::Failed,
                        code: Some("python_runtime_unavailable".to_string()),
                        message: Some(err),
                        missing_requirements: Vec::new(),
                        installed_requirements: Vec::new(),
                        failed_requirements: Vec::new(),
                    };
                    if let Some(context) = context {
                        self.emit_activity(
                            context,
                            "check",
                            row.message
                                .clone()
                                .unwrap_or_else(|| "python runtime unavailable".to_string()),
                            Some(&binding.binding_id),
                            None,
                            None,
                        );
                    }
                    return row;
                }
            }
        };

        let mut missing_requirements = Vec::new();
        let mut failed_requirements = Vec::new();
        for requirement in &binding.requirements {
            if requirement.kind != "python_package" {
                failed_requirements.push(requirement.name.clone());
                continue;
            }
            match Self::is_requirement_satisfied(&python, requirement).await {
                Ok(true) => {}
                Ok(false) => missing_requirements.push(requirement.name.clone()),
                Err(_) => failed_requirements.push(requirement.name.clone()),
            }
        }

        let state = if !failed_requirements.is_empty() {
            DependencyState::Failed
        } else if !missing_requirements.is_empty() {
            DependencyState::Missing
        } else {
            DependencyState::Ready
        };
        let code = match state {
            DependencyState::Failed => Some("dependency_check_failed".to_string()),
            DependencyState::Missing => Some("requirements_missing".to_string()),
            _ => None,
        };

        let row = ModelDependencyBindingStatus {
            binding_id: binding.binding_id.clone(),
            env_id: binding.env_id.clone(),
            state,
            code,
            message: None,
            missing_requirements,
            installed_requirements: Vec::new(),
            failed_requirements,
        };
        if let Some(context) = context {
            self.emit_activity(
                context,
                "check",
                format!(
                    "binding state={} missing={} failed={}",
                    serde_json::to_value(&row.state)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "unknown".to_string()),
                    row.missing_requirements.len(),
                    row.failed_requirements.len()
                ),
                Some(&binding.binding_id),
                None,
                None,
            );
        }
        row
    }

    pub(super) async fn check_binding(
        &self,
        binding: &ModelDependencyBinding,
        context: Option<&DependencyActivityContext>,
    ) -> ModelDependencyBindingStatus {
        let python_override = binding
            .python_executable_override
            .as_deref()
            .map(PathBuf::from);
        self.check_binding_with_python(binding, python_override.as_deref(), context)
            .await
    }

    pub(super) async fn install_binding_requirements(
        &self,
        binding: &ModelDependencyBinding,
        context: Option<&DependencyActivityContext>,
    ) -> ModelDependencyBindingStatus {
        let env_id = binding.env_id.clone().unwrap_or_default();
        if env_id.trim().is_empty() {
            return self.check_binding(binding, context).await;
        }

        if let Some(context) = context {
            self.emit_activity(
                context,
                "install",
                "starting binding install",
                Some(&binding.binding_id),
                None,
                None,
            );
        }

        let lock = self.get_or_create_install_lock(&env_id).await;
        let _guard = lock.lock().await;

        if binding.validation_state != DependencyValidationState::Resolved {
            return self.check_binding(binding, context).await;
        }

        let python = if let Some(override_path) = binding.python_executable_override.as_deref() {
            PathBuf::from(override_path)
        } else {
            let env_ids = vec![env_id];
            match crate::python_runtime::resolve_python_executable_for_env_ids(&env_ids) {
                Ok(path) => path,
                Err(err) => {
                    let mut row = self.check_binding(binding, context).await;
                    row.state = DependencyState::Failed;
                    row.code = Some("python_runtime_unavailable".to_string());
                    row.message = Some(err);
                    if let Some(context) = context {
                        self.emit_activity(
                            context,
                            "install",
                            row.message
                                .clone()
                                .unwrap_or_else(|| "python runtime unavailable".to_string()),
                            Some(&binding.binding_id),
                            None,
                            None,
                        );
                    }
                    return row;
                }
            }
        };

        let mut installed_requirements = Vec::new();
        let mut failed_requirements = Vec::new();
        for requirement in &binding.requirements {
            if requirement.kind != "python_package" {
                failed_requirements.push(requirement.name.clone());
                continue;
            }

            match Self::is_requirement_satisfied(&python, requirement).await {
                Ok(true) => {
                    if let Some(context) = context {
                        self.emit_activity(
                            context,
                            "install",
                            "requirement already satisfied",
                            Some(&binding.binding_id),
                            Some(&requirement.name),
                            None,
                        );
                    }
                    continue;
                }
                Ok(false) => {}
                Err(_) => {}
            }

            match self
                .pip_install_requirement(&python, requirement, context, &binding.binding_id)
                .await
            {
                Ok(()) => installed_requirements.push(requirement.name.clone()),
                Err(_) => failed_requirements.push(requirement.name.clone()),
            }
        }

        let mut post_check = self
            .check_binding_with_python(binding, Some(&python), context)
            .await;
        post_check.installed_requirements = installed_requirements;
        if !failed_requirements.is_empty() {
            post_check.failed_requirements.extend(failed_requirements);
            post_check.failed_requirements.sort();
            post_check.failed_requirements.dedup();
            post_check.state = DependencyState::Failed;
            post_check.code = Some("dependency_install_failed".to_string());
        }
        if let Some(context) = context {
            self.emit_activity(
                context,
                "install",
                format!(
                    "binding state={} installed={} failed={}",
                    serde_json::to_value(&post_check.state)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "unknown".to_string()),
                    post_check.installed_requirements.len(),
                    post_check.failed_requirements.len()
                ),
                Some(&binding.binding_id),
                None,
                None,
            );
        }
        post_check
    }

    pub(super) async fn get_or_create_install_lock(&self, env_id: &str) -> Arc<Mutex<()>> {
        {
            let map = self.install_locks.read().await;
            if let Some(lock) = map.get(env_id) {
                return lock.clone();
            }
        }
        let mut map = self.install_locks.write().await;
        map.entry(env_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}
