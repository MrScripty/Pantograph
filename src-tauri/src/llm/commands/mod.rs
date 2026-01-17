//! Tauri command modules for LLM functionality.
//!
//! This module organizes commands by domain:
//! - `vision`: Vision/image prompt handling
//! - `agent`: Agent orchestration and execution
//! - `rag`: Retrieval Augmented Generation
//! - `config`: Model, device, and app configuration
//! - `server`: LLM server lifecycle management
//! - `backend`: Backend switching and capabilities
//! - `binary`: Binary download and management
//! - `docs`: Documentation and chunking
//! - `sandbox`: Sandbox configuration
//! - `embedding`: Embedding server and memory modes

mod agent;
mod backend;
mod binary;
mod config;
mod docs;
mod embedding;
mod rag;
mod sandbox;
mod server;
mod vision;

// Shared utilities used by multiple command modules
mod shared;

// Re-export all commands for backwards compatibility
// main.rs imports from llm::commands::*
pub use agent::*;
pub use backend::*;
pub use binary::*;
pub use config::*;
pub use docs::*;
pub use embedding::*;
pub use rag::*;
pub use sandbox::*;
pub use server::*;
pub use vision::*;

// Re-export shared types that are part of the public API
pub use shared::SharedAppConfig;
