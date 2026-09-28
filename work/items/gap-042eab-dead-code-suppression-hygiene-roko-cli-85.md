+++
id = "gap-042eab"
kind = "gap"
title = "Dead-code suppression hygiene: roko-cli (85 allow(dead_code))"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/hygiene"]
created = 2026-09-25
updated = 2026-09-28
source = "crates/roko-cli/src/plan_generator.rs"
discovered_from = "audit:crates/roko-cli/src/plan_generator.rs"
anchors = ["crates/roko-cli/src/plan_generator.rs", "crates/roko-cli/src/tui/state/mod.rs", "crates/roko-cli/tests/side_effect_parity.rs", "crates/roko-cli/tests/gate_parity.rs", "crates/roko-cli/tests/common/mod.rs", "crates/roko-cli/src/tui/theme.rs", "crates/roko-cli/src/knowledge_helpers.rs", "crates/roko-runtime/src/state_snapshot.rs", "crates/roko-runtime/tests/builtin_lenses_health.rs", "crates/roko-runtime/src/adapters/telegram.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
85 #[allow(dead_code)] in roko-cli. CLI audit reported blanket suppression masking warnings. Top: src/plan_generator.rs (17), src/tui/state/mod.rs (6), tests/side_effect_parity.rs (5), tests/gate_parity.rs (5). Module-level: tests/common/mod.rs, src/tui/theme.rs, src/knowledge_helpers.rs.

Imported without verification from:
- `crates/roko-cli/src/plan_generator.rs`
- `crates/roko-cli/src/tui/state/mod.rs`
- `crates/roko-cli/tests/side_effect_parity.rs`
- `crates/roko-cli/tests/gate_parity.rs`
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#6. CLI Audit`
- `crates/roko-runtime/src/state_snapshot.rs`
- `crates/roko-runtime/tests/builtin_lenses_health.rs`
- `crates/roko-runtime/src/adapters/telegram.rs`
- `crates/roko-runtime/src/adapters/mattermost.rs`
- `crates/roko-serve/src/routes/webhooks.rs`
- `crates/roko-serve/src/terminal.rs`
- `crates/roko-serve/src/routes/status/helpers.rs`

How to verify: Remove each allow in roko-cli, run cargo clippy -p roko-cli --all-targets -- -D warnings, delete or wire what is flagged. / Remove each allow in roko-runtime, run cargo clippy -p roko-runtime --all-targets -- -D warnings, delete or wire what is flagged. / Remove each allow in roko-serve, run cargo clippy -p roko-serve --all-targets -- -D warnings, delete or wire what is flagged.

Merged 13 mined candidates: m5-024, m5-025, m5-026, m5-027, m5-028, m5-029, m5-030, m5-031, m5-032, m5-033, m5-034, m5-035, m5-036.
