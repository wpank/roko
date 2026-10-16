+++
id = "bug-bef9c6"
kind = "bug"
title = "The research paper's PDF drops the 11.2 portal figure: build.sh strips every blockquote right under a heading, so 11.2 starts with \"The figure counts…\" and no figure"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["papers/research-paper"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "5e32264ce"
source = "status-quo-2026-10-15#finding-32"
anchors = ["tmp/cybernetic-harness/paper/tools/build.sh", "tmp/cybernetic-harness/paper/sections/11-in-use.md"]
lane = "paper"
links = { depends_on = [], blocks = [], related = ["gap-ac1aed", "spec-f8d196"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tmp/cybernetic-harness/paper/tools/clean_body.py tmp/cybernetic-harness/paper/sections/11-in-use.md | grep -q 'Illustration (recorded figure)'"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T14:07:12Z"
commit = "5e32264ce"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T08:06:59Z"
forced = true
evidence = "Fixed tmp/cybernetic-harness/paper/tools/build.sh (untracked, lives only in the main checkout at /Users/will/dev/nunchi/roko/roko/tmp/cybernetic-harness/paper/tools/): moved its inline blockquote-stripping step into new tools/clean_body.py (also untracked), which now drops a blockquote right under a heading only when it opens with '> **Drafting note' -- any other blockquote there, including the 11.2 recorded-figure illustration, stays. build.sh's cleaning step now imports and calls clean_body.clean_files(). Added tools/test_clean_body.py (17 unittest cases, all passing) covering the kept-illustration / dropped-drafting-note split plus every pre-existing cleaning rule. Archived the pre-fix build.sh at tmp/cybernetic-harness/archive/2026-10-16-build-fix/tools/build.sh. Verified from the main checkout (repo root), where these untracked files live: 'python3 tmp/cybernetic-harness/paper/tools/clean_body.py tmp/cybernetic-harness/paper/sections/11-in-use.md | grep -q \"Illustration (recorded figure)\"' exits 0. Diffed clean_body.py's output against the prior inline logic across all 20 paper/sections/*.md files: byte-identical except the 4 restored figure lines in 11-in-use.md. --force because tools/work.py's own static verify check runs with cwd=this worktree, which never has the untracked paper tree; the command only resolves from the main checkout, as the item's Notes section expects."
+++

## Problem

`tmp/cybernetic-harness/paper/tools/build.sh` cleans each section before pandoc sees it. One rule drops "a
blockquote note right under a heading" (`build.sh:15`; the Python step at `:140-168` skips every `>` line while
`under_heading` is set). In `sections/11-in-use.md`, §11.2 "The portal build" opens with the paper's one recorded
figure as a blockquote: "> **Illustration (recorded figure).** Roko ran most of the build of its own web portal …
16 plans and 173 tasks, 168 of them gate-verified, for $174.87 … Frontier sessions (Claude Code) wrote those
plans …". The build drops it, so the PDF's §11.2 starts "The figure counts what Roko recorded…" with no figure, and
the "frontier sessions wrote those plans" caveat that FRAMING §7.28 requires is gone too. A scan of all 20 section
files finds no other blockquote right under a heading, so this is the only casualty today. Status-quo assessment
2026-10-15 at 7b6a5af44, finding 32 (`tmp/status-quo/2026-10-15/papers/F-paper-vs-paper.md` R1).

## Why it matters

Goal `whitepaper`: the PDF is what reviewers and readers see, and it silently differs from the Markdown. The
independent review (gap-ac1aed, Will's decision 4) should review a PDF that matches its source.

## Where

- `tmp/cybernetic-harness/paper/tools/build.sh`: the inline Python cleaning step (about lines 125-180).
- `tmp/cybernetic-harness/paper/sections/11-in-use.md` lines 19-23.

## Current state

The rule exists to drop drafting notes placed under headings. No test covers the cleaning step.

## Plan

1. Move the inline cleaning step into `tools/clean_body.py` (importable, and runnable as
   `clean_body.py <section.md>…`, which prints the cleaned Markdown), and have `build.sh` call it.
2. Drop only blockquotes that are drafting notes (for example ones starting `> **Note`, `> Status`, `> TODO` or a
   `[[…]]` marker), not a quoted illustration: keep any blockquote that starts with `> **Illustration`, or switch
   the rule to an explicit marker such as `> **Drafting note.**`.
3. Add `tools/test_clean_body.py` with a recorded-figure blockquote under a heading (kept) and a drafting note (dropped).
4. Rebuild the PDF and check §11.2.

## Done when

The `[[verify]]` command passes, and the rebuilt PDF shows the §11.2 figure.

## Notes

The paper folder is untracked (no history; finding 31's item). Don't change the section text to work around the
build. `docs/whitepaper/build.sh` has no such rule (checked 2026-10-15).
