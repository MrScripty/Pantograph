import type { DynamicComponent } from '../types';
import { Logger } from './Logger';

class Registry {
  private components: DynamicComponent[] = [];
  private listeners: Array<(comps: DynamicComponent[]) => void> = [];

  public register(comp: DynamicComponent) {
    this.components.push(comp);
    Logger.log('COMPONENT_REGISTERED', { id: comp.id });
    this.notify();
  }

  public unregister(id: string) {
    this.components = this.components.filter((c) => c.id !== id);
    this.notify();
  }

  private notify() {
    this.listeners.forEach((listener) => listener([...this.components]));
  }

  public subscribe(listener: (comps: DynamicComponent[]) => void) {
    this.listeners.push(listener);
    listener([...this.components]);
    return () => {
      this.listeners = this.listeners.filter((cb) => cb !== listener);
    };
  }
}

export const componentRegistry = new Registry();
