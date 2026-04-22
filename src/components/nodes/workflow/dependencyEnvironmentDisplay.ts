import type {
  DependencyActivityEvent,
  DependencyBadge,
  ModelDependencyRequirements,
  ModelDependencyStatus,
} from './dependencyEnvironmentTypes.ts';

export function dependencyTokenLabel(value: string): string {
  return value.replaceAll('_', ' ');
}

export function dependencyCodeLabel(code?: string): string | null {
  switch (code) {
    case 'requirements_missing':
      return 'requirements missing';
    case 'dependency_install_failed':
    case 'dependency_check_failed':
      return 'dependency check failed';
    case 'profile_conflict':
      return 'profile conflict';
    case 'unknown_profile':
      return 'unknown profile';
    case 'invalid_profile':
      return 'invalid profile';
    default:
      return code ? dependencyTokenLabel(code) : null;
  }
}

export function deriveDependencyDisplayState(
  dependencyRequirements: ModelDependencyRequirements | null,
  dependencyStatus: ModelDependencyStatus | null
): string | null {
  if (dependencyStatus) return dependencyStatus.state;
  if (!dependencyRequirements) return null;
  switch (dependencyRequirements.validation_state) {
    case 'resolved':
      return 'resolved';
    case 'unknown_profile':
      return 'unresolved';
    default:
      return 'invalid';
  }
}

export function dependencyBadgeFor(
  dependencyRequirements: ModelDependencyRequirements | null,
  dependencyStatus: ModelDependencyStatus | null
): DependencyBadge {
  const state = deriveDependencyDisplayState(dependencyRequirements, dependencyStatus);
  if (!state) return { label: 'requirements unknown', className: 'text-neutral-400 border-neutral-700' };
  switch (state) {
    case 'ready':
      return { label: 'deps ready', className: 'text-emerald-400 border-emerald-500/40' };
    case 'missing':
      return { label: 'deps missing', className: 'text-amber-400 border-amber-500/40' };
    case 'resolved':
      return { label: 'requirements resolved', className: 'text-cyan-300 border-cyan-500/40' };
    case 'checking':
      return { label: 'deps checking', className: 'text-cyan-400 border-cyan-500/40' };
    case 'installing':
      return { label: 'deps installing', className: 'text-sky-400 border-sky-500/40' };
    case 'unresolved':
      return { label: 'requirements unresolved', className: 'text-violet-400 border-violet-500/40' };
    case 'invalid':
      return { label: 'requirements invalid', className: 'text-orange-400 border-orange-500/40' };
    case 'failed':
      return { label: 'deps failed', className: 'text-red-400 border-red-500/40' };
    default:
      return {
        label: `deps ${dependencyTokenLabel(state)}`,
        className: 'text-neutral-300 border-neutral-600/50',
      };
  }
}

export function formatDependencyActivityLine(line: string, timestamp: string): string | null {
  const normalized = line.trim();
  if (normalized.length === 0) return null;
  return `[${timestamp}] ${normalized}`;
}

export function formatDependencyActivityTimestamp(date: Date): string {
  return date.toLocaleTimeString('en-US', { hour12: false });
}

export function matchesDependencyActivityEvent(
  payload: DependencyActivityEvent,
  upstreamModelPath: string | null
): boolean {
  const upstreamPath = (upstreamModelPath ?? '').trim();
  if (upstreamPath.length === 0) return false;
  const eventPath = (payload.model_path ?? '').trim();
  if (eventPath.length === 0 || eventPath !== upstreamPath) return false;
  return (payload.node_type ?? '').trim() === 'dependency-environment';
}

export function renderDependencyActivityEvent(payload: DependencyActivityEvent): string {
  const parts = [payload.phase];
  if (payload.binding_id) parts.push(payload.binding_id);
  if (payload.requirement_name) parts.push(payload.requirement_name);
  if (payload.stream) parts.push(payload.stream);
  return `${parts.join(' | ')}: ${payload.message}`;
}
