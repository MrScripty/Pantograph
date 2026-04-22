export type DependencyState =
  | 'unresolved'
  | 'invalid'
  | 'resolved'
  | 'checking'
  | 'missing'
  | 'installing'
  | 'ready'
  | 'failed'
  | string;

export type DependencyValidationState =
  | 'resolved'
  | 'unknown_profile'
  | 'invalid_profile'
  | 'profile_conflict';

export interface DependencyValidationError {
  code: string;
  scope: 'top_level' | 'binding';
  binding_id?: string;
  field?: string;
  message: string;
}

export interface ModelDependencyRequirement {
  kind: string;
  name: string;
  exact_pin: string;
}

export interface ModelDependencyBinding {
  binding_id: string;
  profile_id: string;
  profile_version: number;
  profile_hash?: string;
  backend_key?: string;
  platform_selector?: string;
  environment_kind?: string;
  env_id?: string;
  validation_state: DependencyValidationState;
  validation_errors: DependencyValidationError[];
  requirements: ModelDependencyRequirement[];
}

export interface ModelDependencyRequirements {
  model_id: string;
  platform_key: string;
  backend_key?: string;
  dependency_contract_version: number;
  validation_state: DependencyValidationState;
  validation_errors: DependencyValidationError[];
  bindings: ModelDependencyBinding[];
  selected_binding_ids: string[];
}

export interface ModelDependencyBindingStatus {
  binding_id: string;
  env_id?: string;
  state: DependencyState;
  code?: string;
  message?: string;
  missing_requirements?: string[];
  installed_requirements?: string[];
  failed_requirements?: string[];
}

export interface ModelDependencyStatus {
  state: DependencyState;
  code?: string;
  message?: string;
  requirements: ModelDependencyRequirements;
  bindings: ModelDependencyBindingStatus[];
  checked_at?: string;
}

export interface EnvironmentRef {
  contract_version: number;
  environment_key?: string;
  environment_kind?: string;
  env_id?: string;
  env_ids?: string[];
  python_executable?: string;
  state: string;
  requirements_fingerprint?: string;
  platform_key?: string;
  backend_key?: string;
  manifest_path?: string;
}

export interface DependencyOverrideFieldsV1 {
  python_executable?: string;
  index_url?: string;
  extra_index_urls?: string[];
  wheel_source_path?: string;
  package_source_override?: string;
}

export interface DependencyOverridePatchV1 {
  contract_version: number;
  binding_id: string;
  scope: 'binding' | 'requirement';
  requirement_name?: string;
  fields: DependencyOverrideFieldsV1;
  source?: string;
  updated_at?: string;
}

export interface DependencyActivityEvent {
  timestamp: string;
  node_type: string;
  model_path: string;
  phase: string;
  message: string;
  binding_id?: string;
  requirement_name?: string;
  stream?: string;
}

export type StringOverrideField =
  | 'python_executable'
  | 'index_url'
  | 'wheel_source_path'
  | 'package_source_override';

export interface DependencyEnvironmentActionRequest {
  action: 'resolve' | 'check' | 'install' | 'run';
  mode?: 'auto' | 'manual';
  modelPath: string;
  modelId?: string;
  modelType?: string;
  taskTypePrimary?: string;
  backendKey?: string;
  platformContext?: Record<string, string>;
  selectedBindingIds?: string[];
  dependencyRequirements?: ModelDependencyRequirements;
  dependencyOverridePatches?: DependencyOverridePatchV1[];
}

export interface DependencyEnvironmentActionResponse {
  nodeData: Record<string, unknown>;
}

export interface DependencyEnvironmentActionPayloadInput {
  action: DependencyEnvironmentActionRequest['action'];
  mode: 'auto' | 'manual';
  upstreamModelPath: string | null;
  upstreamModelId: string | null;
  upstreamModelType: string | null;
  upstreamTaskType: string | null;
  upstreamBackendKey: string | null;
  upstreamPlatformContext: Record<string, string> | null;
  selectedBindingIds: string[];
  upstreamRequirements: ModelDependencyRequirements | null;
  dependencyRequirements: ModelDependencyRequirements | null;
  effectiveManualOverrides: DependencyOverridePatchV1[];
}

export interface DependencyBadge {
  label: string;
  className: string;
}
