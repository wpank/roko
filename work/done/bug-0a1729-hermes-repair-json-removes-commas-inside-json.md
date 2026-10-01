+++
id = "bug-0a1729"
kind = "bug"
title = "Hermes repair_json removes commas inside JSON string values"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-agent/translate"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/translate/hermes.rs::repair_json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn repair_json_keeps_commas_inside_strings' crates/roko-agent/src/translate/hermes.rs && cargo test -p roko-agent --lib translate::hermes::tests::repair_json_keeps_commas_inside_strings"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:05Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T17:37:10Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`repair_json` (`translate/hermes.rs:258`) walks characters and drops every `,` followed by whitespace and `}` / `]` without tracking whether it is inside a string literal.
It runs after a failed first parse (e.g. a genuine trailing comma), so the repaired call can carry altered argument text such as `"a, ]"` -> `"a ]"`.
Fix: track string and escape state so only structural trailing commas are removed; add a test with a comma before a bracket inside a string.

## Notes

- 2026-10-01 (wk-tiers): implemented on work/bug-7567eb; cargo verification deferred to the batch check.
  `repair_json` tracks string and escape state, so it drops only structural trailing commas (`,` before `}`/`]`
  outside a string literal). A string value such as `"a, ]"` or `"say \"x, }\""` keeps its text. Test:
  `repair_json_keeps_commas_inside_strings`, directly and through `parse_calls`.
