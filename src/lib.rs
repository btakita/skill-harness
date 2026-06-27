//! Agent Skills — management for contextually-activated instruction bundles.
//!
//! Provides skill install/check/uninstall for AI agent environments.
//! When the `detect` feature is enabled, uses `agent-kit` for environment detection.

pub mod compose;
pub mod manage;
pub mod okf;

pub use manage::SkillConfig;

#[cfg(feature = "detect")]
pub use agent_kit::detect::Environment;
