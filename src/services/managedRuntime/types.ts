export type ManagedRuntimeId = 'llama_cpp';

export type ManagedBinaryCategory =
  | 'runtime_sidecar'
  | 'media_tool'
  | 'native_artifact';

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

export type ManagedBinaryActionSupport =
  | 'resolved_command'
  | 'activation'
  | 'status_only';

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
  active_version: string | null;
  default_version: string | null;
}

export interface ManagedRuntimeVersionStatus {
  version: string | null;
  display_label: string;
  runtime_key: string;
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

export interface ManagedBinarySource {
  owner: string | null;
  project: string | null;
}

export interface ManagedBinaryVersionStatus {
  version: string | null;
  display_label: string;
  platform_key: string;
  install_root: string | null;
  expected_files: string[];
  missing_files: string[];
  install_state: ManagedBinaryInstallState;
  readiness_state: ManagedRuntimeReadinessState;
  selected: boolean;
  active: boolean;
  source: ManagedBinarySource | null;
  checksum_sha256: string | null;
}

export interface ManagedRuntimeJobStatus {
  state: ManagedRuntimeJobState;
  status: string;
  current: number;
  total: number;
  resumable: boolean;
  cancellable: boolean;
  error: string | null;
}

export interface ManagedRuntimeJobArtifactStatus {
  version: string;
  archive_name: string;
  downloaded_bytes: number;
  total_bytes: number;
  retained: boolean;
}

export interface ManagedRuntimeInstallHistoryEntry {
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

export interface ManagedBinaryStatus {
  key: string;
  display_name: string;
  category: ManagedBinaryCategory;
  install_state: ManagedBinaryInstallState;
  readiness_state: ManagedRuntimeReadinessState;
  available: boolean;
  can_install: boolean;
  can_remove: boolean;
  missing_files: string[];
  unavailable_reason: string | null;
  active_job: ManagedRuntimeJobStatus | null;
  selected_version: string | null;
  active_version: string | null;
  default_version: string | null;
  action_support: ManagedBinaryActionSupport;
  versions: ManagedBinaryVersionStatus[];
}

export interface ManagedRuntimeProgress {
  runtime_id: ManagedRuntimeId;
  status: string;
  current: number;
  total: number;
  done: boolean;
  error: string | null;
  runtime: ManagedRuntimeManagerRuntimeView;
}
