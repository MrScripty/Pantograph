import type { ArchitectureGraph } from '../services/architecture/types';

export const PANTOGRAPH_ARCHITECTURE: ArchitectureGraph = {
  metadata: {
    generatedAt: '2026-01-17',
    version: '1.0.0'
  },
  nodes: [
    // ==================== COMPONENTS ====================
    {
      id: 'component:App',
      category: 'component',
      label: 'App',
      filePath: 'src/App.svelte',
      description: 'Root application component with view mode switching'
    },
    {
      id: 'component:Canvas',
      category: 'component',
      label: 'Canvas',
      filePath: 'src/components/Canvas.svelte',
      description: 'Drawing canvas with stroke rendering'
    },
    {
      id: 'component:WorkflowGraph',
      category: 'component',
      label: 'WorkflowGraph',
      filePath: 'src/components/WorkflowGraph.svelte',
      description: 'Visual workflow editor using SvelteFlow'
    },
    {
      id: 'component:WorkflowToolbar',
      category: 'component',
      label: 'WorkflowToolbar',
      filePath: 'src/components/WorkflowToolbar.svelte',
      description: 'Workflow actions: Run, Save, Load, Clear'
    },
    {
      id: 'component:NodePalette',
      category: 'component',
      label: 'NodePalette',
      filePath: 'src/components/NodePalette.svelte',
      description: 'Drag-and-drop node selector for workflows'
    },
    {
      id: 'component:SidePanel',
      category: 'component',
      label: 'SidePanel',
      filePath: 'src/components/SidePanel.svelte',
      description: 'Configuration panel with tabs'
    },
    {
      id: 'component:TopBar',
      category: 'component',
      label: 'TopBar',
      filePath: 'src/components/TopBar.svelte',
      description: 'Status display and metadata'
    },
    {
      id: 'component:Toolbar',
      category: 'component',
      label: 'Toolbar',
      filePath: 'src/components/Toolbar.svelte',
      description: 'Drawing tools selector'
    },
    {
      id: 'component:HotLoadContainer',
      category: 'component',
      label: 'HotLoadContainer',
      filePath: 'src/components/HotLoadContainer.svelte',
      description: 'Container for dynamically loaded components'
    },
    {
      id: 'component:ModelConfig',
      category: 'component',
      label: 'ModelConfig',
      filePath: 'src/components/ModelConfig.svelte',
      description: 'LLM model configuration UI'
    },
    {
      id: 'component:ServerStatus',
      category: 'component',
      label: 'ServerStatus',
      filePath: 'src/components/ServerStatus.svelte',
      description: 'Backend server status display'
    },
    {
      id: 'component:RagStatus',
      category: 'component',
      label: 'RagStatus',
      filePath: 'src/components/RagStatus.svelte',
      description: 'RAG indexing status display'
    },
    {
      id: 'component:ChunkPreview',
      category: 'component',
      label: 'ChunkPreview',
      filePath: 'src/components/ChunkPreview.svelte',
      description: 'Document chunk preview modal'
    },

    // ==================== SERVICES ====================
    {
      id: 'service:DrawingEngine',
      category: 'service',
      label: 'DrawingEngine',
      filePath: 'src/services/DrawingEngine.ts',
      description: 'Canvas drawing mechanics and stroke management'
    },
    {
      id: 'service:LLMService',
      category: 'service',
      label: 'LLMService',
      filePath: 'src/services/LLMService.ts',
      description: 'Vision LLM API integration'
    },
    {
      id: 'service:AgentService',
      category: 'service',
      label: 'AgentService',
      filePath: 'src/services/AgentService.ts',
      description: 'AI agent orchestration'
    },
    {
      id: 'service:RagService',
      category: 'service',
      label: 'RagService',
      filePath: 'src/services/RagService.ts',
      description: 'Retrieval-augmented generation operations'
    },
    {
      id: 'service:WorkflowService',
      category: 'service',
      label: 'WorkflowService',
      filePath: 'src/services/workflow/WorkflowService.ts',
      description: 'Workflow execution and persistence via Tauri'
    },
    {
      id: 'service:RuntimeCompiler',
      category: 'service',
      label: 'RuntimeCompiler',
      filePath: 'src/services/RuntimeCompiler.ts',
      description: 'Svelte component runtime compilation'
    },
    {
      id: 'service:HotLoadRegistry',
      category: 'service',
      label: 'HotLoadRegistry',
      filePath: 'src/services/HotLoadRegistry.ts',
      description: 'Dynamic component registration and loading'
    },
    {
      id: 'service:ConfigService',
      category: 'service',
      label: 'ConfigService',
      filePath: 'src/services/ConfigService.ts',
      description: 'Application configuration management'
    },
    {
      id: 'service:HealthMonitorService',
      category: 'service',
      label: 'HealthMonitorService',
      filePath: 'src/services/HealthMonitorService.ts',
      description: 'Backend health monitoring'
    },
    {
      id: 'service:Logger',
      category: 'service',
      label: 'Logger',
      filePath: 'src/services/Logger.ts',
      description: 'Application logging utility'
    },

    // ==================== STORES ====================
    {
      id: 'store:viewModeStore',
      category: 'store',
      label: 'viewModeStore',
      filePath: 'src/stores/viewModeStore.ts',
      description: 'Current view mode (canvas/node-graph/workflow)'
    },
    {
      id: 'store:workflowStore',
      category: 'store',
      label: 'workflowStore',
      filePath: 'src/stores/workflowStore.ts',
      description: 'Workflow nodes, edges, and execution state'
    },
    {
      id: 'store:canvasStore',
      category: 'store',
      label: 'canvasStore',
      filePath: 'src/stores/canvasStore.ts',
      description: 'Canvas pan offset and state'
    },
    {
      id: 'store:panelStore',
      category: 'store',
      label: 'panelStore',
      filePath: 'src/stores/panelStore.ts',
      description: 'Side panel width and visibility'
    },
    {
      id: 'store:interactionModeStore',
      category: 'store',
      label: 'interactionModeStore',
      filePath: 'src/stores/interactionModeStore.ts',
      description: 'Draw vs interact mode toggle'
    },
    {
      id: 'store:chunkPreviewStore',
      category: 'store',
      label: 'chunkPreviewStore',
      filePath: 'src/stores/chunkPreviewStore.ts',
      description: 'Chunk preview modal state'
    },

    // ==================== BACKEND MODULES ====================
    {
      id: 'backend:workflow',
      category: 'backend',
      label: 'workflow',
      filePath: 'src-tauri/src/workflow/mod.rs',
      description: 'Rust workflow engine, node registry, validation'
    },
    {
      id: 'backend:llm',
      category: 'backend',
      label: 'llm',
      filePath: 'src-tauri/src/llm/mod.rs',
      description: 'LLM inference and server management'
    },
    {
      id: 'backend:agent',
      category: 'backend',
      label: 'agent',
      filePath: 'src-tauri/src/agent/mod.rs',
      description: 'AI agent and RAG manager'
    },
    {
      id: 'backend:config',
      category: 'backend',
      label: 'config',
      filePath: 'src-tauri/src/config/mod.rs',
      description: 'Application configuration persistence'
    },
    {
      id: 'backend:hotload_sandbox',
      category: 'backend',
      label: 'hotload_sandbox',
      filePath: 'src-tauri/src/hotload_sandbox/mod.rs',
      description: 'Component sandboxing and hot-reloading'
    },

    // ==================== TAURI COMMANDS ====================
    // Workflow commands
    {
      id: 'command:execute_workflow',
      category: 'command',
      label: 'execute_workflow',
      description: 'Execute a workflow graph'
    },
    {
      id: 'command:validate_workflow_connection',
      category: 'command',
      label: 'validate_workflow_connection',
      description: 'Check if ports can connect'
    },
    {
      id: 'command:get_node_definitions',
      category: 'command',
      label: 'get_node_definitions',
      description: 'Fetch available node types'
    },
    {
      id: 'command:save_workflow',
      category: 'command',
      label: 'save_workflow',
      description: 'Persist workflow to disk'
    },
    {
      id: 'command:load_workflow',
      category: 'command',
      label: 'load_workflow',
      description: 'Load workflow from disk'
    },
    // LLM commands
    {
      id: 'command:send_vision_prompt',
      category: 'command',
      label: 'send_vision_prompt',
      description: 'Send image+text to vision LLM'
    },
    {
      id: 'command:run_agent',
      category: 'command',
      label: 'run_agent',
      description: 'Execute AI agent with tools'
    },
    {
      id: 'command:start_sidecar_llm',
      category: 'command',
      label: 'start_sidecar_llm',
      description: 'Start local LLM server'
    },
    // RAG commands
    {
      id: 'command:search_rag',
      category: 'command',
      label: 'search_rag',
      description: 'Vector search over documents'
    },
    {
      id: 'command:index_rag_documents',
      category: 'command',
      label: 'index_rag_documents',
      description: 'Index documents for RAG'
    },
    // Config commands
    {
      id: 'command:get_model_config',
      category: 'command',
      label: 'get_model_config',
      description: 'Get LLM model configuration'
    },
    {
      id: 'command:set_model_config',
      category: 'command',
      label: 'set_model_config',
      description: 'Update LLM model configuration'
    },
  ],

  connections: [
    // ==================== App imports components ====================
    { id: 'c1', source: 'component:App', target: 'component:Canvas', connectionType: 'import' },
    { id: 'c3', source: 'component:App', target: 'component:WorkflowGraph', connectionType: 'import' },
    { id: 'c4', source: 'component:App', target: 'component:SidePanel', connectionType: 'import' },
    { id: 'c5', source: 'component:App', target: 'component:TopBar', connectionType: 'import' },
    { id: 'c6', source: 'component:App', target: 'component:Toolbar', connectionType: 'import' },
    { id: 'c7', source: 'component:App', target: 'component:WorkflowToolbar', connectionType: 'import' },
    { id: 'c8', source: 'component:App', target: 'component:NodePalette', connectionType: 'import' },
    { id: 'c9', source: 'component:App', target: 'component:HotLoadContainer', connectionType: 'import' },

    // ==================== Component → Store subscriptions ====================
    { id: 'c10', source: 'component:App', target: 'store:viewModeStore', connectionType: 'subscription' },
    { id: 'c11', source: 'component:App', target: 'store:panelStore', connectionType: 'subscription' },
    { id: 'c12', source: 'component:Canvas', target: 'store:canvasStore', connectionType: 'subscription' },
    { id: 'c13', source: 'component:Canvas', target: 'store:interactionModeStore', connectionType: 'subscription' },
    { id: 'c14', source: 'component:WorkflowGraph', target: 'store:workflowStore', connectionType: 'subscription' },
    { id: 'c16', source: 'component:ChunkPreview', target: 'store:chunkPreviewStore', connectionType: 'subscription' },
    { id: 'c17', source: 'component:WorkflowToolbar', target: 'store:workflowStore', connectionType: 'subscription' },
    { id: 'c18', source: 'component:NodePalette', target: 'store:workflowStore', connectionType: 'subscription' },

    // ==================== Component → Service imports ====================
    { id: 'c20', source: 'component:Canvas', target: 'service:DrawingEngine', connectionType: 'import' },
    { id: 'c21', source: 'component:SidePanel', target: 'service:LLMService', connectionType: 'import' },
    { id: 'c22', source: 'component:SidePanel', target: 'service:AgentService', connectionType: 'import' },
    { id: 'c23', source: 'component:SidePanel', target: 'service:RagService', connectionType: 'import' },
    { id: 'c24', source: 'component:HotLoadContainer', target: 'service:HotLoadRegistry', connectionType: 'import' },
    { id: 'c25', source: 'component:WorkflowToolbar', target: 'service:WorkflowService', connectionType: 'import' },

    // ==================== Service → Tauri commands ====================
    { id: 'c30', source: 'service:WorkflowService', target: 'command:execute_workflow', connectionType: 'command' },
    { id: 'c31', source: 'service:WorkflowService', target: 'command:validate_workflow_connection', connectionType: 'command' },
    { id: 'c32', source: 'service:WorkflowService', target: 'command:get_node_definitions', connectionType: 'command' },
    { id: 'c33', source: 'service:WorkflowService', target: 'command:save_workflow', connectionType: 'command' },
    { id: 'c34', source: 'service:WorkflowService', target: 'command:load_workflow', connectionType: 'command' },
    { id: 'c35', source: 'service:LLMService', target: 'command:send_vision_prompt', connectionType: 'command' },
    { id: 'c36', source: 'service:LLMService', target: 'command:start_sidecar_llm', connectionType: 'command' },
    { id: 'c37', source: 'service:AgentService', target: 'command:run_agent', connectionType: 'command' },
    { id: 'c38', source: 'service:RagService', target: 'command:search_rag', connectionType: 'command' },
    { id: 'c39', source: 'service:RagService', target: 'command:index_rag_documents', connectionType: 'command' },
    { id: 'c40', source: 'service:ConfigService', target: 'command:get_model_config', connectionType: 'command' },
    { id: 'c41', source: 'service:ConfigService', target: 'command:set_model_config', connectionType: 'command' },

    // ==================== Commands → Backend modules ====================
    { id: 'c50', source: 'command:execute_workflow', target: 'backend:workflow', connectionType: 'uses' },
    { id: 'c51', source: 'command:validate_workflow_connection', target: 'backend:workflow', connectionType: 'uses' },
    { id: 'c52', source: 'command:get_node_definitions', target: 'backend:workflow', connectionType: 'uses' },
    { id: 'c53', source: 'command:save_workflow', target: 'backend:workflow', connectionType: 'uses' },
    { id: 'c54', source: 'command:load_workflow', target: 'backend:workflow', connectionType: 'uses' },
    { id: 'c55', source: 'command:send_vision_prompt', target: 'backend:llm', connectionType: 'uses' },
    { id: 'c56', source: 'command:start_sidecar_llm', target: 'backend:llm', connectionType: 'uses' },
    { id: 'c57', source: 'command:run_agent', target: 'backend:agent', connectionType: 'uses' },
    { id: 'c58', source: 'command:search_rag', target: 'backend:agent', connectionType: 'uses' },
    { id: 'c59', source: 'command:index_rag_documents', target: 'backend:agent', connectionType: 'uses' },
    { id: 'c60', source: 'command:get_model_config', target: 'backend:config', connectionType: 'uses' },
    { id: 'c61', source: 'command:set_model_config', target: 'backend:config', connectionType: 'uses' },

    // ==================== Backend cross-module uses ====================
    { id: 'c70', source: 'backend:workflow', target: 'backend:llm', connectionType: 'uses' },
    { id: 'c71', source: 'backend:workflow', target: 'backend:agent', connectionType: 'uses' },
    { id: 'c72', source: 'backend:agent', target: 'backend:llm', connectionType: 'uses' },
  ]
};
