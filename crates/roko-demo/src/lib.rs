#![deny(unsafe_code)]
#![warn(missing_docs)]
// Demo crate uses alloy types with required .into() conversions and trait impls with literal returns.
#![allow(
    clippy::implicit_hasher,
    clippy::literal_string_with_formatting_args,
    clippy::unnecessary_literal_bound,
    clippy::manual_clamp,
    clippy::struct_field_names,
    clippy::too_many_lines,
    clippy::useless_conversion
)]

//! Manifest-driven orchestrator for the roko demo environment.
//!
//! See `roko/demo/` for the declarative config + scenarios consumed by this
//! crate's `roko-demo` binary.

pub mod autonomous;
pub mod benchmark;
pub mod bindings;
pub mod chain_ctx;
pub mod deploy;
pub mod events;
pub mod fixtures;
pub mod manifest;
pub mod scenarios;
pub mod tournament;
pub mod tui;
pub mod verify;
pub mod ws_server;

pub use chain_ctx::ChainCtx;
pub use deploy::{ContractArtifact, DeployCtx, DeployedSuite, deploy_suite};
pub use fixtures::{FixtureRegistry, RustFixture, run_fixtures};
pub use manifest::{LoadedManifest, Manifest, Scenario};
