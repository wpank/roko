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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "gaps-md#allowdead_code-sites----hygiene--deferred"
anchors = ["crates/roko-cli/src/lib.rs:14", "crates/roko-cli/src/main.rs:11"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'TEMPORARILY DISABLED FOR AUDIT' crates/roko-cli/src/lib.rs && ! grep -rn --include='*.rs' '#\\[allow(dead_code)\\]' crates/ | grep -v /target/ | grep -q . && cargo clippy -p roko-cli --no-deps -- -D warnings"
+++

The 2026-08-16 inventory of 48 standalone attributes is out of date: there are now 156 `#[allow(dead_code)]` attributes under `crates/`. The crate-wide `dead_code` allow in `crates/roko-cli/src/lib.rs` is commented out ("TEMPORARILY DISABLED FOR AUDIT"). Broad crate-wide clippy allows remain in `lib.rs:14-24` and `main.rs:11-16`. Backlog #43 (clippy suppression removal) is archived as Blocked. The earlier audits estimated about 4,500 lines of removable dead CLI code behind the blanket allow.

Fix: decide whether the `dead_code` allow stays off, and remove or justify each crate-wide clippy allow. Give every remaining `#[allow(dead_code)]` an owner and a reason, or delete the code.

Rechecked 2026-09-29 at d9e79e9d8. The crate-wide allow blocks are larger than the body says: lib.rs:14-101 lists 83 clippy lints and main.rs:11-35 lists 19. The dead_code allow is still commented out ('TEMPORARILY DISABLED FOR AUDIT'). The count is now 151 '#[allow(dead_code)]' attributes under crates/, or 167 'allow(dead_code)' matches counting multi-lint and cfg_attr forms, 80 of them in roko-cli.
