+++
id = "bug-73aa85"
kind = "bug"
title = "Fix `EffectsPreset` default from `Off` to `Minimal`"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#1.10 Fix `EffectsPreset` default from `Off` to `Minimal`"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#1.10 Fix `EffectsPreset` default from `Off` to `Minimal`"
anchors = ["crates/roko-cli/src/tui/effects_config.rs:86-88", "EffectsConfig::default()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Change `EffectsConfig::default()` to `Self::from_preset(EffectsPreset::Minimal)`. Source fix is prepared; needs final live check.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#1.10 Fix `EffectsPreset` default from `Off` to `Minimal``

How to verify: Source: UX audit (P5 quick win); TUI parity P4.4. Check `crates/roko-cli/src/tui/effects_config.rs:86-88` for: Change `EffectsConfig::default()` to `Self::from_preset(EffectsPreset::Minimal)`. Source fix is prepared; needs final live check.
