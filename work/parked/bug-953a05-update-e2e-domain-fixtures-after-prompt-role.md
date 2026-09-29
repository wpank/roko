+++
id = "bug-953a05"
kind = "bug"
title = "Update e2e_domain fixtures after [prompt].role removal (5 ignored domain tests)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/domains"]
created = 2026-09-05
updated = 2026-09-28
source = "crates/roko-cli/tests/e2e_domain.rs:98"
discovered_from = "audit:crates/roko-cli/tests/e2e_domain.rs:98"
anchors = ["crates/roko-cli/tests/e2e_domain.rs::config_with_default_domain_parses", "crates/roko-cli/tests/e2e_domain.rs::run_with_research_domain_uses_shell_gate", "crates/roko-cli/tests/e2e_domain.rs::tasks_with_custom_domain_validates"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Five domain e2e tests are ignored because the config schema changed during engine convergence ([prompt].role removed) and fixtures were never updated; domain config/gate/validation behaviour is untested end-to-end.

Imported without verification from:
- `crates/roko-cli/tests/e2e_domain.rs:98`
- `crates/roko-cli/tests/e2e_domain.rs:115`
- `crates/roko-cli/tests/e2e_domain.rs:136`
- `crates/roko-cli/tests/e2e_domain.rs:192`
- `crates/roko-cli/tests/e2e_domain.rs:220`

How to verify: cargo test -p roko-cli --test e2e_domain -- --ignored; rewrite fixtures without [prompt].role.
