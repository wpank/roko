+++
id = "gap-cb1a87"
kind = "gap"
title = "roko learn's four views (all, loops, self-model fit, probes list) in the run view's style: a header with a context line, labelled sections, §6.3 dates and money, glyphs, one Next, no absolute paths; each view's facts and --json kept"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/commands"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
source = "round v8 lead, wave 2 item 3 (the terminal output overhaul)"
discovered_from = "agent:roko-recovery (round v8)"
anchors = ["crates/roko-cli/src/commands/learn.rs", "crates/roko-cli/src/commands/learn_loops.rs", "crates/roko-cli/src/commands/learn_self_model.rs"]
links = { depends_on = [], blocks = [], related = ["gap-8752ce", "gap-2e6afb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_probes_read_like_roko' crates/roko-cli/src && grep -rqw 'fn learn_all_reads_like_roko' crates/roko-cli/src && cargo test -p roko-cli --lib -- learn_probes_read_like_roko learn_loops_leads_with_a_plain_summary && cargo test -p roko-cli --bin roko -- learn_all_reads_like_roko learn_self_model_fit_reports_insufficient_n"
+++

## Problem

roko's terminal output overhaul (round v8) gave the run view, status, plan writing, validate and diagnose one style:

- a `◆ roko` header with one context line;
- a label column;
- glyphs for states;
- dates, times and money in the design system's §6.3 forms;
- one Next command;
- no absolute paths.

The four `roko learn` views that show what roko has learned still print in an older, mixed style. roko-learnguard
fixed what they say in round v8 (gap-8752ce, bug-808700, gap-ea1576, gap-45f6bb, gap-2e6afb, bug-399240,
bug-c63eb1), but not how they look:

- **`learn all`** opens every section with an absolute `path:` line and repeats the path in its header line ("Cascade
  router: 25 observations, 5 models at /…/.roko/learn/cascade-router.json"). It gives full RFC 3339 times
  ("2026-10-28T11:09:22.325032+00:00") and mixes headings ("T0 Reflex Store — 0 rules", "Compounding (last 131
  episodes):").
- **`learn loops`** gives a plain summary line, then a bare 10-column table.
- **`learn self-model fit`** is a block of `key: value` lines under "S01 runs:" and "legacy logs:".
- **`learn probes list`** gives one line per model with "2026-10-20 20:33".

## Why it matters

Goal tooling. The roko guide's "how roko gets better" section rests on these views. They're the terminal's part of
round v8's polish, and they should read like the rest of roko.

## Where

- `crates/roko-cli/src/commands/learn.rs` (`learn all` and the probes list).
- `crates/roko-cli/src/commands/learn_loops.rs` and `crates/roko-learn/src/loop_audit/census.rs` (`learn loops`).
- `crates/roko-cli/src/commands/learn_self_model.rs` (`learn self-model fit`).

## Plan

1. Each of the four views gets the same frame:
   - the `◆ roko  learn <view>` header with a context line;
   - labelled sections;
   - §6.3 dates and money;
   - glyphs for states;
   - no absolute paths: the files a view read go in one dim closing line, relative to the workspace;
   - one Next where there is something to do.
2. Each view keeps its facts. Its `--json` doesn't change.
3. Tests for each view's frame and facts, with before and after captures on copies of Undertow's and roko's
   workspaces.

## Done when

The four views read in the run view's style with every fact they gave before, their JSON is unchanged, and the
`[[verify]]` passes.
