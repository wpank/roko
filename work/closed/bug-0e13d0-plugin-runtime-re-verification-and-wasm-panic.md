+++
id = "bug-0e13d0"
kind = "bug"
title = "Plugin Runtime Re-Verification and WASM Panic Boundary"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-plugin"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/93-plugin-runtime-verification.md#93 — Plugin Runtime Re-Verification and WASM Panic Boundary"
discovered_from = "audit:tmp/backlog/archive/93-plugin-runtime-verification.md#93 — Plugin Runtime Re-Verification and WASM Panic Boundary"
anchors = ["crates/roko-cli/src/runner/extension_loader.rs::WasmExtension", "crates/roko-plugin/src/manifest.rs:100", "crates/roko-plugin/src/registry.rs::validate_signed_package"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
evidence = "Obsolete: the WASM hook runtime this item hardens was deleted with Runner-v2. WasmExtension::load (crates/roko-cli/src/runner/extension_loader.rs:42-60) always returns an error, so tier=\"wasm\" plugins fail closed and are skipped, and no wasmtime/invoke_json mutex is left to poison. Install-time package checksums remain (crates/roko-plugin/src/registry.rs:175-179). If a WASM runtime comes back, it must re-verify manifest wasm_sha256 at load and add a panic boundary. Note: the doc comment at manifest.rs:95-99 still claims a load-time re-verification that no code performs."
+++
Security gap: installed plugins are not re-verified on load, and a WASM panic can permanently poison the WASM runtime mutex for all subsequent hooks. Roko supports WASM plugins that are installed from a registry package. During installation (`roko plugin install`) and during relay verification…

Imported without verification from:
- `tmp/backlog/archive/93-plugin-runtime-verification.md#93 — Plugin Runtime Re-Verification and WASM Panic Boundary`

Some cited files are gone: `src/runner/wasm_extension.rs`, `workdir/plugins`.

How to verify: Check: A plugin whose WASM binary is modified on disk after installation is rejected at startup with an error log and skipped (does not crash the runner).; A plugin without a `wasm_sha256` field in its manifest (hand-authored or pre-installation)… [evidence: 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): S | 3 |]

Verified 2026-09-28: obsolete. The WASM runtime was removed (extension_loader.rs:42-60); see [closed].
