//! # UFC Plugin Host
//!
//! Manages plugin registration, discovery, and sandboxed execution.

pub mod registry;
pub mod sandbox;

pub use registry::PluginRegistry;
pub use sandbox::SandboxManager;
