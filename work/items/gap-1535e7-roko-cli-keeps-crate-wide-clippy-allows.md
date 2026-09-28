+++
id = "gap-1535e7"
kind = "gap"
title = "roko-cli keeps crate-wide clippy allows and 156 #[allow(dead_code)] sites"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/lints", "workspace/hygiene"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#allowdead_code-sites----hygiene--deferred"
anchors = ["crates/roko-cli/src/lib.rs:14", "crates/roko-cli/src/main.rs:11"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

The 2026-08-16 inventory of 48 standalone attributes is out of date: there are now 156 `#[allow(dead_code)]` attributes under `crates/`. The crate-wide `dead_code` allow in `crates/roko-cli/src/lib.rs` is commented out ("TEMPORARILY DISABLED FOR AUDIT"). Broad crate-wide clippy allows remain in `lib.rs:14-24` and `main.rs:11-16`. Backlog #43 (clippy suppression removal) is archived as Blocked. The earlier audits estimated about 4,500 lines of removable dead CLI code behind the blanket allow.

Fix: decide whether the `dead_code` allow stays off, and remove or justify each crate-wide clippy allow. Give every remaining `#[allow(dead_code)]` an owner and a reason, or delete the code.
