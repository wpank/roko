Status: stub · budget 550 words · owner gap-2aad7d

# 8 Evaluation plan

[[TODO: Write this section to gap-2aad7d's plan, in about 550 words. Waits for: gap-d9e9fe, for pilot numbers only. It contains "cost per verified task", and states the arms, metrics and falsifiers, with the scope of each arm. Keep line 1 and set it to `draft`, then replace everything below the heading, this list of claims included. Conventions and canonical numbers: `README.md` in this directory.]]

## Claims

| Id | Kind | Claim | Writer's source | Cite in the text as |
|---|---|---|---|---|
| EV1 | design | The claim under test is equal quality at a lower cost per verified task. The cost counts the planner and verification at API list price; subscription cash is reported separately. | tldr/05 decision 9; PLAN §1 | spec-567e52 (E12) |
| EV2 | design | The arms: first the pilot (a cheap model alone, a cheap model in Roko, Claude Code on Opus 5.5; 20 hidden-test tasks, 3 seeds, at most $15), then the full comparison. Say which arm answers which question. | PLAN §3 E12; tldr/04 "How to prove it"; S09 | spec-567e52 (E12); dec-b78874 |
| EV3 | design | The metrics, stratified by task type: cost per verified task, verified success, pass^k over seeds, false greens found by hidden tests, and wall-clock time. | tldr/04; draft §5.4 | spec-567e52 (E12) |
| EV4 | design | The safeguards: hidden tests, an isolated Claude Code config, the executed model checked on every attempt, and the secret kept in a driver-only file. | PLAN §3 E12 rows 10–12 | The E12 items |
| EV5 | design | The falsifiers: which results would count against the thesis. | draft §5; S09 | spec-567e52 (E12) |
| EV6 | scope | Scope: single tasks plus a small plan-level slice; the pilot itself measures single tasks only. | PLAN §1 ("Evaluation scope"); W9 PW05 | gap-89f393; gap-1cd676 |
| EV7 | number | Pilot numbers appear only if `vb report --pilot` has run; otherwise the section gives none. | gap-d9e9fe | A frozen report with its sha256 |
| EV8 | status | What must hold first: honest verdicts end to end (E2) and the tier ladder (E5). The Python driver replaces `roko bench`, whose gold-patch leak is bug-28becc. | tldr/04; PLAN §3 E12 | spec-e9d7ec (E2); spec-98f76d (E5); bug-28becc |

## Sources

Writer inputs; the paths under `tmp/` are gitignored and are never cited in the text.

- PLAN: `tmp/cybernetic-harness/workstreams/PLAN.md`. The author's answers (section 1) and the epics (section 3).
- tldr/04: `tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md`. The 11-step loop, the eight design rules, the real-run numbers, the three-arm test.
- tldr/05: `tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md`. Scorecard V1-V10, proposals P0-P3, parking, doc corrections, decisions.
- draft §N: `tmp/cybernetic-harness/paper/sections/`. The research draft's sections; outline in `paper/OUTLINE.md`, conventions in `paper/00-README.md`.
- S01–S11: `tmp/cybernetic-harness/specs/`. Programme specs; cite the tracked epics that carry them.
- W9: `tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md`. Rules for honest ideal-state writing; the phantom identifiers.
