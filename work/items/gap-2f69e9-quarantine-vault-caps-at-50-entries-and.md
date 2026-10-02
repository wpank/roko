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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
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

## Notes

- 2026-10-01 (wk-filer4): implemented the first step (the review listing) on work/gap-cd51b7; cargo verification deferred to the batch check. Release, confirm and the capacity need design calls (below), so the item stays open and its verify still fails.
- 2026-10-01 (wk-filer4): What changed: `GET /api/safety/quarantine` used to list only pending entries, by a shortened hash. It now lists every entry, escalated ones included, each with its review `status` and its `full_hash`. Each vault also reports its `capacity` and whether it is `full`, and `/safety/incidents` covers every entry. This needed two new getters in roko-core: `QuarantineVault::entries()` (oldest first) and `capacity()`. docs/v3/26-HTTP-API.md says the same.
- 2026-10-01 (wk-filer4): Design calls left. (1) Capacity: raising it first needs a bound on incident links. `quarantine_scoped` links each entry to every other entry from the same source, so one noisy source makes n(n-1) links of about 200 bytes each in the pretty-printed vault. That is about 0.5 MB at 50 entries, and past the 4 MiB transaction limit (`MAX_QUARANTINE_VAULT_BYTES`) at about 145, after which the boundary cannot write the vault. A configured capacity also has to reach `update_vault` (crates/roko-agent/src/tool_immune.rs:849), which takes no config today. (2) Release and confirm: decide what a review does beyond setting the status. `Approved` and `Rejected` promise a release or a purge that nothing performs (`drain_resolved` has no caller), resolved entries count against capacity until they are drained, and the source's tool control stays in force. A mutating route gets `ConfigEdit` by default (crates/roko-serve/src/routes/route_permissions.rs). (3) The "evictions" premise is wrong: a full vault refuses new entries (immune.rs:487-489). The boundary then still denies the result, but it only logs "quarantine vault is at capacity" and writes no receipt (tool_immune.rs:865-868).
