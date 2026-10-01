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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "c58c7c2ba"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/translate/hermes.rs::repair_json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn repair_json_keeps_commas_inside_strings' crates/roko-agent/src/translate/hermes.rs && cargo test -p roko-agent --lib translate::hermes::tests::repair_json_keeps_commas_inside_strings"
+++

`repair_json` (`translate/hermes.rs:258`) walks characters and drops every `,` followed by whitespace and `}` / `]` without tracking whether it is inside a string literal.
It runs after a failed first parse (e.g. a genuine trailing comma), so the repaired call can carry altered argument text such as `"a, ]"` -> `"a ]"`.
Fix: track string and escape state so only structural trailing commas are removed; add a test with a comma before a bracket inside a string.

## Notes

- 2026-10-01 (wk-tiers): implemented on work/bug-7567eb; cargo verification deferred to the batch check.
  `repair_json` tracks string and escape state, so it drops only structural trailing commas (`,` before `}`/`]`
  outside a string literal). A string value such as `"a, ]"` or `"say \"x, }\""` keeps its text. Test:
  `repair_json_keeps_commas_inside_strings`, directly and through `parse_calls`.
