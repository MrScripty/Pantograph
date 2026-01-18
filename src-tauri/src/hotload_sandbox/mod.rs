//! Hotload Sandbox Module
//!
//! Provides sandboxed validation of Svelte components before they are written to disk.
//! Uses two-stage validation:
//! 1. Svelte syntax validation via Node.js (existing approach)
//! 2. Runtime semantic validation via boa_engine sandbox
//!
//! This module catches errors that pass syntax validation but would fail at runtime,
//! such as using primitive values as components.

pub mod runtime_sandbox;
pub mod svelte_validator;
