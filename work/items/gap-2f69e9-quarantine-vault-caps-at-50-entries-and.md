+++
id = "gap-2f69e9"
kind = "gap"
title = "Quarantine vault caps at 50 entries and has no review surface"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/immune", "roko-serve/safety"]
created = 2026-09-28
updated = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-core/src/immune.rs::DEFAULT_QUARANTINE_VAULT_CAPACITY", "crates/roko-core/src/immune.rs::QuarantineVault"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`QuarantineVault` defaults to `DEFAULT_QUARANTINE_VAULT_CAPACITY = 50` (`roko-core/src/immune.rs:38`, used at `:460`) and exposes no entry or capacity getters.
Per a local audit there is no route or CLI to list, release or confirm a quarantined item, so once the vault is full older quarantines disappear without an operator seeing them.
Fix: configurable capacity, evictions recorded as incidents, and a review surface (list / release / confirm) over the vault.
