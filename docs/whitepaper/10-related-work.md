Status: draft · budget 500 words · owner gap-ec516e

# 10 Related work

This survey covers the literature as of 2026-09-29 and vendor documentation fetched that day.

**Harness engineering.** Recent work optimizes the harness around a frozen model directly [@lee2026meta]. The 2026
papers on self-improving harnesses converge on one guard: test each edit on held-out and anchor tasks, and commit only
what passes, with rollback [@kang2026harness; @tayebati2026self; @xia2026rrsi]. The guard matters: agents predict the
regressions of their own edits poorly [@lin2026agentic], and on Terminal-Bench gains compounded only with regression
control in the loop; without it, one optimized agent fell below its unoptimized baseline [@wang2026compound]. Harness
evolution also does not consistently beat test-time scaling on matched budgets [@wang2026rethinking]. Roko is designed
to adopt the guard (§5, spec-6ac537), not the promise of compounding gains.

**Coding agents and planner–executor products.** Open agents defined the tool-using loop [@yang2024sweagent;
@wang2025openhands], multi-agent systems split plans across roles [@hong2024metagpt; @fourney2024magentic], and a
fixed localize–repair–validate pipeline remains a cheap, strong baseline [@xia2025agentless]. Several products pair a
frontier model with a cheaper one: Claude Code's opusplan alias plans with Opus and executes with Sonnet
[@anthropic2026modelconfig], Devin Fusion pairs "a frontier lead model with a cost-efficient sidekick"
[@cognition2026fusion], and aider's architect mode has one model describe a solution and another edit the files
[@aider2024architect]. Kiro turns a spec into tasks and runs independent ones concurrently, in dependency waves
[@kiro2026specs]. At a fixed model, a paired, contamination-controlled study found no average difference between a
vendor's own harness and a neutral one [@arjmandi2026harness]; §8 plans to measure Roko's harness effect rather than
assume it (spec-567e52).

**Routing and cascades.** Cascades and learned routers cut cost at matched quality on single-turn queries
[@chen2024frugalgpt; @ong2025routellm], when the quality estimate is accurate [@dekoninck2024unified]. For agentic
coding the evidence is thinner and mixed. On 266 Python tasks from SWE-bench Pro, a router matched the best single
model at about a fifth of its cost per solve, but a no-router ablation tied the router [@bhola2026scrouting]. Every
such policy is also capped by how often all candidate models fail together [@chen2026when]. Roko claims no routing
algorithm.

**Verification, gaming and audits.** Augmented tests exposed 345 wrongly passing patches on the SWE-bench leaderboards
[@yu2025utboost], agents edit or special-case the tests that grade them [@zhong2025impossiblebench], and on
long-horizon coding tasks the gap between visible and held-out tests is larger for smaller models
[@zhao2026specbench]. Model-written test oracles tend to encode the code's actual behaviour, not the expected one
[@konstantinou2024do]. Cheap verifiers have blind spots: in a cascade on hard MATH queries, the dashboard read a flat
3% error while delivered error rose as high as 32% [@rajput2026cheap]. Ranking audits by self-reported confidence does
worse than random past a miscalibration threshold [@zavattari2026one], so Roko's deep audits are designed to sample
passed work by lottery (spec-6ac537).

**Cybernetics for agents.** Ashby's essential variables [@ashby1960design] and control theory for self-adaptive
software [@filieri2015software] supply the vocabulary and the tests of a regulator. *Agent Cybernetics*, a position
paper without implementation or experiments, is the closest framing to ours [@wang2026agent]; Paul treats context
assembly as the controlled variable of a harness policy [@paul2026context].

**Where Roko sits.** We claim no firstness. As documented on 2026-09-29, no product or paper we reviewed combines
routing learned from a team's own verified outcomes across vendors, an audited false-green rate, and per-loop evidence
that learning changes decisions. Roko is designed to provide all three (spec-6ac537); none works end to end today.
