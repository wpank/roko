+++
id = "gap-785a4f"
kind = "gap"
title = "harness-engineering.md builds on six harness principles credited to Meta-Harness, which the paper's abstract doesn't name"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["docs/v3"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "014e73289"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's report)"
anchors = ["docs/v3/depth/05-agent/harness-engineering.md", "tools/docs_integrity/citation_errata.json"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-212b75", "gap-b23ebd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qi 'six harness principles' docs/v3/depth/05-agent/harness-engineering.md || grep -iE 'six harness principles' docs/v3/depth/05-agent/harness-engineering.md | grep -qE '§|[Ss]ection [0-9]'"

[closed]
at = 2026-10-01
commit = "014e73289"
evidence = "full paper read (arXiv HTML): no principles; harness-engineering.md §1 rewritten with section refs, §2 marked Roko's own synthesis; attribution removed in 11 v3/v1 files; Meta-Harness prose phrases added to citation_errata.json; verify passes; checker --prose clean on 905 files; 13 tests pass"
+++

## Problem

`docs/v3/depth/05-agent/harness-engineering.md` builds its section on "six harness principles" credited to Meta-Harness (Lee et al., 2026, "Meta-Harness: End-to-End Optimization of Model Harnesses"; :3, :9, :23). The paper's abstract describes an outer-loop search over harness code and names no principles (wk-rp-cite). Whether the principles come from the paper needs a reading of the full text.

## Why it matters

Release: if the principles aren't the paper's, the docs credit it with claims it doesn't make, which is the kind of error gap-212b75 removed elsewhere. p3.

## Plan

1. Read the full paper. If it states the principles, cite the section. If not, reattribute them (to roko's own synthesis) or rewrite the section.
2. `citation_errata.json` already has a Meta-Harness entry, and `--prose` passes, so it doesn't catch this claim. Add the adjudication to the entry, so the checker enforces it from now on.

## Done when

- [ ] The section attributes only what the paper says.
- [ ] The `[[verify]]` command passes: the phrase is gone, or each mention cites the paper's section.

## Notes

- 2026-10-01 (wk-rp-cite): read the full paper (arXiv HTML of 2603.28052). It states no harness principles. Its
  contribution is an outer-loop search over harness code, in which a coding-agent proposer reads every earlier
  candidate's code, scores and traces (§3). The results (§4) are +7.7 points over ACE with 4x fewer context tokens,
  +4.7 points on 200 IMO-level problems across five held-out models, and #1 among Haiku 4.5 agents on TerminalBench-2.
  Appendix D gives procedural tips for running the search, and calls them engineering lessons, not scientific claims.
  The "6x gap" is a result the paper cites (SWE-bench Mobile). It says the harness "often matters as much as the model
  itself", not more. SWE-bench is not one of its benchmarks.
- `harness-engineering.md` §1 now says what the paper shows, with section references, and §2 presents the principles as
  Roko's own synthesis. §9 drops the unsupported "25% or 85% on SWE-bench" claim. Three citation titles in §10 are
  corrected from their registry records.
- The same attribution ("Meta-Harness principle 1/5/6", "six harness principles", "dominant performance factor") is also
  removed from `05-AGENT.md`, the `depth/05-agent` format-translation, temperament-profiling and agent-roles docs, and
  their docs/v1 counterparts.
- `citation_errata.json` now gives Meta-Harness `prose` phrases, so `--prose` flags any section that credits principles
  to it again (checked with a planted regression). Belief Divergence and the mechanism-level review get records for
  their invented titles.
