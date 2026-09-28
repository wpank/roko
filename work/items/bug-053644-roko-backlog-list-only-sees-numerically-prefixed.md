+++
id = "bug-053644"
kind = "bug"
title = "roko backlog list only sees numerically prefixed files in a hard-coded directory"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/backlog"]
created = 2026-09-28
updated = 2026-09-28
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/plan_generate.rs:597", "crates/roko-cli/src/commands/backlog.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`backlog list` reads `DEFAULT_BACKLOG_DIR = "tmp/backlog"` (`plan_generate.rs:597`); per a local audit it accepts only files whose prefix parses as a `u32`, lists a status summary as item `#0`, ignores archive directories, never shows the `**Status**:` line that `mark-done` writes, and checks for imports in a path the CLI does not write.
Fix: configurable source directories, a tolerant id grammar, and status/import detection that matches what the other backlog commands write.
