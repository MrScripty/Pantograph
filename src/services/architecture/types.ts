// Architecture visualization types

export type ArchNodeCategory =
  | 'component'    // Svelte components
  | 'service'      // TypeScript services
  | 'store'        // Svelte stores
  | 'backend'      // Rust modules
  | 'command';     // Tauri commands

export type ArchConnectionType =
  | 'import'       // ES6 import
  | 'command'      // Tauri invoke() call
  | 'subscription' // Store subscription ($store)
  | 'event'        // Event emission/listening
  | 'uses';        // General usage relationship

export interface ArchNodeDefinition {
  id: string;
  category: ArchNodeCategory;
  label: string;
  description?: string;
  filePath?: string;
}

export interface ArchConnection {
  id: string;
  source: string;
  target: string;
  connectionType: ArchConnectionType;
  label?: string;
}

export interface ArchitectureGraph {
  nodes: ArchNodeDefinition[];
  connections: ArchConnection[];
  metadata?: {
    generatedAt?: string;
    version?: string;
  };
}

// Color mapping for node categories
export const CATEGORY_COLORS: Record<ArchNodeCategory, string> = {
  component: '#2563eb',  // Blue
  service: '#16a34a',    // Green
  store: '#9333ea',      // Purple
  backend: '#d97706',    // Amber
  command: '#0891b2',    // Cyan
};

// Edge styling for connection types
export const CONNECTION_STYLES: Record<ArchConnectionType, { stroke: string; strokeDasharray: string }> = {
  import: { stroke: '#6366f1', strokeDasharray: 'none' },
  command: { stroke: '#06b6d4', strokeDasharray: '5,5' },
  subscription: { stroke: '#8b5cf6', strokeDasharray: 'none' },
  event: { stroke: '#f59e0b', strokeDasharray: '3,3' },
  uses: { stroke: '#6b7280', strokeDasharray: 'none' },
};
