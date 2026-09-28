+++
id = "bug-4cb029"
kind = "bug"
title = "Quarantine routes read a file the runtime never writes"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/safety"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/safety.rs:48", "crates/roko-serve/src/routes/safety.rs:94", "crates/roko-agent/src/tool_immune.rs::QUARANTINE_VAULT_RELATIVE_PATH"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "join(\"quarantine.json\")" crates/roko-serve/src/routes/safety.rs'

[[verify]]
command = 'cargo test -p roko-serve routes::safety'
+++

The safety quarantine handlers read a `quarantine.json` file (`routes/safety.rs:48`, `:94`), but the tool immune layer persists its vault at `.roko/immune/quarantine-vault.json` (`roko-agent/src/tool_immune.rs:33`).
The routes always report an empty quarantine while the runtime is quarantining tool results.
Fix: read the vault through the runtime's path helper and type, and add a route test that sees an entry the immune layer wrote.
