+++
id = "gap-af0b57"
kind = "gap"
title = "paperlint: a whitepaper check that fails on unsupported claims"
status = "open"
triage = "unverified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["tools/paperlint"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md (F3; PW01)"
anchors = ["tools/paperlint.py", "tools/test_paperlint.py"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["gap-0191eb", "gap-35a614", "gap-8d2c79"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_strict_fails_on_each_rule' tools/test_paperlint.py && python3 tools/test_paperlint.py -k test_strict_fails_on_each_rule"

[[verify]]
command = "grep -qw 'def test_strict_passes_clean_section' tools/test_paperlint.py && python3 tools/test_paperlint.py -k test_strict_passes_clean_section"
+++

## Problem

Nothing checks that the whitepaper's ideal-state prose stays within what the code and the evidence support. The
research draft has the same gap: its marker grammar (`paper/00-README.md`) is a convention with no lint (W9 F3).

## Why it matters

Unchecked ideal-state writing repeats docs/v3's pattern (W9, risk 2). Every section's verify, the review (gap-8d2c79)
and the epic's exit check run this tool.

## Where

- **New:**
  - `tools/paperlint.py`, standard library only;
  - `tools/test_paperlint.py`, in `unittest` style like `tools/test_work.py`.
- **Reuse:** `tools/docs_integrity/check_markdown_links.py` for local links and figure paths. Don't write a second
  link checker.

## Current state

Checked at `41c7ffbd6`: no paperlint exists, in `tools/` or in the programme folder. W9's eight phantom identifiers
(`AttemptKey`, `LoopSpec`, `HarnessParams`, `audit_select`, `cost_source`, `AuditRecord`, `DecisionRecord`,
`ExposureRecord`) are still missing from `crates/`. gap-528762 (E4.1) will add `AttemptKey`, so the tests must use a
fixture repository, not the real tree.

## Plan

1. **Command line:** `paperlint.py [--strict] [--report] [--require-status S] PATH…`. A directory path means its
   `NN-*.md` and `appendix-*.md` files.
2. **`--strict` fails on:**
   1. a leftover `[[…]]` marker outside code. Skip TOML tables such as `[[task.verify]]`; W9 found seven collisions.
   2. a `[@key]` that is not in `docs/whitepaper/references.bib`.
   3. an identifier or repo path in backticks that `git grep -w` or `git ls-files` can't find at HEAD, unless its
      sentence or table row carries `MISSING@…` or `(designed)`.
   4. a status tag without `@<commit>`, or whose commit is not an ancestor of HEAD.
   5. a `$` amount, a percentage, an `N×` ratio or an "N of M" count in a paragraph with no footnote that names a
      commit, snapshot, rollup key, `[@key]` or work item.
   6. a missing status header, `Status: stub`, or a word count outside 0.5–1.3× the header's budget.
   7. a banned word from the README's conventions.
   8. a broken local link or figure path.
3. **Other modes:**
   - `--report` prints counts per file and per rule, and exits 0.
   - `--require-status reviewed` is for the final pass.
4. **Tests** build a temporary git repository with a small `crates/` tree. Each rule gets one failing fixture, and one
   clean section must pass.

## Done when

- [ ] Each rule has a fixture that fails `--strict`, and the clean fixture passes.
- [ ] A stub from gap-0191eb fails `--strict`, and `--report` exits 0.
- [ ] Both `[[verify]]` commands pass.

## Notes

- **Needs** only Python 3 and `git`, with no network access. Lane `paper`; no hot files.
- **Rules 3 and 4 check against HEAD,** so a later rename in `crates/` fails the lint. That is intended: the text must
  then be refreshed.
- The research draft can adopt the tool later. W9 PW01's claims aggregator is out of scope.
- **Decided 2026-09-29:** the tests use the standard library's `unittest`, and `tools/test_paperlint.py` ends with `unittest.main()`. pytest is not installed for the system `python3`.
