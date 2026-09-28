+++
id = "gap-18b3c9"
kind = "gap"
title = "[provider F142] Rust-specific build commands hardcoded in role identities"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/templates"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F142"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F142"
anchors = ["crates/roko-compose/src/templates/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Implementer, reviewer, and other role prompts reference `cargo check`, `cargo test`, `cargo clippy`, `Cargo.toml` directly. These would need parameterization for use with non-Rust projects.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F142`
- `tmp/archive/provider-audit/15-prompt-composition.md`

How to verify: Confirm in crates/roko-compose/src/templates/ whether still true: Rust-specific build commands hardcoded in role identities
