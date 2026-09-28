+++
id = "bug-053644"
kind = "bug"
title = "roko backlog list only sees numerically prefixed files in a hard-coded directory"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/backlog"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/plan_generate.rs:597", "crates/roko-cli/src/commands/backlog.rs", "crates/roko-cli/src/plan_generate.rs:651", "crates/roko-cli/src/commands/backlog.rs::cmd_backlog_list", "crates/roko-cli/src/commands/backlog.rs::has_imported_idea"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`backlog list` reads `DEFAULT_BACKLOG_DIR = "tmp/backlog"` (`plan_generate.rs:597`); per a local audit it accepts only files whose prefix parses as a `u32`, lists a status summary as item `#0`, ignores archive directories, never shows the `**Status**:` line that `mark-done` writes, and checks for imports in a path the CLI does not write.
Fix: configurable source directories, a tolerant id grammar, and status/import detection that matches what the other backlog commands write.

Verified 2026-09-28 (static check against 3d0ee4d02): cmd_backlog_list (crates/roko-cli/src/commands/backlog.rs:57-104) hard-codes workdir.join("tmp/backlog") (:58; DEFAULT_BACKLOG_DIR itself now at plan_generate.rs:651), reads only top-level *.md whose first '-' segment parses as u32 (:65-80), skips only 00-INDEX so 00-STATUS-SUMMARY.md lists as #0, never descends into archive/, and prints only 'imported'/'-' (never the **Status**: line mark-done writes at :819). has_imported_idea reads .roko/prd/ideas/ideas.md (:85,:107-113) while import writes through prd::cmd_idea to .roko/prd/ideas.md (workspace_paths.rs:29-30), so imports are never detected. No commit or uncommitted edit touches backlog.rs.
