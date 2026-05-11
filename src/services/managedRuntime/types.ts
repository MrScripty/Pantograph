export type ManagedRuntimeId = 'llama_cpp';

export type ManagedDependencyCategory =
  | 'runtime_sidecar'
  | 'media_tool'
  | 'native_artifact';

export type ManagedMediaToolDependencyId = 'ffmpeg' | 'ocioconvert' | 'oiiotool';

export type ManagedNativeArtifactDependencyId = 'open_color_io';

export type ManagedDependencyKey =
  | { runtime_sidecar: ManagedRuntimeId }
  | { media_tool: ManagedMediaToolDependencyId }
  | { native_artifact: ManagedNativeArtifactDependencyId };

export type ManagedBinaryInstallState =
  | 'installed'
  | 'system_provided'
  | 'missing'
  | 'unsupported';

export type ManagedRuntimeReadinessState =
  | 'unknown'
  | 'missing'
  | 'downloading'
  | 'extracting'
  | 'validating'
  | 'ready'
  | 'failed'
  | 'unsupported';

export type ManagedRuntimeJobState =
  | 'queued'
  | 'downloading'
  | 'paused'
  | 'extracting'
  | 'validating'
  | 'ready'
  | 'failed'
  | 'cancelled';

export type ManagedRuntimeHistoryEventKind =
  | 'installed'
  | 'removed'
  | 'selection_updated'
  | 'recovery_reconciled'
  | 'validation_failed'
  | 'paused'
  | 'cancelled';

export interface ManagedRuntimeSelectionState {
  selected_version: string | null;
  selected_runtime_variant_id: string | null;
  active_version: string | null;
  active_runtime_variant_id: string | null;
  default_version: string | null;
  default_runtime_variant_id: string | null;
}

export interface ManagedRuntimeVersionStatus {
  version: string | null;
  display_label: string;
  runtime_key: string;
  runtime_variant_id: string;
  platform_key: string;
  install_root: string | null;
  executable_name: string;
  executable_ready: boolean;
  install_state: ManagedBinaryInstallState;
  readiness_state: ManagedRuntimeReadinessState;
  catalog_available: boolean;
  installable: boolean;
  selected: boolean;
  active: boolean;
}

export interface ManagedDependencyVersionStatus {
  version: string | null;
  platform_key: string;
  install_root: string | null;
  expected_files: string[];
  missing_files: string[];
  install_state: ManagedBinaryInstallState;
  readiness_state: ManagedRuntimeReadinessState;
  selected: boolean;
  active: boolean;
}

export interface ManagedRuntimeJobStatus {
  runtime_variant_id: string;
  state: ManagedRuntimeJobState;
  status: string;
  current: number;
  total: number;
  resumable: boolean;
  cancellable: boolean;
  error: string | null;
}

export interface ManagedRuntimeJobArtifactStatus {
  runtime_variant_id: string;
  version: string;
  archive_name: string;
  downloaded_bytes: number;
  total_bytes: number;
  retained: boolean;
}

export interface ManagedRuntimeInstallHistoryEntry {
  runtime_variant_id: string;
  event: ManagedRuntimeHistoryEventKind;
  version: string | null;
  at_ms: number;
  detail: string | null;
}

export interface ManagedRuntimeManagerRuntimeView {
  id: ManagedRuntimeId;
  display_name: string;
  install_state: ManagedBinaryInstallState;
  readiness_state: ManagedRuntimeReadinessState;
  available: boolean;
  can_install: boolean;
  can_remove: boolean;
  missing_files: string[];
  unavailable_reason: string | null;
  versions: ManagedRuntimeVersionStatus[];
  selection: ManagedRuntimeSelectionState;
  active_job: ManagedRuntimeJobStatus | null;
  job_artifact: ManagedRuntimeJobArtifactStatus | null;
  install_history: ManagedRuntimeInstallHistoryEntry[];
}

export interface ManagedDependencySelectionState {
  selected_version: string | null;
  active_version: string | null;
  default_version: string | null;
}

export interface ManagedDependencyStatus {
  key: ManagedDependencyKey;
  display_name: string;
  category: ManagedDependencyCategory;
  install_state: ManagedBinaryInstallState;
  readiness_state: ManagedRuntimeReadinessState;
  available: boolean;
  missing_files: string[];
  unavailable_reason: string | null;
  selection: ManagedDependencySelectionState;
  versions: ManagedDependencyVersionStatus[];
}

export interface ManagedRuntimeProgress {
  runtime_id: ManagedRuntimeId;
  runtime_variant_id: string;
  status: string;
  current: number;
  total: number;
  done: boolean;
  error: string | null;
  runtime: ManagedRuntimeManagerRuntimeView;
}
