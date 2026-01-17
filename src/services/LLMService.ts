import { invoke, Channel } from '@tauri-apps/api/core';
import { Logger } from './Logger';

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  imageBase64?: string;
  timestamp: number;
}

export interface StreamEvent {
  content: string | null;
  done: boolean;
  error: string | null;
}

export interface LLMStatus {
  ready: boolean;
  mode: 'none' | 'external' | 'sidecar';
  url: string | null;
}

export interface LLMState {
  status: LLMStatus;
  isGenerating: boolean;
  messages: ChatMessage[];
  currentResponse: string;
  error: string | null;
}

class LLMServiceClass {
  private state: LLMState = {
    status: { ready: false, mode: 'none', url: null },
    isGenerating: false,
    messages: [],
    currentResponse: '',
    error: null,
  };

  private listeners: Array<(state: LLMState) => void> = [];

  public subscribe(callback: (state: LLMState) => void): () => void {
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

  public async connectToServer(url: string): Promise<void> {
    try {
      this.state.error = null;
      this.notify();

      const status = await invoke<LLMStatus>('connect_to_server', { url });
      this.state.status = status;
      Logger.log('LLM_CONNECTED_EXTERNAL', { url });
      this.notify();
    } catch (error) {
      this.state.error = String(error);
      Logger.log('LLM_CONNECT_ERROR', { error: String(error) }, 'error');
      this.notify();
      throw error;
    }
  }

  public async startSidecar(modelPath: string, mmprojPath: string): Promise<void> {
    try {
      this.state.error = null;
      this.notify();

      const status = await invoke<LLMStatus>('start_sidecar_llm', {
        modelPath,
        mmprojPath,
      });
      this.state.status = status;
      Logger.log('LLM_SIDECAR_STARTED', { modelPath, mmprojPath });
      this.notify();
    } catch (error) {
      this.state.error = String(error);
      Logger.log('LLM_SIDECAR_ERROR', { error: String(error) }, 'error');
      this.notify();
      throw error;
    }
  }

  public async refreshStatus(): Promise<LLMStatus> {
    try {
      const status = await invoke<LLMStatus>('get_llm_status');
      this.state.status = status;
      this.notify();
      return status;
    } catch (error) {
      Logger.log('llm_status_refresh_failed', { error: String(error) }, 'warn');
      return this.state.status;
    }
  }

  public get isReady(): boolean {
    return this.state.status.ready;
  }

  public async sendVisionPrompt(prompt: string, imageBase64: string): Promise<void> {
    if (!this.state.status.ready) {
      throw new Error('LLM not ready');
    }

    if (this.state.isGenerating) {
      throw new Error('Already generating');
    }

    const userMessage: ChatMessage = {
      role: 'user',
      content: prompt,
      imageBase64,
      timestamp: Date.now(),
    };
    this.state.messages.push(userMessage);
    this.state.isGenerating = true;
    this.state.currentResponse = '';
    this.state.error = null;
    this.notify();

    Logger.log('LLM_PROMPT_SENT', { promptLength: prompt.length });

    try {
      const channel = new Channel<StreamEvent>();

      channel.onmessage = (event: StreamEvent) => {
        if (event.error) {
          Logger.log('LLM_STREAM_ERROR', { error: event.error }, 'error');
          this.state.error = event.error;
          this.state.isGenerating = false;
          this.notify();
          return;
        }

        if (event.content) {
          this.state.currentResponse += event.content;
          this.notify();
        }

        if (event.done) {
          const assistantMessage: ChatMessage = {
            role: 'assistant',
            content: this.state.currentResponse,
            timestamp: Date.now(),
          };
          this.state.messages.push(assistantMessage);
          this.state.isGenerating = false;
          Logger.log('LLM_RESPONSE_COMPLETE', {
            responseLength: assistantMessage.content.length,
          });
          this.state.currentResponse = '';
          this.notify();
        }
      };

      await invoke('send_vision_prompt', {
        prompt,
        imageBase64,
        channel,
      });
    } catch (error) {
      this.state.isGenerating = false;
      this.state.error = String(error);
      Logger.log('LLM_ERROR', { error: String(error) }, 'error');
      this.notify();
      throw error;
    }
  }

  public getState(): LLMState {
    return { ...this.state };
  }

  public clearHistory(): void {
    this.state.messages = [];
    this.state.currentResponse = '';
    this.state.error = null;
    Logger.log('LLM_HISTORY_CLEARED', {});
    this.notify();
  }

  public async stop(): Promise<void> {
    try {
      await invoke('stop_llm');
      this.state.status = { ready: false, mode: 'none', url: null };
      this.state.isGenerating = false;
      Logger.log('LLM_STOPPED', {});
      this.notify();
    } catch (error) {
      Logger.log('LLM_STOP_ERROR', { error: String(error) }, 'error');
    }
  }
}

export const LLMService = new LLMServiceClass();
