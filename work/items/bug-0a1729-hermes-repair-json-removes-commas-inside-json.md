+++
id = "bug-0a1729"
kind = "bug"
title = "Hermes repair_json removes commas inside JSON string values"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-agent/translate"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/translate/hermes.rs::repair_json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-agent --lib translate::hermes'
+++

`repair_json` (`translate/hermes.rs:258`) walks characters and drops every `,` followed by whitespace and `}` / `]` without tracking whether it is inside a string literal.
It runs after a failed first parse (e.g. a genuine trailing comma), so the repaired call can carry altered argument text such as `"a, ]"` -> `"a ]"`.
Fix: track string and escape state so only structural trailing commas are removed; add a test with a comma before a bracket inside a string.
