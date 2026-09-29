Status: reviewed · budget 500 words · owner gap-ec516e

# 10 Related work

**Harness engineering.** Recent work optimizes the harness around a frozen model [@lee2026meta]. Self-improving
harnesses converge on one guard: test each edit on held-out and anchor tasks, and commit only what passes, with
rollback [@kang2026harness; @tayebati2026self; @xia2026rrsi]. The guard matters: agents predict their own edits'
regressions poorly [@lin2026agentic], and harness gains compounded only with regression control in the loop
[@wang2026compound] (§5.4). Roko is designed to adopt the guard (§5, spec-6ac537), not the promise of compounding
gains.

**Coding agents and planner–executor products.** Open agents defined the tool-using loop [@yang2024sweagent;
@wang2025openhands], multi-agent systems split plans across roles [@hong2024metagpt; @fourney2024magentic], and a fixed
localize–repair–validate pipeline remains a cheap, strong baseline [@xia2025agentless]. Products pair a frontier model
with a cheaper one: Claude Code's `opusplan` alias plans with Opus and executes with Sonnet [@anthropic2026modelconfig],
Devin Fusion pairs "a frontier lead model with a cost-efficient sidekick" [@cognition2026models], and aider's architect
mode has one model describe a solution and another edit the files [@aider2024architect]. Kiro runs a spec's independent
tasks concurrently, in dependency waves [@kiro2026specs]. At a fixed model, a controlled study found no average
difference between vendor and neutral harnesses [@arjmandi2026harness]; §8 plans to measure Roko's harness effect rather
than assume it (spec-567e52).

**Routing and cascades.** Cascades and learned routers cut cost at matched quality on single-turn queries
[@chen2024frugalgpt; @ong2025routellm], when the quality estimate is accurate [@dekoninck2024unified]. For agentic
coding the evidence is mixed: on 266 SWE-bench Pro tasks, a router matched the best single model at about a fifth of its
cost per solve, but a no-router ablation tied it [@bhola2026scrouting]. Every such policy is capped by how often all
candidate models fail together [@chen2026when]. Roko claims no routing algorithm.

**Verification, gaming and audits.** Augmented tests exposed 345 wrongly passing patches on SWE-bench leaderboards
[@yu2025utboost], agents edit or special-case the tests that grade them [@zhong2025impossiblebench], and on
long-horizon coding tasks the gap between visible and held-out tests is larger for smaller models
[@zhao2026specbench]. Model-written test oracles tend to encode actual rather than expected behaviour
[@konstantinou2024do]. Cheap verifiers have blind spots: on hard MATH queries a cascade's dashboard read a flat 3%
error while delivered error reached 32% [@rajput2026cheap]. Ranking audits by self-reported confidence does worse
than random past a miscalibration threshold [@zavattari2026one], so Roko's deep audits are designed to sample passed
work by lottery (spec-6ac537).

**Cybernetics for agents.** Ashby's essential variables [@ashby1960design] and control theory for self-adaptive
software [@filieri2015software] supply a regulator's vocabulary and tests. *Agent Cybernetics*, a position paper
without implementation or experiments, is the closest framing to ours [@wang2026agent]; Paul treats context assembly
as the controlled variable of a harness policy [@paul2026context].

**Where Roko sits.** We claim no firstness. As of 2026-09-29, no product or paper we reviewed combines routing learned
from a team's own verified outcomes across vendors, an audited false-green rate, and per-loop evidence that learning
changes decisions. The closest partial matches are a production triage agent that estimates its auto-passed items' miss
rate from audit samples [@gao2026pinsieve], a harness that measures its gates' false-accept rate on planted mutants
[@bhardwaj2026bounded], routing learned across vendors from benchmark outcomes [@li2026rsirouter], and offline audits of
whether stored experience changes decisions [@xu2026replaylens]. Roko is designed to provide all three (spec-6ac537);
none works end to end today.
