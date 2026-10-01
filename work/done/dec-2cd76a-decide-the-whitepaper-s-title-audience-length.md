+++
id = "dec-2cd76a"
kind = "decision"
title = "Decide the whitepaper's title, audience, length and venue"
status = "done"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "70820a74c"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/workstreams/PLAN.md (§3 E1, item 1)"
anchors = ["docs/whitepaper/README.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["gap-0191eb", "gap-8117a8", "spec-ae5f94"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-29
by = "Will (decided 2026-09-29)"
evidence = "Will decided 2026-09-29. Title: 'Roko: a Cybernetic Harness for Spec'd Agent Work'. Audience: engineering leads deciding how to run agent work unattended. Length: 10-14 pages (about 6,500 words plus the status appendix). Venue: the repo plus a generated PDF (no arXiv for now). Citations: tracked files and commits only; quoted inputs frozen into docs/whitepaper/evidence/ with sha256. AI-assistance disclosure: none. Figures: install librsvg so pandoc can embed SVG figures."
+++

## Problem

Two parts were decided on 2026-09-29:
- **Placement:** the whitepaper lives in tracked `docs/whitepaper/`.
- **Thesis:** golden path plus cybernetics.

Six parts are still open, listed under Plan. The outline (gap-0191eb) and publishing (gap-8117a8) need the answers.

## Why it matters

Title and length set the scope of ten parallel section items, and the citation rule decides whether a public reader
can check the paper's sources.

## Where

`docs/whitepaper/README.md` (new; gap-0191eb creates it) records the answers in its title block.

## Current state

PLAN §3 E1 proposes 10–14 pages. The research draft's working title was flagged for stating its main result before
any data existed (`paper/OUTLINE.md`, review O5). The same risk applies here, because the cheap-model half of the
thesis is untested (tldr/04).

## Plan

Each part lists its options; the recommended default comes first.

1. **Title.** It must not state an unproven result.
   - *Frontier Models Plan, Cheap Models Execute: the Design and Status of Roko* (default).
   - *Roko: a Cybernetic Harness for Spec'd Agent Work*.
   - *Roko: Frontier Plans, Cheap Execution, Measured Trust*.
2. **Audience.**
   - Engineering leads deciding how to run agent work unattended (default). This follows tldr/01, "when to use Roko".
   - Reviewers of the application the release goal serves (`work/goals.toml`): the same text plus a one-page summary.
   - Researchers: the later research paper serves them better.
3. **Length.**
   - 10–14 pages: about 6,500 words plus the status appendix (default).
   - A 6-page short version.
   - More than 20 pages, which would duplicate the research paper.
4. **Venue.** Every public option needs the repo to be public (goal `release`).
   - The repo plus a PDF on a GitHub release at v1, then arXiv (cs.SE) at v1.1 once the pilot (E12) has numbers
     (default).
   - arXiv at v1.
   - The repo only.
5. **What the whitepaper may cite.** The tldr, specs, research notes and field evidence all sit in gitignored `tmp/`.
   - Cite only tracked files and commits, and freeze the few inputs the text quotes (the rollup, the cases) into
     `docs/whitepaper/evidence/` with their sha256 (default).
   - Publish the programme's specs and notes alongside the whitepaper.
   - Cite them as unpublished internal notes.
6. **AI-assistance disclosure.** Claude agents draft the sections.
   - A short note in the README and in §1 (default), like the research draft's App. F.
   - No disclosure.

## Done when

- [ ] The author has chosen each part, or accepted its default, and the choices are recorded in this item's
      `[closed]` evidence.
- [ ] `docs/whitepaper/README.md` states the title, audience, length and venue.

## Notes

- **A length change after drafting** means re-budgeting the status headers (gap-0191eb).
- **Anything copied out of `tmp/`** must be derived, scrubbed data with no transcripts, because agents can still read
  provider keys (bug-7d7200).
