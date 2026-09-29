Status: stub · budget 550 words · owner gap-370d3c

# 2 Design principles

[[TODO: Write this section to gap-370d3c's plan, in about 550 words. Each of the eight rules cites at least one verified source, so the section has at least 8 `[@` citations. Keep line 1 and set it to `draft`, then replace everything below the heading, this list of claims included. Conventions and canonical numbers: `README.md` in this directory.]]

## Claims

| Id | Kind | Claim | Writer's source | Cite in the text as |
|---|---|---|---|---|
| DP1 | lit | Size tasks for the executor, not for cohesion: models' 80%-success horizons are 4–6× shorter than their 50% horizons, and per-step errors compound. | tldr/04 rule 1; C1 | [@kwa2025measuring]; [@sinha2025illusion] |
| DP2 | lit | Split on independent outputs, not sequential steps: parallel agents helped decomposable work (+80.8%) and hurt sequential work (−70%). | tldr/04 rule 2; C1 | [@kim2025towards] |
| DP3 | lit | The planner writes the gating checks and the implementer never does: model-written oracles tend to encode the code's actual behaviour, not the expected one. | tldr/04 rule 3; C2 | [@konstantinou2024do] |
| DP4 | lit | Assume visible checks will be gamed, most of all by cheap models: the gap between visible and held-out tests is larger for smaller models. | tldr/04 rule 4; C2 | [@zhao2026specbench] |
| DP5 | lit | Retry twice cheaply with the distilled gate errors (never the failed transcript), then escalate one rung, then split or replan. | tldr/04 rule 5; C1 | [@sinha2025illusion]; [@prasad2024adapt] |
| DP6 | lit | Merge, then verify, through a queue: in one study 16% of merges conflicted, and a further 7% merged cleanly but broke the build or tests. | tldr/04 rule 6; C1 | [@brun2011proactive] |
| DP7 | lit | Put ambiguity back into authoring: models don't notice underspecification, and a check before execution recovers most of the loss. | tldr/04 rule 7; C2 | [@vijayvargiya2025ambigswe]; [@edwards2026askorassume] |
| DP8 | lit | Count cost per verified task, including verification, retries, escalations and the planner: a cheap verifier's blind spots can cancel the savings. | tldr/04 rule 8; C1; C2 | [@rajput2026cheap] |
| DP9 | lit | The cybernetic vocabulary: essential variables (verified pass rate, cost per verified task, false-green rate, latency), regulation, and audits of the regulators. *Agent Cybernetics* is the closest framing; no claim of firstness. | tldr/02 "What cybernetic means here"; draft §2.2 and §2.7 | `wang2026agent` and `ashby1960design` (both in the draft's `references.bib`; §2 adds them after refcheck) |
| DP10 | scope | Say where each result was measured (single function, QA, documents or repository scale), so that no single-function result reads as repository-scale evidence. | C1 "Size of the effects"; tldr/01 "The bet" | A hedge beside each [@key] |

## Sources

Writer inputs; the paths under `tmp/` are gitignored and are never cited in the text.

- tldr/02: `tmp/cybernetic-harness/tldr/02-HOW-IT-WORKS.md`. Architecture, the control stack, what `.roko/` records.
- tldr/04: `tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md`. The 11-step loop, the eight design rules, the real-run numbers, the three-arm test.
- C1: `tmp/cybernetic-harness/tldr/research/C1-research-planning-decomposition-cascades.md`. Literature: planning, decomposition, cascades, integration.
- C2: `tmp/cybernetic-harness/tldr/research/C2-research-specs-acceptance-verification.md`. Literature: specs, acceptance, verification.
- draft §N: `tmp/cybernetic-harness/paper/sections/`. The research draft's sections; outline in `paper/OUTLINE.md`, conventions in `paper/00-README.md`.
- 01-THESIS: `tmp/cybernetic-harness/01-THESIS.md`. Section 6: safe and avoided claims.
