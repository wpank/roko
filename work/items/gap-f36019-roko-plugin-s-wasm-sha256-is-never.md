+++
id = "gap-f36019"
kind = "gap"
title = "roko-plugin's wasm_sha256 is never set or checked, though its doc said the loader re-verifies it"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-plugin"]
created = 2026-10-08
updated = 2026-10-08
last_verified = 2026-10-08
source = "stash-triage-2026-10-08 (roko-worktree-archive/2026-10-08-stash-triage/TRIAGE.md)"
discovered_from = "stash triage side finding"
anchors = ["crates/roko-plugin/src/manifest.rs::PluginManifestFile"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rn 'wasm_sha256' crates/roko-plugin/src --include='*.rs' | grep -v 'src/manifest.rs' | grep -q ."
+++

## Problem

`PluginManifestFile::wasm_sha256` (`crates/roko-plugin/src/manifest.rs`) was documented as "the loader re-verifies the
on-disk binary before executing any hooks. Set during installation". Nothing sets the field and nothing checks it,
so a reader trusted a check that does not exist.

## Why it matters

Plugin integrity: a tampered WASM binary would run unchecked while the docs say otherwise. The WASM runtime is
dormant today, so the risk is latent.

## Where

`crates/roko-plugin/src/manifest.rs::PluginManifestFile::wasm_sha256`, the plugin installer and the WASM loader.

## Current state

The doc comment now says the field is not checked (batch 24). The 2026-09-21 stash had an implementation (triage:
`roko-worktree-archive/2026-10-08-stash-triage/unsure-05-wasm-checksum.patch`).

## Plan

Set the checksum at install time and verify it before loading the module; refuse a mismatch. Or, if WASM plugins are
not planned, remove the field.

## Done when

- [ ] Installation records the hash and loading refuses a mismatched binary, or the field is removed.
- [ ] `[[verify]]` passes.
