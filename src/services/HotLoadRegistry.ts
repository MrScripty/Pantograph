import type { DynamicComponent } from '../types';
import { Logger } from './Logger';
import { RuntimeCompiler } from './RuntimeCompiler';

export interface Position {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

export interface ComponentUpdate {
  id: string;
  path: string;
  position: Position;
  size: Size;
  source: string;
}

export interface GeneratedComponent extends DynamicComponent {
  source: string;
  path: string;
  position: Position;
  size: Size;
}

class Registry {
  private components: GeneratedComponent[] = [];
  private listeners: Array<(comps: GeneratedComponent[]) => void> = [];

  /**
   * Register a pre-built component
   */
  public register(comp: DynamicComponent) {
    const generated: GeneratedComponent = {
      ...comp,
      source: '',
      path: '',
      position: { x: 0, y: 0 },
      size: { width: 200, height: 100 },
    };
    this.components.push(generated);
    Logger.log('COMPONENT_REGISTERED', { id: comp.id });
    this.notify();
  }

  /**
   * Register a component from source code with position
   */
  public async registerFromSource(
    id: string,
    source: string,
    path: string,
    position: Position,
    size: Size
  ): Promise<void> {
    // Try to import the component dynamically
    const compiled = await RuntimeCompiler.importComponent(path);

    if (compiled.error || !compiled.component) {
      Logger.log('COMPONENT_COMPILE_ERROR', { id, path, error: compiled.error }, 'error');
      // Still register with null component - UI can show placeholder
    }

    const existing = this.components.findIndex((c) => c.id === id);
    const component: GeneratedComponent = {
      id,
      component: compiled.component!,
      source,
      path,
      position,
      size,
      props: {
        style: `position: absolute; left: ${position.x}px; top: ${position.y}px; width: ${size.width}px; height: ${size.height}px;`,
      },
    };

    if (existing >= 0) {
      this.components[existing] = component;
      Logger.log('COMPONENT_UPDATED', { id, path, position });
    } else {
      this.components.push(component);
      Logger.log('COMPONENT_REGISTERED_FROM_SOURCE', { id, path, position });
    }

    this.notify();
  }

  /**
   * Register component from a ComponentUpdate object
   */
  public async registerFromUpdate(update: ComponentUpdate): Promise<void> {
    await this.registerFromSource(
      update.id,
      update.source,
      update.path,
      update.position,
      update.size
    );
  }

  /**
   * Update a component's position
   */
  public updatePosition(id: string, x: number, y: number): void {
    const comp = this.components.find((c) => c.id === id);
    if (comp) {
      comp.position = { x, y };
      comp.props = {
        ...comp.props,
        style: `position: absolute; left: ${x}px; top: ${y}px; width: ${comp.size.width}px; height: ${comp.size.height}px;`,
      };
      Logger.log('COMPONENT_POSITION_UPDATED', { id, x, y });
      this.notify();
    }
  }

  /**
   * Update a component's size
   */
  public updateSize(id: string, width: number, height: number): void {
    const comp = this.components.find((c) => c.id === id);
    if (comp) {
      comp.size = { width, height };
      comp.props = {
        ...comp.props,
        style: `position: absolute; left: ${comp.position.x}px; top: ${comp.position.y}px; width: ${width}px; height: ${height}px;`,
      };
      Logger.log('COMPONENT_SIZE_UPDATED', { id, width, height });
      this.notify();
    }
  }

  public unregister(id: string) {
    this.components = this.components.filter((c) => c.id !== id);
    this.notify();
  }

  private notify() {
    this.listeners.forEach((listener) => listener([...this.components]));
  }

  public subscribe(listener: (comps: GeneratedComponent[]) => void) {
    this.listeners.push(listener);
    listener([...this.components]);
    return () => {
      this.listeners = this.listeners.filter((cb) => cb !== listener);
    };
  }

  public getAll(): GeneratedComponent[] {
    return [...this.components];
  }

  public getById(id: string): GeneratedComponent | undefined {
    return this.components.find((c) => c.id === id);
  }
}

export const componentRegistry = new Registry();
