+++
id = "gap-2f69e9"
kind = "gap"
title = "Quarantine vault caps at 50 entries and has no review surface"
status = "open"
triage = "verified"
severity = "p3"
goal = "features"
subsystem = ["roko-core/immune", "roko-serve/safety"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-core/src/immune.rs::DEFAULT_QUARANTINE_VAULT_CAPACITY", "crates/roko-core/src/immune.rs::QuarantineVault", "crates/roko-core/src/immune.rs:38", "crates/roko-core/src/immune.rs::QuarantineVault::validate_integrity", "crates/roko-serve/src/routes/safety.rs::quarantine_handler"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE 'route\\(\"/safety/quarantine/[^\"]+\"' crates/roko-serve/src/routes/safety.rs && ! grep -q 'max_entries > DEFAULT_QUARANTINE_VAULT_CAPACITY' crates/roko-core/src/immune.rs"
+++

`QuarantineVault` defaults to `DEFAULT_QUARANTINE_VAULT_CAPACITY = 50` (`roko-core/src/immune.rs:38`, used at `:460`) and exposes no entry or capacity getters.
Per a local audit there is no route or CLI to list, release or confirm a quarantined item, so once the vault is full older quarantines disappear without an operator seeing them.
Fix: configurable capacity, evictions recorded as incidents, and a review surface (list / release / confirm) over the vault.

Verified 2026-09-28 (static check against 3d0ee4d02): Capacity is fixed: with_defaults uses DEFAULT_QUARANTINE_VAULT_CAPACITY = 50 (crates/roko-core/src/immune.rs:38, :460), and validate_integrity rejects max_entries > 50 on load (:803-805). There is no release/confirm surface: roko-serve registers only GET /safety/quarantine and GET /safety/incidents (crates/roko-serve/src/routes/safety.rs:9-10), no CLI command exists, and review() (immune.rs:556) is called only automatically at the tool boundary (crates/roko-agent/src/tool_immune.rs:871). Two parts of the description are wrong: entry getters exist (get :667, pending :673, stats :707, count :889, is_full :895) and GET /safety/quarantine already lists entries (safety.rs:43-60); and a full vault refuses new entries (quarantine returns false at immune.rs:487-489; tool_immune.rs:865-868 errors 'quarantine vault is at capacity') rather than evicting older ones. immune.rs was last changed in 244f564e1.

Re-checked 2026-09-29 at d9e79e9d8: unchanged. The capacity is still fixed at 50, and serve still exposes only the two GET routes (safety.rs:18-19). There is still no release or confirm route and no CLI command.
