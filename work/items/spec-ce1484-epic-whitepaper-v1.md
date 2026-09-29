+++
id = "spec-ce1484"
kind = "spec"
title = "Epic: whitepaper v1"
status = "open"
triage = "unverified"
severity = "p1"
goal = "whitepaper"
size = "L"
subsystem = ["docs/whitepaper", "tools/paperlint"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/workstreams/PLAN.md (§1, the author's answers of 2026-09-29; §3 E1)"
anchors = ["docs/whitepaper/README.md", "tools/paperlint.py"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "paper"
links = { depends_on = ["dec-2cd76a", "gap-0191eb", "gap-af0b57", "gap-35a614", "gap-353d57", "gap-370d3c", "gap-4161ea", "gap-ac4646", "gap-e8cb4d", "gap-424bf8", "gap-29a64e", "gap-2aad7d", "gap-c19902", "gap-ec516e", "gap-d1d92c", "gap-8d2c79", "gap-8117a8"], blocks = [], related = ["spec-567e52", "spec-f2463d", "spec-ae5f94", "gap-cdf3fc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/paperlint.py && python3 tools/paperlint.py --strict --require-status reviewed docs/whitepaper && grep -q '^Verdict: accept' docs/whitepaper/REVIEW.md"
+++

## Problem

The author chose a whitepaper as Roko's first publication (PLAN §1, 2026-09-29). Its thesis is the golden path plus
cybernetics: frontier models plan, cheap models execute in parallel, gates and a whole-plan check verify, and feedback
loops make the system improve measurably. None of it exists yet: there is no `docs/whitepaper/`, no outline, and no
check that stops ideal-state prose from claiming what the code does not do.

## Why it matters

This is the `whitepaper` goal, second after the release blockers. The paper is written in the ideal-state voice, and
docs/v3 shows what that voice produces when nothing enforces it: claims the code or the literature contradicts
(tldr/05 §5) and phantom identifiers. W9 found that 8 of the 15 code identifiers named in the research draft's §4 are
not in `crates/`. So every mechanism carries a status tag at a commit, and every number traces to a snapshot, a rollup
or a commit.

## Where

These files are all new. Every child item uses this naming scheme:
- **`docs/whitepaper/`** (tracked):
  - `README.md`: the outline and conventions;
  - `00-abstract.md`;
  - `01-introduction.md`, `02-design-principles.md`, `03-architecture.md`, `04-golden-path.md`,
    `05-cybernetic-mechanisms.md`, `06-measured-trust.md`, `07-field-evidence.md`, `08-evaluation-plan.md`,
    `09-status-and-roadmap.md`, `10-related-work.md`;
  - `appendix-status-matrix.md`, `data/mechanisms.toml`, `figures/`, `evidence/`, `references.bib`, `REVIEW.md` and
    `build.sh`.
- **Tools:** `tools/paperlint.py` and `tools/status_matrix.py`, each with its tests.

## Current state

Checked at `41c7ffbd6`: neither `docs/whitepaper/` nor `tools/paperlint.py` exists. Every source is in gitignored
`tmp/cybernetic-harness/`:
- tldr/00–06 and its 20 research notes, written against `d9e79e9d8`;
- specs S01–S11;
- the field evidence: 42 snapshots, 121 notes and 8 cases;
- the research draft: 18 sections, about 70k words, with its marker grammar in `paper/00-README.md`.

**Figures differ between these documents:**
- false greens are 101 of 373 in B7 but 102 of 350 in the companion;
- the portal's spend is $174.87 in B7, and the rollup shows $192.92 over all 42 captured runs.

**Rows that have moved since `d9e79e9d8`:**
- the ready queue (`445a60d0d`, `3e7552acd`);
- adaptive retry budgets (`99adacd6d`, `41c7ffbd6`);
- cost recorded for timed-out attempts (`d4be4e872`).

## Plan

This is the implementation plan, in order.

1. **Settle the open parts** (dec-2cd76a): the title, audience, length and venue, and what may be cited publicly.
2. **Build the honesty check first** (gap-af0b57), in parallel with the outline and stubs (gap-0191eb). Every
   section's verify runs paperlint, so it must land before any section closes.
3. **Build the status matrix** (gap-35a614): every mechanism tagged at one commit. §4, §5 and §9 take their tags from
   it.
4. **Write the sections, one agent per file, 2–3 at a time:**
   - §1, §2, §3, §6, §8 and §10 need only the stubs;
   - §4, §5 and §9 need the matrix;
   - §7 waits for the rollup fix (bug-7b37c4, E13.1);
   - §8 includes pilot numbers only if E12 has reported.
5. **Figures** (gap-d1d92c), once §3 and §4 are written.
6. **Independent review and strict lint** (gap-8d2c79), then **publish** (gap-8117a8) once the repo is public.

## Done when

- [x] dec-2cd76a: Decide the whitepaper's title, audience, length and venue
- [x] gap-0191eb: Whitepaper outline and section stubs: the claims each section makes and their sources
- [ ] gap-af0b57: paperlint: a whitepaper check that fails on unsupported claims
- [ ] gap-35a614: Status matrix: every Roko mechanism with its status tag at a commit
- [ ] gap-353d57: Whitepaper §1 Introduction
- [ ] gap-370d3c: Whitepaper §2 Design principles
- [ ] gap-4161ea: Whitepaper §3 Architecture and control stack, with Figure 1
- [ ] gap-ac4646: Whitepaper §4 The golden path step by step, with Figure 2
- [ ] gap-e8cb4d: Whitepaper §5 Cybernetic mechanisms
- [ ] gap-424bf8: Whitepaper §6 Measured trust
- [ ] gap-29a64e: Whitepaper §7 Field evidence
- [ ] gap-2aad7d: Whitepaper §8 Evaluation plan
- [ ] gap-c19902: Whitepaper §9 Status, limitations and roadmap
- [ ] gap-ec516e: Whitepaper §10 Related work
- [ ] gap-d1d92c: Whitepaper figures: architecture, golden-path loop and status matrix
- [ ] gap-8d2c79: Whitepaper review: an independent read and a strict paperlint pass
- [ ] gap-8117a8: Publish the whitepaper: PDF build, release tag and venue
- [ ] The epic's `[[verify]]` command passes: strict paperlint over `docs/whitepaper/` with every section reviewed,
      and a review verdict of "accept".

## Notes

- **Lane `paper`, no Rust and no hot files.** Two files are shared, `README.md` and `references.bib`: edit them in
  small, separate commits.
- **Author decisions:**
  - dec-2cd76a;
  - whether the evaluation covers planning and integration (W9 PW05), which changes §8 (gap-2aad7d).
- **Waits on other epics:**
  - §7 on E13 (spec-f2463d);
  - §8's numbers on E12 (spec-567e52);
  - publishing on the release goal (spec-ae5f94).
- **Addition to PLAN's outline:** an abstract, `00-abstract.md`, written with §1.
- **Later:** the empirical research paper (`tmp/cybernetic-harness/paper/`) can reuse paperlint and the matrix.
- **Decided 2026-09-29 (Will):** title 'Roko: a Cybernetic Harness for Spec'd Agent Work'; audience engineering leads; 10-14 pages; the repo plus a generated PDF (no arXiv for now); cite tracked files and frozen evidence only; no AI-assistance disclosure; install librsvg for SVG figures. See dec-2cd76a.
