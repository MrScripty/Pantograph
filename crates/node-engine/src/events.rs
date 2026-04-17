//! Event types for streaming workflow progress.
//!
//! The event contract and sink implementations live in focused submodules so
//! execution, transport, and binding work can extend them without deepening a
//! single catch-all file.

mod contract;
mod sinks;

#[cfg(test)]
mod tests;

pub use contract::WorkflowEvent;
pub(crate) use contract::unix_timestamp_ms;
pub use sinks::{
    BroadcastEventSink, CallbackEventSink, CompositeEventSink, EventError, EventSink,
    NullEventSink, VecEventSink,
};
