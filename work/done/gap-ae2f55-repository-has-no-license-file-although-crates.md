+++
id = "gap-ae2f55"
kind = "gap"
title = "Repository has no LICENSE file although crates declare MIT OR Apache-2.0"
status = "done"
triage = "verified"
severity = "p1"
goal = "release"
subsystem = ["release"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["Cargo.toml:104"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'ls LICENSE* >/dev/null 2>&1'

[[verify]]
command = 'ls LICENSE* >/dev/null 2>&1'

[closed]
at = 2026-09-29
by = "Will (licence decision); files added by Claude"
evidence = "LICENSE-MIT (standard MIT text, Copyright (c) 2026 Will Pankiewicz) and LICENSE-APACHE (canonical Apache-2.0 text) were added in the same commit that closed this item; the verify command passes. The question about another contributor's code in apps/mirage-rs is tracked separately."
+++

`Cargo.toml` declares `license = "MIT OR Apache-2.0"` (`:104`), but the repository root has no LICENSE or COPYING file, so the terms are not shipped with the source.
Fix: add `LICENSE-MIT` and `LICENSE-APACHE` (plus a NOTICE if third-party terms require one) and a license/advisory check (e.g. cargo-deny) in CI.

2026-09-28: Will chose MIT OR Apache-2.0, with Will Pankiewicz as copyright holder. `LICENSE-MIT` (standard text) and `LICENSE-APACHE` (canonical text) are drafted in the working tree and not yet committed. Open question: 1,357 lines by another contributor (JaeLeex, April 2026) survive in `apps/mirage-rs`, mostly `precompiles/hdc.rs` (839) and `rpc.rs` (320). They need that contributor's agreement, a carve-out, or a rewrite. None of simp-son's 5 lines survive.
