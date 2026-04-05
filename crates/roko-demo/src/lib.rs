#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Manifest-driven orchestrator for the roko demo environment.
//!
//! See `roko/demo/` for the declarative config + scenarios consumed by this
//! crate's `roko-demo` binary.

pub mod bindings;
pub mod chain_ctx;
pub mod deploy;
pub mod fixtures;
pub mod manifest;
pub mod scenarios;
pub mod verify;

pub use chain_ctx::ChainCtx;
pub use deploy::{ContractArtifact, DeployCtx, DeployedSuite, deploy_suite};
pub use fixtures::{FixtureRegistry, RustFixture, run_fixtures};
pub use manifest::{LoadedManifest, Manifest, Scenario};
