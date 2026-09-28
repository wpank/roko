+++
id = "bug-92bd7d"
kind = "bug"
title = "WASM extension tests ignored: 'extension system dormant; fixtures not built for current wasmtime'"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/extensions"]
created = 2026-09-21
updated = 2026-09-28
source = "crates/roko-cli/src/runner/extension_loader.rs:2841"
discovered_from = "audit:crates/roko-cli/src/runner/extension_loader.rs:2841"
anchors = ["crates/roko-cli/src/runner/extension_loader.rs::wasm_extension_loads_and_executes_exported_hook", "crates/roko-cli/src/runner/extension_loader.rs::wasm_extension_infinite_hook_exhausts_fuel", "crates/roko-cli/src/serve_runtime.rs::serve_preflight_rejects_required_init_failure_before_bind"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Four WASM extension tests (load+execute hook, fuel exhaustion, required-load fatal init, serve preflight) are bare #[ignore] with a comment that the WASM extension system is dormant. Conflicts with E32 claims of bounded all-hook WASM execution.

Imported without verification from:
- `crates/roko-cli/src/runner/extension_loader.rs:2841`
- `crates/roko-cli/src/runner/extension_loader.rs:2859`
- `crates/roko-cli/src/runner/extension_loader.rs:2911`
- `crates/roko-cli/src/serve_runtime.rs:1524`

How to verify: Run with --ignored; check wasmtime version vs fixture WAT; reconcile with E32 status in .roko/GAPS.md.
