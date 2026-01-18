import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { Logger } from './Logger';

// Types matching Rust backend
export interface HealthCheckResult {
  healthy: boolean;
  status: HealthStatus;
  response_time_ms: number | null;
  error: string | null;
  timestamp: string;
  consecutive_failures: number;
}

export type HealthStatus =
  | { type: 'healthy' }
  | { type: 'degraded'; reason: string }
  | { type: 'unhealthy'; reason: string }
  | { type: 'unknown' };

export type ServerEvent =
  | { type: 'health_update'; result: HealthCheckResult }
  | { type: 'server_crashed'; error: string }
  | { type: 'recovery_started' }
  | { type: 'recovery_complete'; success: boolean; error: string | null };

export interface PortStatus {
  port: number;
  available: boolean;
  blocking_process: ProcessInfo | null;
  is_pantograph: boolean;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  command: string | null;
}

export interface RecoveryConfig {
  auto_recovery_enabled: boolean;
  max_attempts: number;
  backoff_base_ms: number;
  backoff_max_ms: number;
  try_alternate_port: boolean;
}

export interface HealthMonitorState {
  isRunning: boolean;
  lastResult: HealthCheckResult | null;
  isRecovering: boolean;
  recoveryAttempts: number;
  error: string | null;
}

type HealthMonitorListener = (state: HealthMonitorState) => void;
type ServerEventListener = (event: ServerEvent) => void;

class HealthMonitorServiceClass {
  private state: HealthMonitorState = {
    isRunning: false,
    lastResult: null,
    isRecovering: false,
    recoveryAttempts: 0,
    error: null,
  };

  private stateListeners: HealthMonitorListener[] = [];
  private eventListeners: ServerEventListener[] = [];
  private eventUnlisten: UnlistenFn | null = null;

  /**
   * Subscribe to state changes
   */
  public subscribeState(callback: HealthMonitorListener): () => void {
    this.stateListeners.push(callback);
    callback({ ...this.state });
    return () => {
      this.stateListeners = this.stateListeners.filter((l) => l !== callback);
    };
  }

  /**
   * Subscribe to server events (health updates, crashes, recovery)
   */
  public subscribeEvents(callback: ServerEventListener): () => void {
    this.eventListeners.push(callback);
    return () => {
      this.eventListeners = this.eventListeners.filter((l) => l !== callback);
    };
  }

  private notifyState() {
    const stateCopy = { ...this.state };
    this.stateListeners.forEach((l) => l(stateCopy));
  }

  private notifyEvent(event: ServerEvent) {
    this.eventListeners.forEach((l) => l(event));
  }

  /**
   * Start health monitoring
   */
  public async start(): Promise<void> {
    if (this.state.isRunning) {
      Logger.log('health_monitor_already_running', {}, 'warn');
      return;
    }

    try {
      // Set up event listener
      this.eventUnlisten = await listen<ServerEvent>('server-health', (event) => {
        this.handleServerEvent(event.payload);
      });

      // Start the backend monitor
      await invoke('start_health_monitor');

      this.state.isRunning = true;
      this.state.error = null;
      this.notifyState();

      Logger.log('health_monitor_started', {});
    } catch (error) {
      this.state.error = String(error);
      this.notifyState();
      Logger.log('health_monitor_start_failed', { error: String(error) }, 'error');
      throw error;
    }
  }

  /**
   * Stop health monitoring
   */
  public async stop(): Promise<void> {
    if (!this.state.isRunning) {
      return;
    }

    try {
      // Remove event listener
      if (this.eventUnlisten) {
        this.eventUnlisten();
        this.eventUnlisten = null;
      }

      // Stop the backend monitor
      await invoke('stop_health_monitor');

      this.state.isRunning = false;
      this.notifyState();

      Logger.log('health_monitor_stopped', {});
    } catch (error) {
      Logger.log('health_monitor_stop_failed', { error: String(error) }, 'error');
    }
  }

  /**
   * Handle incoming server events
   */
  private handleServerEvent(event: ServerEvent) {
    Logger.log('health_event_received', { type: event.type });

    switch (event.type) {
      case 'health_update':
        this.state.lastResult = event.result;
        this.notifyState();
        break;

      case 'server_crashed':
        this.state.error = event.error;
        this.notifyState();
        break;

      case 'recovery_started':
        this.state.isRecovering = true;
        this.notifyState();
        break;

      case 'recovery_complete':
        this.state.isRecovering = false;
        if (!event.success && event.error) {
          this.state.error = event.error;
        } else {
          this.state.error = null;
        }
        this.notifyState();
        break;
    }

    this.notifyEvent(event);
  }

  /**
   * Get the last health check result
   */
  public async getHealthStatus(): Promise<HealthCheckResult | null> {
    try {
      return await invoke<HealthCheckResult | null>('get_health_status');
    } catch (error) {
      Logger.log('get_health_status_failed', { error: String(error) }, 'error');
      return null;
    }
  }

  /**
   * Trigger an immediate health check
   */
  public async checkNow(): Promise<HealthCheckResult | null> {
    try {
      const result = await invoke<HealthCheckResult | null>('check_health_now');
      if (result) {
        this.state.lastResult = result;
        this.notifyState();
      }
      return result;
    } catch (error) {
      Logger.log('health_check_now_failed', { error: String(error) }, 'error');
      return null;
    }
  }

  /**
   * Check if a port is available
   */
  public async checkPortStatus(port?: number): Promise<PortStatus> {
    return await invoke<PortStatus>('check_port_status', { port });
  }

  /**
   * Find an available alternate port
   */
  public async findAlternatePort(start?: number): Promise<number> {
    return await invoke<number>('find_alternate_port', { start });
  }

  /**
   * Get the default server port
   */
  public async getDefaultPort(): Promise<number> {
    return await invoke<number>('get_default_port');
  }

  /**
   * Trigger manual recovery
   */
  public async triggerRecovery(): Promise<number> {
    try {
      this.state.isRecovering = true;
      this.notifyState();

      const port = await invoke<number>('trigger_recovery');

      this.state.isRecovering = false;
      this.state.error = null;
      this.notifyState();

      Logger.log('recovery_triggered', { port });
      return port;
    } catch (error) {
      this.state.isRecovering = false;
      this.state.error = String(error);
      this.notifyState();

      Logger.log('recovery_trigger_failed', { error: String(error) }, 'error');
      throw error;
    }
  }

  /**
   * Reset recovery state after manual intervention
   */
  public async resetRecoveryState(): Promise<void> {
    try {
      await invoke('reset_recovery_state');
      this.state.isRecovering = false;
      this.state.recoveryAttempts = 0;
      this.state.error = null;
      this.notifyState();
    } catch (error) {
      Logger.log('reset_recovery_state_failed', { error: String(error) }, 'error');
    }
  }

  /**
   * Get recovery configuration
   */
  public async getRecoveryConfig(): Promise<RecoveryConfig> {
    return await invoke<RecoveryConfig>('get_recovery_config');
  }

  /**
   * Check if recovery is in progress
   */
  public async isRecoveryInProgress(): Promise<boolean> {
    return await invoke<boolean>('is_recovery_in_progress');
  }

  /**
   * Get current recovery attempt count
   */
  public async getRecoveryAttemptCount(): Promise<number> {
    return await invoke<number>('get_recovery_attempt_count');
  }

  /**
   * Get current state
   */
  public getState(): HealthMonitorState {
    return { ...this.state };
  }

  /**
   * Get health status as a human-readable string
   */
  public getStatusLabel(status: HealthStatus | null): string {
    if (!status) return 'Unknown';

    switch (status.type) {
      case 'healthy':
        return 'Healthy';
      case 'degraded':
        return `Degraded: ${status.reason}`;
      case 'unhealthy':
        return `Unhealthy: ${status.reason}`;
      case 'unknown':
        return 'Unknown';
    }
  }

  /**
   * Get health status color class for UI
   */
  public getStatusColor(status: HealthStatus | null): string {
    if (!status) return 'text-neutral-500';

    switch (status.type) {
      case 'healthy':
        return 'text-green-400';
      case 'degraded':
        return 'text-yellow-400';
      case 'unhealthy':
        return 'text-red-400';
      case 'unknown':
        return 'text-neutral-500';
    }
  }
}

export const HealthMonitorService = new HealthMonitorServiceClass();
