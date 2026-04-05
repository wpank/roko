//! The `roko` binary's library surface.
//!
//! This crate wires Roko's primitives (Substrate, Composer, Agent, Gate,
//! Policy) into a one-shot CLI loop. It does **not** implement a plan runner
//! or DAG executor — it drives a single prompt through the universal loop
//! and writes the resulting signals to disk.
//!
//! See [`run_once`] for the core loop and [`Config`] for the `roko.toml`
//! schema.

#![allow(clippy::module_name_repetitions)]

pub mod clean;
pub mod config;
pub mod config_cmd;
pub mod episode;
pub mod run;

pub use config::{
    load_layered, AgentConfig, Config, ConfigLayer, ConfigPaths, ConfigSources, GateConfig,
    PromptConfig, PromptFile, ResolvedConfig, Source,
};
pub use config_cmd::{run_init_wizard, EditTarget, WizardInputs};
pub use episode::EpisodePolicy;
pub use run::{run_once, RunReport};
