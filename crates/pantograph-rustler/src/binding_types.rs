use rustler::{NifStruct, NifUnitEnum};

/// Port data type enum for Elixir.
#[derive(NifUnitEnum)]
pub enum ElixirPortDataType {
    String,
    Number,
    Boolean,
    Json,
    KvCache,
    Image,
    Audio,
    Video,
    Embedding,
    Document,
    Binary,
    Any,
}

/// Node category enum for Elixir.
#[derive(NifUnitEnum)]
pub enum ElixirNodeCategory {
    Input,
    Output,
    Processing,
    Control,
    Storage,
    Integration,
}

/// Execution mode enum for Elixir.
#[derive(NifUnitEnum)]
pub enum ElixirExecutionMode {
    Reactive,
    Manual,
    Stream,
}

/// Orchestration node type enum for Elixir.
#[derive(NifUnitEnum)]
pub enum ElixirOrchestrationNodeType {
    Start,
    End,
    DataGraph,
    Condition,
    Loop,
    Merge,
}

/// Node definition struct for Elixir (metadata about a node type).
#[derive(NifStruct)]
#[module = "Pantograph.NodeDefinition"]
pub struct ElixirNodeDefinition {
    pub node_type: String,
    pub category: ElixirNodeCategory,
    pub label: String,
    pub description: String,
    pub input_count: u32,
    pub output_count: u32,
    pub execution_mode: ElixirExecutionMode,
}

/// Cache statistics struct for Elixir.
#[derive(NifStruct)]
#[module = "Pantograph.CacheStats"]
pub struct ElixirCacheStats {
    pub cached_nodes: u32,
    pub total_versions: u32,
    pub global_version: u64,
}

/// Orchestration graph metadata for Elixir.
#[derive(NifStruct)]
#[module = "Pantograph.OrchestrationMetadata"]
pub struct ElixirOrchestrationMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub node_count: u32,
}
