//! Tool-dispatch safety enforcement (§36.e).
//!
//! Each submodule owns **one policy family** that gates a specific
//! capability before the dispatcher hands a tool call to its handler.
//! Policies are **pure validators**: they take a call + context and
//! return a verdict — no side effects, no mutation of the caller's state.
//!
//! # Families (wave 1)
//!
//! - [`path`] (§36.46) — worktree-relative canonicalization & escape prevention
//! - [`bash`] (§36.47) — command allowlist / denylist for the `bash` tool
//! - [`network`] (§36.48) — outbound-destination allowlist for network tools
//!
//! # Families (later waves)
//!
//! - `git` (§36.49) — branch-protection policy
//! - `scrub` (§36.50) — secret-scrubbing from outputs
//! - `rate_limit` (§36.51) — per-tool / per-role rate limits
//! - `audit` (§36.52) — append-only JSONL audit log (lives in `roko-fs`)
//!
//! # Composition
//!
//! Each policy exposes a `check(...)` that returns `Result<(), ToolError>`.
//! The dispatcher chains them in order; the first failure short-circuits
//! and is returned verbatim to the caller.

#![allow(clippy::module_name_repetitions)]

pub mod bash;
pub mod git;
pub mod network;
pub mod path;
pub mod rate_limit;
pub mod scrub;
