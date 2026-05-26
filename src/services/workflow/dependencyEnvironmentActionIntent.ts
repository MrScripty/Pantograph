export type DependencyEnvironmentAction = 'resolve' | 'check' | 'install';

export type DependencyEnvironmentActionIntentStatus = 'request_ready' | 'blocked';

export interface InferenceInterfaceDiagnostic {
  code: string;
  severity: string;
  message: string;
  node_id?: string;
  port_id?: string;
  field_path?: string;
}

export interface DependencyEnvironmentActionIntent {
  contract_version: number;
  graph_session_id: string;
  graph_revision: string;
  validation_session_id?: string | null;
  target_node_id: string;
  action: DependencyEnvironmentAction;
}

export interface DependencyEnvironmentActionIntentResult {
  contract_version: number;
  graph_session_id: string;
  graph_revision: string;
  validation_session_id?: string | null;
  target_node_id: string;
  action: DependencyEnvironmentAction;
  status: DependencyEnvironmentActionIntentStatus;
  diagnostics?: InferenceInterfaceDiagnostic[];
}
