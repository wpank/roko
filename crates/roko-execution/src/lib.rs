//! Profile-driven runtime service builder for Roko execution surfaces (#243).
//!
//! This crate provides [`RuntimeServicesBuilder`] which constructs shared
//! service bundles (dispatch, prompt, feedback, extensions, observation,
//! guards) from a validated config and a [`RuntimeProfile`]. Both Runner-v2
//! and Graph plan engines consume the same builder so that provider health,
//! rate limiter, cost table, prompt cache, and shutdown/process supervisor
//! contracts are shared.
//!
//! # Layer
//!
//! This crate is layer 3. It may depend on layer 0-3 crates but must never
//! depend on `roko-cli`, `roko-serve`, or `roko-acp` (all layer 4).
//!
//! # Modules
//!
//! - [`diagnostics`] -- Shared diagnostic and preflight service (#279).
//! - [`replan_controller`] -- Durable Graph gate-failure replan controller (#252).
//! - [`profiles`] -- RuntimeProfile enum and profile bundle matrix.
//! - [`builder`] -- RuntimeServicesBuilder and service bundle types.
//! - [`lifecycle`] -- Runner lifecycle event types for #208 envelope.

pub mod diagnostics;
pub mod profiles;
pub mod replan_controller;
pub mod runtime_services;

pub use profiles::{
    BundleRequirement, ProfileMatrix, RuntimeProfile, ServiceBundleId,
};
pub use runtime_services::{
    CostSettlement, CostSettlementError, NonPlanServiceHandle, NonPlanServiceRequest,
    ServiceConstructionError, ShutdownRegistration, build_non_plan_services,
    overrides_for_acp, overrides_for_chat, overrides_for_workflow, validate_service_request,
};
