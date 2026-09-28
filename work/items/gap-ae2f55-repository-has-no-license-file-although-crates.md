+++
id = "gap-ae2f55"
kind = "gap"
title = "Repository has no LICENSE file although crates declare MIT OR Apache-2.0"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
subsystem = ["release"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["Cargo.toml:104"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'ls LICENSE* >/dev/null 2>&1'

[[verify]]
command = 'ls LICENSE* >/dev/null 2>&1'
+++

`Cargo.toml` declares `license = "MIT OR Apache-2.0"` (`:104`), but the repository root has no LICENSE or COPYING file, so the terms are not shipped with the source.
Fix: add `LICENSE-MIT` and `LICENSE-APACHE` (plus a NOTICE if third-party terms require one) and a license/advisory check (e.g. cargo-deny) in CI.
