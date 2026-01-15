import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Logger } from './Logger';

export interface ModelConfig {
  vlm_model_path: string | null;
  vlm_mmproj_path: string | null;
  embedding_model_path: string | null;
  candle_embedding_model_path: string | null;
}

export interface DeviceConfig {
  device: string;
  gpu_layers: number;
}

export interface DeviceInfo {
  id: string;
  name: string;
  total_vram_mb: number;
  free_vram_mb: number;
}

export interface ConnectionMode {
  type: 'None' | 'External' | 'Sidecar';
  url?: string;
}

export interface AppConfig {
  models: ModelConfig;
  device: DeviceConfig;
  connection_mode: ConnectionMode;
  external_url: string | null;
}

export interface ServerModeInfo {
  mode: string;
  ready: boolean;
  url: string | null;
  model_path: string | null;
  is_embedding_mode: boolean;
}

export interface ConfigState {
  config: AppConfig;
  serverMode: ServerModeInfo;
  isLoading: boolean;
  error: string | null;
}

const defaultConfig: AppConfig = {
  models: {
    vlm_model_path: null,
    vlm_mmproj_path: null,
    embedding_model_path: null,
    candle_embedding_model_path: null,
  },
  device: {
    device: 'auto',
    gpu_layers: -1,
  },
  connection_mode: { type: 'None' },
  external_url: null,
};

const defaultServerMode: ServerModeInfo = {
  mode: 'none',
  ready: false,
  url: null,
  model_path: null,
  is_embedding_mode: false,
};

class ConfigServiceClass {
  private state: ConfigState = {
    config: { ...defaultConfig },
    serverMode: { ...defaultServerMode },
    isLoading: false,
    error: null,
  };

  private listeners: Array<(state: ConfigState) => void> = [];

  public subscribe(callback: (state: ConfigState) => void): () => void {
    this.listeners.push(callback);
    callback({ ...this.state });
    return () => {
      this.listeners = this.listeners.filter((l) => l !== callback);
    };
  }

  private notify(): void {
    const stateCopy = { ...this.state };
    this.listeners.forEach((l) => l(stateCopy));
  }

  public getState(): ConfigState {
    return { ...this.state };
  }

  /**
   * Load configuration from backend
   */
  public async loadConfig(): Promise<void> {
    this.state.isLoading = true;
    this.state.error = null;
    this.notify();

    try {
      const config = await invoke<AppConfig>('get_app_config');
      this.state.config = config;
      Logger.log('CONFIG_LOADED', { config });
    } catch (error) {
      this.state.error = String(error);
      Logger.log('CONFIG_LOAD_ERROR', { error: String(error) }, 'error');
    } finally {
      this.state.isLoading = false;
      this.notify();
    }
  }

  /**
   * Save configuration to backend
   */
  public async saveConfig(config: AppConfig): Promise<void> {
    try {
      await invoke('set_app_config', { newConfig: config });
      this.state.config = config;
      this.state.error = null;
      Logger.log('CONFIG_SAVED', { config });
      this.notify();
    } catch (error) {
      this.state.error = String(error);
      Logger.log('CONFIG_SAVE_ERROR', { error: String(error) }, 'error');
      this.notify();
      throw error;
    }
  }

  /**
   * Update model configuration
   */
  public async setModelConfig(models: ModelConfig): Promise<void> {
    try {
      await invoke('set_model_config', { models });
      this.state.config.models = models;
      this.state.error = null;
      Logger.log('MODEL_CONFIG_SAVED', { models });
      this.notify();
    } catch (error) {
      this.state.error = String(error);
      Logger.log('MODEL_CONFIG_SAVE_ERROR', { error: String(error) }, 'error');
      this.notify();
      throw error;
    }
  }

  /**
   * Get device configuration
   */
  public async getDeviceConfig(): Promise<DeviceConfig> {
    try {
      const device = await invoke<DeviceConfig>('get_device_config');
      this.state.config.device = device;
      this.notify();
      return device;
    } catch (error) {
      Logger.log('DEVICE_CONFIG_ERROR', { error: String(error) }, 'error');
      throw error;
    }
  }

  /**
   * Update device configuration
   */
  public async setDeviceConfig(device: DeviceConfig): Promise<void> {
    try {
      await invoke('set_device_config', { device });
      this.state.config.device = device;
      this.state.error = null;
      Logger.log('DEVICE_CONFIG_SAVED', { device });
      this.notify();
    } catch (error) {
      this.state.error = String(error);
      Logger.log('DEVICE_CONFIG_SAVE_ERROR', { error: String(error) }, 'error');
      this.notify();
      throw error;
    }
  }

  /**
   * List available compute devices
   */
  public async listDevices(): Promise<DeviceInfo[]> {
    try {
      const devices = await invoke<DeviceInfo[]>('list_devices');
      Logger.log('DEVICES_LISTED', { count: devices.length });
      return devices;
    } catch (error) {
      Logger.log('LIST_DEVICES_ERROR', { error: String(error) }, 'error');
      throw error;
    }
  }

  /**
   * Get server mode info
   */
  public async refreshServerMode(): Promise<ServerModeInfo> {
    try {
      const mode = await invoke<ServerModeInfo>('get_server_mode');
      this.state.serverMode = mode;
      this.notify();
      return mode;
    } catch (error) {
      Logger.log('SERVER_MODE_ERROR', { error: String(error) }, 'error');
      throw error;
    }
  }

  /**
   * Start sidecar in inference mode (VLM)
   */
  public async startInferenceMode(): Promise<ServerModeInfo> {
    try {
      const mode = await invoke<ServerModeInfo>('start_sidecar_inference');
      this.state.serverMode = mode;
      this.state.error = null;
      Logger.log('SIDECAR_INFERENCE_STARTED', { mode });
      this.notify();
      return mode;
    } catch (error) {
      this.state.error = String(error);
      Logger.log('SIDECAR_INFERENCE_ERROR', { error: String(error) }, 'error');
      this.notify();
      throw error;
    }
  }

  /**
   * Start sidecar in embedding mode
   */
  public async startEmbeddingMode(): Promise<ServerModeInfo> {
    try {
      const mode = await invoke<ServerModeInfo>('start_sidecar_embedding');
      this.state.serverMode = mode;
      this.state.error = null;
      Logger.log('SIDECAR_EMBEDDING_STARTED', { mode });
      this.notify();
      return mode;
    } catch (error) {
      this.state.error = String(error);
      Logger.log('SIDECAR_EMBEDDING_ERROR', { error: String(error) }, 'error');
      this.notify();
      throw error;
    }
  }

  /**
   * Open file picker for model selection
   */
  public async pickModelFile(title: string): Promise<string | null> {
    try {
      const result = await open({
        title,
        filters: [
          {
            name: 'GGUF Models',
            extensions: ['gguf'],
          },
          {
            name: 'All Files',
            extensions: ['*'],
          },
        ],
        multiple: false,
        directory: false,
      });

      if (result && typeof result === 'string') {
        return result;
      }
      return null;
    } catch (error) {
      Logger.log('FILE_PICKER_ERROR', { error: String(error) }, 'error');
      return null;
    }
  }

  /**
   * Open directory picker for SafeTensors model selection
   */
  public async pickDirectory(title: string): Promise<string | null> {
    try {
      const result = await open({
        title,
        multiple: false,
        directory: true,
      });

      if (result && typeof result === 'string') {
        return result;
      }
      return null;
    } catch (error) {
      Logger.log('DIR_PICKER_ERROR', { error: String(error) }, 'error');
      return null;
    }
  }

  /**
   * Check if VLM models are configured
   */
  public get hasVlmModels(): boolean {
    return !!(
      this.state.config.models.vlm_model_path &&
      this.state.config.models.vlm_mmproj_path
    );
  }

  /**
   * Check if embedding model is configured
   */
  public get hasEmbeddingModel(): boolean {
    return !!this.state.config.models.embedding_model_path;
  }

  /**
   * Check if server is ready
   */
  public get isServerReady(): boolean {
    return this.state.serverMode.ready;
  }

  /**
   * Check if in embedding mode
   */
  public get isEmbeddingMode(): boolean {
    return this.state.serverMode.is_embedding_mode;
  }
}

export const ConfigService = new ConfigServiceClass();
