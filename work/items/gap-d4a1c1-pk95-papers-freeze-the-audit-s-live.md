+++
id = "gap-d4a1c1"
kind = "gap"
title = "PK95 Papers: Freeze the audit's live-run findings (R3, R4) as whitepaper evidence (+10 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "whitepaper"
rank = 95
size = "L"
subsystem = ["paper"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK95"
anchors = ["docs/whitepaper/00-abstract.md", "docs/whitepaper/01-introduction.md", "docs/whitepaper/03-architecture.md", "docs/whitepaper/04-golden-path.md", "docs/whitepaper/05-cybernetic-mechanisms.md", "docs/whitepaper/06-measured-trust.md", "docs/whitepaper/08-evaluation-plan.md", "docs/whitepaper/09-status-and-roadmap.md", "docs/whitepaper/README.md", "docs/whitepaper/REVIEW.md", "docs/whitepaper/appendix-status-matrix.md", "docs/whitepaper/data/mechanisms.toml", "docs/whitepaper/evidence/README.md", "docs/whitepaper/evidence/SHA256SUMS", "docs/whitepaper/figures/fig1-architecture.svg", "docs/whitepaper/figures/fig2-golden-path.svg", "docs/whitepaper/figures/fig3-status-matrix.svg", "tmp/cybernetic-harness/paper/FIGURES-TABLES.md", "tmp/cybernetic-harness/paper/sections/00-abstract.md", "tmp/cybernetic-harness/paper/sections/01-introduction.md", "tmp/cybernetic-harness/paper/sections/02-background.md", "tmp/cybernetic-harness/paper/sections/04-system.md", "tmp/cybernetic-harness/paper/sections/08-discussion.md", "tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md"]
lane = "paper"
parent = "spec-6afe5c"
links = { depends_on = [], blocks = [], related = ["gap-08d9b2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/evidence/2026-10-02-live-cheap-model-run.md && test -f docs/whitepaper/evidence/2026-10-02-live-defect-root-causes.md && grep -q ' 2026-10-02-live-cheap-model-run.md$' docs/whitepaper/evidence/SHA256SUMS && grep -q ' 2026-10-02-live-defect-root-causes.md$' docs/whitepaper/evidence/SHA256SUMS && (cd docs/whitepaper/evidence && shasum -a 256 -c SHA256SUMS >/dev/null) && ! grep -rqE '/(Users|home|private)/' docs/whitepaper/evidence/"

[[verify]]
command = "python3 -c \"import tomllib,subprocess,sys;d=tomllib.load(open('docs/whitepaper/data/mechanisms.toml','rb'));p=d['matrix']['pinned'];r={x['id']:x for x in d['row']};a=dict(EX4='PARTIAL',IS5='PARTIAL',QA7='PARTIAL',RC1='PARTIAL',RC3='PARTIAL',RC4='PARTIAL',RC5='PARTIAL',SS3='BROKEN');new=subprocess.run(['git','merge-base','--is-ancestor','a43288b5f',p]).returncode==0;bad=[k for k,t in a.items() if r[k]['tag']!=t and 'a43288b5f' not in r[k]['note']];sys.exit(0 if new and not bad else 1)\" && python3 tools/status_matrix.py --check"

[[verify]]
command = "! grep -qE '(WIRED|PARTIAL|BROKEN|ORPHANED|MISSING|BUILT-UNWIRED|REMOVED|DOCS-ONLY|UNPROVEN)@41228d7b2' docs/whitepaper/[0-9]*.md docs/whitepaper/README.md && grep -q '2026-10-02-live-cheap-model-run.md' docs/whitepaper/09-status-and-roadmap.md && python3 tools/status_matrix.py --check && python3 tools/paperlint.py --strict --require-status reviewed docs/whitepaper/*.md"

[[verify]]
command = "grep -q '^## Re-read after the audit re-pin' docs/whitepaper/REVIEW.md && grep -q '^Verdict: accept' docs/whitepaper/REVIEW.md && python3 tools/paperlint.py --strict --require-status reviewed docs/whitepaper/*.md"

[[verify]]
command = "grep -q 'cut, masked, stale and unlogged' tmp/cybernetic-harness/paper/sections/00-abstract.md && grep -q 'cut, masked, stale and unlogged' tmp/cybernetic-harness/paper/sections/01-introduction.md && ! grep -q 'TODO: confirm both bars' tmp/cybernetic-harness/paper/sections/01-introduction.md && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "python3 -c \"import re,sys;a=open('tmp/cybernetic-harness/paper/sections/01-introduction.md').read();b=open('tmp/cybernetic-harness/paper/sections/08-discussion.md').read();bad=re.search(r'await\\s+the\\s+companion',a) or re.search(r'replay\\s+the\\s+15\\s*\\+\\s*2',a) or re.search(r'final\\s+dormant-loop\\s+count',b) or not re.search(r'17\\s+inert',a);sys.exit(1 if bad else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "python3 -c \"import re,sys;t=open('tmp/cybernetic-harness/paper/bibliography/BIB-QA.md').read();b=open('tmp/cybernetic-harness/paper/sections/02-background.md').read();sys.exit(0 if re.search(r'^Sample: 60 entries',t,re.M) and re.search(r'^Verdict: pass',t,re.M) and not re.search(r'TODO:\\s*page-level\\s+check',b) else 1)\""

[[verify]]
command = "python3 -c \"import re,sys;s=open('tmp/cybernetic-harness/paper/sections/04-system.md').read();d=open('tmp/cybernetic-harness/paper/sections/08-discussion.md').read();bad=re.search(r'TODO:\\s*(pre-register|per-EV|M1\\s+or\\s+M3)',s) or re.search(r'TODO:\\s*before\\s+the\\s+lock',d);sys.exit(1 if bad else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "python3 -c \"import re,sys;t=open('tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md').read();bad=re.search(r'TODO:\\s*(model IDs|agent count|models of the other|earlier periods|report the tokens|replace this estimate|cite the API)',t) or re.search(r'\\|\\s*\\[\\[TODO\\]\\]\\s*\\|',t) or ('CITE?' in t);sys.exit(1 if bad else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "test -x tmp/cybernetic-harness/paper/tools/build.sh && tmp/cybernetic-harness/paper/tools/build.sh --list companion | grep -q 'E10-DRAFT.md' && tmp/cybernetic-harness/paper/tools/build.sh --list paper | grep -q 'F-ai-assistance-ethics.md' && tmp/cybernetic-harness/paper/tools/build.sh companion --draft --out \"${TMPDIR:-/tmp}/w95-build-check\" && test -s \"${TMPDIR:-/tmp}/w95-build-check/companion.pdf\""

[[verify]]
command = "test -f tmp/cybernetic-harness/paper/figures/f1-control-stack.svg && python3 -c \"import re,sys;bad=[f for f in ('02-background.md','04-system.md') if re.search(r'\\[\\[(FIG F1|TAB T1)\\]\\]|TODO:\\s*align the F1',open('tmp/cybernetic-harness/paper/sections/'+f).read())];sys.exit(1 if bad else 0)\""
+++

## Problem

This package delivers 11 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK95, slice 95xx, phase W), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9501 | S | p1 | Freeze the audit's live-run findings (R3, R4) as whitepaper evidence | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9501-whitepaper-freeze-live-run-evidence.md` |
| 2 | 9502 | M | p1 | Whitepaper matrix: re-pin at a43288b5f from the audit's re-check, lowering the eight rows the live run contradicts | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9502-whitepaper-matrix-repin-audit-lower-rows.md` |
| 3 | 9503 | M | p1 | Whitepaper sections: re-pin every status tag to the new matrix pin and rewrite what rests on the lowered rows | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9503-whitepaper-sections-repin-and-lowered-rows.md` |
| 4 | 9504 | S | p2 | Whitepaper: an independent re-read of the sections the audit re-pin changed | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9504-whitepaper-reread-after-audit-repin.md` |
| 5 | 9505 | S | p2 | Research paper abstract and §1.5: state H4 and H7 as §5 and S09 do | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9505-paper-abstract-intro-hypotheses-match-protocol.md` |
| 6 | 9506 | S | p2 | Research paper §1 and §8: give the loop counts §7.3 gives, and drop the stale loop TODOs | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9506-paper-loop-counts-follow-section-7.md` |
| 7 | 9507 | S | p2 | Research paper bibliography: re-verify a 60-entry sample, and check §2's Beer wording against the text | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9507-paper-bibliography-qa-sample-and-beer-check.md` |
| 8 | 9508 | S | p2 | Research paper §4.8 and §8: settle the pre-registration TODOs against S09 v1.4 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9508-paper-system-closures-prereg-todos.md` |
| 9 | 9510 | S | p2 | Research paper Appendix F: fill model ids, agent counts and session costs from the transcript harvest, and cite the provider terms | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9510-paper-ai-assistance-appendix-harvest-and-terms.md` |
| 10 | 9511 | M | p2 | Build script: a PDF and an arXiv source bundle for the research paper and the companion report | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9511-paper-and-companion-build-script.md` |
| 11 | 9512 | S | p2 | Research paper Figure 1 and Table 1: draw the control stack and resolve their markers in §2 and §4 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9512-paper-figure-1-and-table-1.md` |

## Why it matters

Track W: the papers (alongside, after their inputs). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9500-writing-papers-and-whitepaper-corrections.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `docs/whitepaper/00-abstract.md`, `docs/whitepaper/01-introduction.md`, `docs/whitepaper/03-architecture.md`, `docs/whitepaper/04-golden-path.md`, `docs/whitepaper/05-cybernetic-mechanisms.md`, `docs/whitepaper/06-measured-trust.md`, `docs/whitepaper/08-evaluation-plan.md`, `docs/whitepaper/09-status-and-roadmap.md`, `docs/whitepaper/README.md`, `docs/whitepaper/REVIEW.md`, `docs/whitepaper/appendix-status-matrix.md`, `docs/whitepaper/data/mechanisms.toml`, `docs/whitepaper/evidence/2026-10-02-live-cheap-model-run.md`, `docs/whitepaper/evidence/2026-10-02-live-defect-root-causes.md`, `docs/whitepaper/evidence/README.md`, `docs/whitepaper/evidence/SHA256SUMS`, `docs/whitepaper/figures/fig1-architecture.svg`, `docs/whitepaper/figures/fig2-golden-path.svg`, `docs/whitepaper/figures/fig3-status-matrix.svg`, `tmp/cybernetic-harness/paper/FIGURES-TABLES.md`, `tmp/cybernetic-harness/paper/bibliography/BIB-QA.md`, `tmp/cybernetic-harness/paper/bibliography/bib-qa-sample.jsonl`, `tmp/cybernetic-harness/paper/figures/f1-control-stack.svg`, `tmp/cybernetic-harness/paper/figures/f1-control-stack.txt`, `tmp/cybernetic-harness/paper/sections/00-abstract.md`, `tmp/cybernetic-harness/paper/sections/01-introduction.md`, `tmp/cybernetic-harness/paper/sections/02-background.md`, `tmp/cybernetic-harness/paper/sections/04-system.md`, `tmp/cybernetic-harness/paper/sections/08-discussion.md`, `tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md`, `tmp/cybernetic-harness/paper/tools/build.sh`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: nothing.
- Existing work items this package covers or touches: gap-08d9b2. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.

## Progress

- 9501: implemented at ac644b6d7
- 9502: implemented at 9d059cc72
- 9503: implemented at 29996ca33 (sections) and ec12c7513 (README)
- 9504: blocked: the re-read must come from a reader other than 9503's author; this agent wrote 9503 and may not run sub-agents
- 9505: implemented in place in the main checkout (tmp/ is untracked, so no commit)
- 9506: implemented in place (untracked)
- 9507: implemented in place (untracked): 0 major and 2 minor errors in 60 entries, a pass; fixing the two (via `bib-overrides.json`) and adding a secondary source for §2's Beer wording (via `new-refs`) need files outside the task
- 9508: implemented in place (untracked)
- 9510: blocked: `work/telemetry/ROLLUP.md` (2026-10-01) has 0 harvested call rows and `work/telemetry/harvest/` does not exist, so the model ids, agent counts, tokens and costs per activity cannot come from the harvest; the provider-term citations need bib keys through `new-refs`, outside the task's files
- 9511: implemented in place (untracked): `tmp/cybernetic-harness/paper/tools/build.sh`
- 9512: implemented in place (untracked): `tmp/cybernetic-harness/paper/figures/f1-control-stack.svg` and `.txt`
- Wave 3 (2026-10-02), 9504: re-read by another agent (w3-pk95), recorded at 88a97ed04 under `## Re-read after the audit re-pin` in `docs/whitepaper/REVIEW.md`, with `Verdict: revise`. Findings 29–31 must change before the tag: the 2026-10-02 run is called the first live run on cheap models, though B7 and the field rollup record earlier gpt-oss-120b runs (it is the ladder's first); "needed six operator interventions", where the run counts four against unattended running; and the GLM blank answers are blamed on the stream parser, which R4 leaves open (§3.1 and row RC1). Findings 32–37 should change in the same pass, 38–39 wait for gap-08d9b2's final re-pin, and 40 (7,755 words, 1.19× the budget) is the author's call. Strict paperlint passes. The verify still exits 0, but only because the first review's `Verdict: accept` line matches its grep, so it cannot show this verdict.
