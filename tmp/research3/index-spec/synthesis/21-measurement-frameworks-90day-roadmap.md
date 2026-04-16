# Measurement Frameworks, Proof Points, Risk Register, and 90-Day Execution Roadmap

This document specifies how to measure whether a new agent coordination protocol delivers on its claims, what demonstrations will convince skeptical investors, what can go wrong in the first 90 days, and what must ship when. It is written from scratch for someone with zero prior context. Every metric, benchmark, citation, and timeline is explained as if the reader has never encountered any of the underlying research.

---

## 1. Four Protocol Claims and Their Measurement

Any new agent protocol must answer four questions that every technical buyer and investor will ask: Is it cheaper? Is it composable? Do agent groups become smarter than their parts? Can it improve itself? Each claim requires a specific, reproducible measurement that outsiders can verify.

### Claim 1: 10-30x Cost Reduction

The assertion is that this protocol executes agent tasks at 10-30x lower cost than incumbent frameworks (LangGraph, CrewAI, bare ReAct loops) at equal or better quality.

**Why the claim is hard to prove.** AI inference costs are falling fast. Epoch AI's "Algorithmic Efficiency and Falling Cost of AI Inference" (arXiv:2511.23455, March 2026) documents 10-1000x per year price drops at constant performance across frontier model families. Any cost reduction claim must beat this background trend -- a protocol that is 10x cheaper today but whose advantage shrinks to 1x when DeepSeek's next model drops is not a durable wedge.

**The canonical benchmark suite.** Princeton's HAL (Holistic Agent Leaderboard) maintains Pareto-frontier leaderboards that jointly evaluate success rate versus token cost on three agent benchmarks: GAIA (general-purpose multi-step reasoning), SWE-bench Verified (real GitHub issue resolution), and AppWorld (API-driven application tasks). Kapoor, Stroebl, and Narayanan formalized this joint Pareto evaluation methodology in "AI Agents That Matter" (2024), establishing that reporting success rate without cost, or cost without success rate, is scientifically meaningless. A system that solves 90% of tasks at $44.86 per task and another that solves 85% at $1.42 per task are not directly comparable on either axis alone -- the Pareto frontier is the only honest comparison.

Princeton HAL's data already shows the gap: naive agent execution on their benchmark suite averages $44.86 per task, while optimized execution averages $1.42 per task -- a 31x differential. This establishes the empirical ceiling for cost-reduction claims.

**New metric: CPCA (Cost Per Correct Answer).** Existing benchmarks report success rate and total cost separately. CPCA unifies them:

    CPCA = total_dollars / (n_tasks * pass_at_k)

Where `total_dollars` is the all-in cost of running the agent (inference, tool calls, retries, orchestration overhead), `n_tasks` is the number of attempted tasks, and `pass@k` is the fraction of tasks solved correctly within k attempts. CPCA has units of dollars-per-correct-answer. Lower is better. The denominator penalizes both low success rates and high attempt counts. A system that spends $100 to solve 10 out of 100 tasks at pass@1 has CPCA = $100 / (100 * 0.10) = $10.00. A system that spends $20 to solve 50 out of 100 tasks has CPCA = $20 / (100 * 0.50) = $0.40.

**The decisive demo.** A live three-bar comparison on GAIA or tau-squared-bench, running the same model (model held constant is the single most credible empirical move), across three orchestrations: bare ReAct loop, LangGraph DAG, and the protocol under evaluation. Each bar shows two axes: success rate (height) and cumulative cost (real-time dollar counter updating as tasks complete). Holding the model constant isolates the protocol's contribution from the model's capability. The audience watches costs accumulate in real time.

One additional slider toggle labeled "cache primitives" demonstrates the composability cost lever: enabling caching of previously computed sub-results drops the cost bar by another 2x. This pairs with the Cline/Uber anecdote (Uber engineering reported $18 per task with Cline's caching versus $720 without -- a 40x spread on identical tasks) to make the cost story visceral.

**Model-held-constant discipline.** Cost comparisons across different models are confounded by model pricing. Claude Opus 4.6 at $15/M input tokens and GPT-4o at $2.50/M input tokens produce incomparable CPCA numbers. The protocol must demonstrate cost reduction at a single model, then show the reduction holds across models as a robustness check.

### Claim 2: Composability

The assertion is that a small number of protocol primitives compose into a combinatorially large number of valid workflows, with minimal glue code.

**No canonical benchmark exists.** Unlike cost (Princeton HAL) or self-improvement (METR), composability has no established measurement. The protocol must invent and publish three metrics:

**T2NW (Time to New Workflow).** A cohort comparison: give N developers a task specification for a workflow they have not seen before and measure wall-clock time from "start" to "first passing integration test." Target: median under 5 minutes for the protocol versus over 2 hours for LangGraph. The study must control for developer experience and task complexity. A statistically significant difference with a paired t-test at p < 0.05 and a practically meaningful effect size (Cohen's d > 0.8) constitutes evidence.

**Composability Index (CI).** The ratio of unique valid workflows expressible per protocol primitive. If 5 primitives generate 47 valid pipelines, CI = 47/5 = 9.4. If 10 primitives generate 523 valid pipelines, CI = 52.3. The claim is super-linear growth: CI should increase as primitives are added, not remain constant. Plotted on a log-log scale, the slope should exceed 1.0. A slope of exactly 1.0 means linear growth (each primitive adds a constant number of new workflows). A slope below 1.0 means diminishing returns. Super-linear growth means each new primitive multiplies the workflow space rather than merely adding to it.

**Valid-combination coverage.** For N primitives, there are N-choose-k possible k-element combinations. What fraction type-check and pass a smoke test? If 5 primitives yield C(5,3) = 10 possible 3-element combinations and 9 of them type-check and pass, coverage = 90%. The target is >80% coverage at k=3 and >50% at k=5. Coverage below 50% suggests primitives are not actually composable -- they have hidden incompatibilities that require case-by-case debugging.

**Marketing-grade evidence template.** Stripe's API documentation reports that "the median customer composes 4.7 primitives" in their payment integration. This is not a benchmark number -- it is a usage metric that demonstrates real-world composability. The protocol should track and publish equivalent usage metrics once design partners are onboarded: median primitives per workflow, 90th percentile composition depth, and unique workflow count per organization.

### Claim 3: c-factor (Collective Intelligence)

The assertion is that groups of agents running the protocol exhibit collective intelligence -- the group is measurably smarter than the sum of its individual members.

**The Woolley protocol.** Anita Williams Woolley's 2010 paper "Evidence for a Collective Intelligence Factor in the Performance of Human Groups" (Science, Vol 330) established the gold standard. The experimental design: assemble groups of varying size (2-10 members), administer a heterogeneous battery of tasks spanning different cognitive domains (verbal reasoning, spatial reasoning, negotiation, brainstorming, visual pattern matching), and factor-analyze the group performance scores. If a single principal component (the "c-factor") explains more than 30% of variance across tasks, the group exhibits collective intelligence -- performance on one task type predicts performance on unrelated task types, indicating a latent group-level capability that transcends any individual task.

**Replication with agents.** Run the Woolley protocol with agent groups instead of human groups. Replace verbal reasoning with code generation, spatial reasoning with ARC-AGI pattern completion, negotiation with multi-party resource allocation, brainstorming with divergent solution generation, and visual pattern matching with GUI navigation. Vary group size from 2 to 10 agents. Factor-analyze. If the first principal component explains >30% variance, agent groups exhibit a c-factor.

**The killer demo.** Form two groups of 5 agents: one using the protocol's coordination primitives, one using bare message passing. Administer the same 6-domain task battery. Show that adding one agent running the protocol to the bare-message-passing group raises the group's c-factor by 0.3 standard deviations. This demonstrates that the protocol contributes intelligence beyond model IQ -- the protocol itself is the differentiator, not the underlying model.

**Pair with a 100-agent run.** The Project Sid experiment (Altera.AI, arXiv:2411.00114, November 2024) placed 1,000 agents in a persistent Minecraft world and observed emergent role specialization, governance, and economic structures. A 100-agent run on a structured task (not Minecraft but a software engineering or data analysis project) demonstrates coordination at scale. Dochkina (MIPT, arXiv:2603.28990, March 2026) established that quality plateaus at 64 agents -- the demo should show that protocol-mediated coordination maintains quality above the plateau via hierarchical sharding (64-agent clusters with inter-cluster protocols).

### Claim 4: Self-Improvement

The assertion is that agents running the protocol get better at their tasks over time without human intervention, using a frozen foundation model.

**METR Time Horizon 1.1 (January 2026).** METR (Model Evaluation & Threat Research) maintains the gold-standard benchmark for measuring how long an AI agent can sustain coherent, goal-directed work. Their metric is the "50% time horizon" -- the maximum task duration at which a model achieves at least 50% success rate. The January 2026 release established the frontier: Claude Opus 4.5 at approximately 2 hours 17 minutes, o3 at approximately 110 minutes, GPT-5 at approximately 137 minutes. The post-2023 doubling time is 130.8 days (approximately 89 days restricted to the 2024-2025 window). Current Opus 4.6 is estimated at a 14.5-hour 50% time horizon based on extrapolation from the 89-day doubling rate.

**Sakana DGM (Darwinian Godel Machine).** Zhang, Hu, Lu, Lange, and Clune (arXiv:2505.22954) demonstrated a self-improving agent that evolves its own scaffolding code. Starting from a 20% baseline on SWE-bench Verified, DGM reached 50% through iterative self-modification -- with the foundation model frozen. The agent rewrites its own tool definitions, prompt templates, and retry logic. The foundation model's weights never change. The improvement comes entirely from the scaffolding layer.

**Per-protocol self-improvement curve.** Lock the LLM. Plot generations (x-axis) against pass rate on a held-out task set (y-axis). The curve should show monotonic improvement with diminishing returns (a log curve, not a plateau). The Huxley-Godel Machine (metauto-ai, arXiv:2510.21614) demonstrated that evaluating self-improvement by the aggregate performance of the entire descendant tree (clade-metaproductivity) beats greedy single-generation evaluation. Thompson sampling over variant lineages preserves exploration of promising but currently underperforming lineages.

A critical constraint from Meta FAIR's SPICE framework (arXiv:2510.24684): pure ungrounded self-play collapses. Mutual information between successive generations decreases monotonically without external grounding. The self-improvement curve must include external data injection (new task distributions, updated benchmarks, fresh code repositories) to prevent information-symmetry collapse.

**The 90-minute demo.** A compressed recording showing a protocol-based agent improving from 25% to 55% on a held-out SWE-bench-mini subset over 20 self-modification iterations. Each iteration's diff is readable -- the audience sees the agent rewriting its own tool definitions, adding error-handling patterns it discovered in previous iterations, and compressing multi-step procedures into single function calls. Closing frame: "DGM with the foundation model frozen. The protocol is the substrate for self-improvement."

### Five KPIs for Slide Three

These are the five numbers that belong on the third slide of any pitch deck or technical presentation:

1. **CPCA** -- the headline 10-30x cost number, denominated in dollars-per-correct-answer, with model held constant.
2. **T2NW** -- the composability proxy, measured in minutes, with a paired comparison against the leading incumbent.
3. **50% time horizon at fixed model** -- the METR-style self-sufficiency metric, showing how long a protocol-based agent can work autonomously.
4. **Self-improvement slope** -- delta-accuracy per iteration on held-out tasks with a frozen foundation model. Units: percentage points per generation.
5. **Pairwise win rate versus incumbent** -- head-to-head comparison on the same task set, with a paired-difference confidence interval (95% CI). A win rate of 60% with CI [52%, 68%] is statistically significant. A win rate of 55% with CI [47%, 63%] is not.

### A/B Testing Methodology

**Paired-difference paradigm.** Anthropic's "Adding Error Bars to Evals" methodology computes paired differences: for each task, subtract the incumbent's score from the protocol's score, then bootstrap a 95% confidence interval over the paired differences. This eliminates task-level variance (some tasks are harder than others) and isolates the protocol effect. The bootstrap uses 10,000 resamples with bias-corrected and accelerated (BCa) intervals.

**AgentAssay (arXiv:2603.02601).** AgentAssay introduces three-valued evaluation: PASS, FAIL, and INCONCLUSIVE. Traditional binary evaluation (pass/fail) forces a decision on ambiguous cases, inflating both false positives and false negatives. The INCONCLUSIVE category captures cases where the evaluator cannot determine correctness with confidence. AgentAssay reports 78-100% cost reduction in regression testing by skipping tasks whose outcomes are predetermined (the agent always passes or always fails) and focusing evaluation resources on the uncertain middle.

**Shadow A/B.** In production, every request is asynchronously routed through both the current version (v_old) and the candidate version (v_new). The user sees v_old's response. v_new's response is logged for offline comparison. Shadow A/B accumulates paired differences without user-facing risk. When the paired-difference CI excludes zero with sufficient effect size, the candidate is promoted.

---

## 2. The "Prove It" List -- Demos That Convince Skeptical VCs

Each demo targets a specific claim and is designed to be run live in a meeting, not presented as a recorded video.

### 10x Cost Demo

**Setup.** Three terminal windows side by side. Same model (Claude Opus 4.6 or equivalent). Same task set (20 GAIA tasks or 20 tau-squared-bench tasks). Three orchestrations: bare ReAct loop (left), LangGraph DAG (center), protocol under evaluation (right).

**What the audience sees.** All three start simultaneously. A real-time token meter and dollar counter update above each terminal. Tasks complete at different rates. The bare ReAct loop accumulates $40-50. LangGraph accumulates $15-20. The protocol accumulates $2-4.

**The slider.** After the first run completes, toggle a single setting labeled "cache primitives." Re-run 5 tasks. The protocol's cost bar drops another 2x because previously computed sub-results (tool outputs, intermediate reasoning chains, verified sub-conclusions) are retrieved from cache rather than recomputed. The audience sees the cost drop happen in real time from a single configuration change.

**Pair with anecdote.** Uber engineering reported that Cline with caching cost $18 per coding task versus $720 without caching -- a 40x spread on identical tasks. The demo makes this concrete.

**Methodological credibility.** Reference Princeton HAL's Pareto-frontier methodology explicitly. Show the Pareto chart with all three systems plotted. The protocol should be on or near the Pareto frontier (high success rate at low cost). Systems that are dominated (lower success rate AND higher cost than another system) are eliminated from consideration.

### Composability Demo

**Setup.** Two-pane terminal. Left pane: a code editor showing 5 primitive definitions in 7 lines of configuration. Right pane: a pipeline runner.

**What the audience sees.** The presenter picks any 3 of the 5 primitives. Types a one-line composition command. The right pane shows the 3 primitives auto-composing into a runnable workflow -- type-checking, wiring inputs to outputs, and executing a smoke test. Total time: under 10 seconds.

**The combinatorial explosion.** The presenter runs a script that enumerates all valid 3-element combinations of the 5 primitives. Result: 47 valid pipelines with zero additional code. Each pipeline type-checks (the type system guarantees well-formedness) and passes a smoke test (produces output on a trivial input).

**The LangGraph diff.** Side-by-side: rebuilding the same 47 pipelines in LangGraph requires approximately 5,400 lines of glue code (state definitions, edge functions, conditional routing, error handling, retry logic). The diff is projected on screen. The point is not that LangGraph is bad -- it is that framework-level glue code is a tax that compounds with every new workflow.

### c-factor Demo

**Setup.** "Agent Wechsler test" -- named after the Wechsler Adult Intelligence Scale, which measures human IQ across multiple cognitive domains.

**What the audience sees.** 5 anonymous agent groups (identities hidden to prevent brand-bias). 6 task domains presented in sequence: code generation, pattern completion, resource allocation, divergent ideation, document analysis, and multi-step planning. Each group works on each domain for 5 minutes. After 30 minutes, the factor loading is computed and displayed: a single principal component explaining >30% variance across domains indicates collective intelligence.

**The protocol effect.** One of the 5 groups is a mixed group: 4 agents using bare coordination plus 1 agent running the protocol. After the factor analysis, the mixed group's c-factor is 0.3 standard deviations higher than the all-bare groups. The protocol agent's individual performance is not the highest -- but its contribution to group intelligence is disproportionate. The protocol is not a better model; it is a better coordination substrate.

### Self-Improvement Demo

**Setup.** A 90-minute compressed recording (presented at 4x speed in the meeting, 22.5 minutes wall clock).

**What the audience sees.** Iteration 1: the agent attempts 20 SWE-bench-mini tasks, solves 5 (25%). The agent examines its failures, identifies 3 patterns (missing error handling, incorrect API usage, incomplete test coverage), and rewrites its tool definitions to address each pattern. The diffs are shown -- readable, concrete, small.

**Iterations 2-20.** Each iteration: attempt the same 20 tasks with the updated scaffolding, identify new failure patterns, rewrite tools. Pass rate climbs: 25% to 30% to 35% ... to 55% by iteration 20. Each diff is shown briefly. The audience sees the agent learning from its mistakes in a way that is human-readable and auditable.

**Closing frame.** "DGM with the foundation model frozen. The protocol is the substrate." The Sakana DGM reference (arXiv:2505.22954) is cited on-screen: SWE-bench 20% to 50% via scaffolding self-modification with a frozen foundation model. The protocol achieves 55% by adding persistent tool libraries indexed by HDC fingerprints -- tools synthesized in iteration 5 are retrieved and reused in iteration 15 rather than reinvented.

---

## 3. Risk Register -- Top 10 Risks

Risks are ranked by the product of probability and impact. Probability is estimated as high (>60%), moderate (30-60%), or low (<30%). Impact is estimated as catastrophic (existential threat), severe (6+ month setback), or moderate (1-3 month setback).

### Risk 1: MCP/A2A/x402 Closes the Window Before Cross-Lab Endorsement

**P: High. I: Catastrophic.**

MCP (Model Context Protocol) reached 97 million monthly SDK downloads by March 2026 in 16 months from announcement. Google's A2A (Agent-to-Agent) protocol gained 150+ supporting organizations. Coinbase's x402 is processing an estimated 63 million transactions per month. These standards are crystallizing. Once they lock in, the coordination layer above them becomes the next land-grab. If MCP ships a native composition layer, A2A adds type-safe workflows, or x402 expands to cover agent coordination payments, the protocol's window closes before it can establish cross-lab endorsement.

**Mitigation.** Compose, do not compete. Ship as an MCP-server adapter and A2A AgentCard extension on day one. The protocol is a layer above MCP/A2A, not a replacement. Secure 20+ named launch partners who publicly commit before launch. Target partners across the MCP ecosystem (tool server providers), A2A ecosystem (enterprise agent builders), and x402 ecosystem (payment integrators). The protocol becomes the composition layer that MCP/A2A/x402 need but do not provide.

### Risk 2: EU AI Act August 2 Enforcement Blocks EU Procurement

**P: Moderate-High. I: Severe.**

The EU AI Act's Article 50 transparency requirements and Annex III high-risk system classification take enforcement effect on August 2, 2026. Agent systems that lack mandatory transparency documentation, risk assessments, and human oversight mechanisms will be blocked from EU procurement. The Digital Omnibus proposal (released February 2026) may soften requirements for certain AI systems, but the regulatory trajectory is toward stricter requirements, not weaker ones. Any protocol targeting enterprise customers with EU operations must be compliant by enforcement date.

**Mitigation.** Ship Article 50 conformity hooks and Annex III risk-assessment template integration by July 2026 -- one month before enforcement. Track the Digital Omnibus proposal weekly for changes that affect classification. Partner with a EU AI Act compliance firm (e.g., Holistic AI, Credo AI) for conformity assessment documentation. Publish a compliance guide for protocol users that maps protocol capabilities to AI Act requirements.

### Risk 3: Replit-Style or GTG-1002-Style Incident in First 90 Days

**P: Moderate. I: Catastrophic.**

Replit's agent deployment incidents and the GTG-1002 autonomous agent incident (where an agent took unauthorized real-world actions) demonstrated that a single high-profile failure can set back an entire category. If the protocol's first production deployment causes data loss, unauthorized actions, or a security breach, the reputational damage is existential. The MCP CVE record illustrates the attack surface: CVE-2025-6514 (mcp-remote, CVSS 9.6), CVE-2025-54136 (MCPoison in Cursor), the Postmark-mcp backdoor affecting 300 organizations, and Anthropic's own mcp-server-git RCE chain (CVE-2025-68143/68144/68145).

**Mitigation.** Default-deny for all destructive grants. No agent gets write access to production databases, file systems outside sandboxed directories, or external APIs with side effects unless explicitly granted by a human operator. Scoped credentials with time-limited tokens. Approval gates for high-impact actions (financial transactions above a threshold, code merges to protected branches, external communications). Principal-versus-delegate separation: every agent action carries both the delegating principal's identity and the agent's identity, enabling audit and attribution. The CIK (Cognitive Immune Kernel) threat model -- a 5-layer defense pipeline processing every trust-boundary crossing -- is the architectural backstop.

### Risk 4: Price Collapse Renders Cost Wedge Unmarketable

**P: Moderate. I: Severe.**

DeepSeek's trajectory demonstrates that inference costs can drop 10-100x in a single model generation. If the background cost-reduction trend (documented at 10-1000x per year in arXiv:2511.23455) accelerates, a protocol whose primary value proposition is "10x cheaper" becomes indistinguishable from "wait 6 months and costs drop anyway." The cost wedge is necessary but not sufficient.

**Mitigation.** Reframe as "10x at fixed quality budget" -- the protocol delivers 10x more work per dollar, not 10x fewer dollars per task. Use model-held-constant Pareto charts that isolate the protocol's contribution from model pricing trends. Emphasize composability and self-improvement as the durable wedges that do not erode with model cost deflation. Cost reduction gets attention; composability and self-improvement close deals.

### Risk 5: Multi-Agent Token Burn Creates Pricing Crisis

**P: High. I: Moderate.**

Anthropic's internal data suggests multi-agent configurations consume approximately 15x the tokens of single-agent configurations for equivalent tasks. Kim et al. (DeepMind, arXiv:2512.08296) measured 17.2x error amplification in independent multi-agent systems. Customers who scale from 1 to 10 agents expect 10x throughput but discover 15x cost. The sticker shock creates churn before the protocol's coordination benefits can manifest.

**Mitigation.** Hop-count and token-budget primitives as first-class protocol features. Every multi-agent workflow declares a maximum hop count (number of inter-agent message exchanges) and a total token budget. When either limit is reached, the workflow degrades gracefully to single-agent execution rather than burning tokens on diminishing-returns coordination. Default to single-agent execution -- multi-agent is opt-in, not default. Expose a real-time cost meter in every SDK and dashboard so customers see costs as they accumulate, not after the fact.

### Risk 6: OpenAI Symphony or Hyperscaler Announcement in 90 Days

**P: Moderate. I: Severe.**

OpenAI, Google, or Microsoft could announce a native multi-agent coordination framework (rumors of "OpenAI Symphony" have circulated) that bundles with their model API at zero marginal cost. A hyperscaler announcement collapses the protocol's market position from "novel infrastructure" to "third-party alternative to a free bundled feature."

**Mitigation.** Launch fast. Differentiate on three axes that hyperscalers cannot match: composability (type-safe composition with formal guarantees, not Python glue code), security (default-deny with scoped credentials and principal-delegate separation, not ambient authority), and self-improvement (protocol-level scaffolding evolution with frozen foundation models, not model-specific fine-tuning). The Bittensor playbook applies: decentralized protocols survive hyperscaler competition by offering properties (permissionlessness, composability, portability) that centralized platforms structurally cannot.

### Risk 7: Anthropic Rate-Limit Cuts During Public Demo

**P: Moderate. I: Moderate.**

A live demo that depends on real-time API calls to a single provider is a single point of failure. Anthropic rate-limit changes, temporary outages, or latency spikes during a public demo or VC meeting would be embarrassing and potentially deal-breaking.

**Mitigation.** Multi-provider routing with circuit breakers. The protocol's cascade router maintains connections to multiple model providers (Anthropic, OpenAI, Google, local Ollama). When one provider's latency exceeds a threshold or returns rate-limit errors, the router fails over to the next provider within 2 seconds. For high-stakes demos, maintain a cache of pre-computed demo responses that can be displayed if all providers fail simultaneously. The cache is clearly labeled as cached if used.

### Risk 8: NeurIPS 2026 May 6 Deadline Missed

**P: Moderate. I: Moderate.**

NeurIPS 2026 abstract submission deadline is May 6, 2026. Missing this deadline means the protocol's academic credibility paper cannot appear at the premier ML venue in 2026. Academic credibility matters for enterprise sales (CTO's check citations), hiring (researchers want to publish), and partnership conversations (labs want to cite peer-reviewed work).

**Mitigation.** Dedicated 2-person team on the paper from Day 1. Paper scope is narrowed to a single claim (composability via categorical foundations or cost reduction via protocol-level caching) rather than attempting to cover all four claims. Backup venues with later deadlines: AAMAS 2027 (multi-agent systems, deadline typically October), OSDI 2027 (systems, deadline typically April 2027). A companion arXiv preprint published on launch day provides immediate citability regardless of conference acceptance.

### Risk 9: YC Dark Horse With Stronger Partners

**P: Moderate. I: Moderate.**

Y Combinator's W2026 and S2026 batches will include agent infrastructure startups. A well-connected YC team with stronger launch partners (e.g., an Anthropic engineer as co-founder, or a Google DeepMind researcher as advisor) could announce a competing protocol with more credible endorsements.

**Mitigation.** Composability moat with MCP/A2A integration. A YC startup can match on brand and partners, but retrofitting categorical composition (parametric lenses, polynomial-functor protocols, DPO type-preserving rewrites) onto an empirical framework requires an architectural rebuild. The talent moat compounds: the intersection of HDC, category theory, sheaf mathematics, and corrigibility verification contains fewer than 50 active researchers worldwide (Kanerva and Rahimi for HDC at Stanford/Berkeley; Gavranovic, Lessard, Velickovic for categorical foundations funded via Symbolica's $31M raise; Hansen and Ghrist at UPenn for sheaf mathematics; Nayebi at Harvard for corrigibility). Competitors are not yet recruiting from these pools.

### Risk 10: Enterprise Requires SOC 2 + ISO 42001 Before Deployment

**P: High. I: Moderate.**

Enterprise procurement at any Fortune 500 company requires SOC 2 Type II certification (12-month observation period minimum) and increasingly ISO 42001 (AI management system standard). Without these certifications, the protocol cannot pass enterprise security review, regardless of its technical merits. The 12-month SOC 2 timeline means certification cannot be achieved in the first 90 days.

**Mitigation.** Start SOC 2 Type II audit on Day 30 with a firm like Vanta, Drata, or Secureframe that specializes in fast-track startup compliance. The audit observation period begins immediately. In the interim, provide an alignment letter from the audit firm confirming the audit is in progress plus a controls matrix mapping protocol security features to SOC 2 requirements. Partner with a cyber liability insurance provider (e.g., Coalition, Corvus) to offer design partners liability coverage during the pre-certification period. For ISO 42001, begin a readiness assessment in parallel -- the standard is newer and fewer enterprises require it today, but early movers gain credibility.

---

## 4. The 90-Day Execution Roadmap

### Days 0-30: Launch Foundation

The goal of the first 30 days is to establish the protocol as a credible, usable, open standard with enough surface area for developers to build on and enough institutional backing to attract design partners.

**Core specification (under 50 pages).** The protocol spec must be concise enough to read in an afternoon. Fifty pages maximum including formal definitions, wire formats, composition rules, and security model. Reference: the MCP specification is approximately 30 pages and achieved 97M monthly SDK downloads in 16 months. Brevity is a feature.

**SDKs: Python and TypeScript.** Two reference implementations covering the two largest agent developer populations. The Python SDK targets LangChain/CrewAI/DSPy users. The TypeScript SDK targets Vercel AI SDK, Mastra, and Claude Code users. Both SDKs must have a sub-10-line "hello agent" snippet that runs without signup, authentication, or infrastructure setup. The snippet should complete a visible task (summarize a document, answer a question, generate code) in under 30 seconds. Reference: Stripe's 7-line payment integration is the gold standard for developer onboarding.

**Five reference servers.** Working servers demonstrating the protocol's five primary use cases: research (multi-source information gathering), data extraction (structured output from unstructured sources), multi-agent coordination (task decomposition and parallel execution), RAG (retrieval-augmented generation with persistent knowledge), and browser automation (web interaction via headless browser). Each server is a Docker image that runs with `docker run` and no configuration.

**One reference host and Docker image.** A hosted sandbox where developers can try the protocol without installing anything. No signup required. Accessible via a web URL. The sandbox runs 5 pre-loaded example workflows and allows custom workflow composition via a web editor.

**Licensing.** Dual MIT/Apache 2.0 license. MIT for maximum permissiveness. Apache 2.0 for patent protection. The dual license is standard for open infrastructure protocols (Kubernetes, Terraform, and most CNCF projects use Apache 2.0).

**Community infrastructure.** Discord server with channels for announcements, help, showcase, and RFC discussion. GitHub Discussions for long-form technical conversation. A SEP (Specification Enhancement Proposal) process modeled on Python's PEP process: numbered proposals, shepherds, acceptance criteria, and a public decision record.

**Five starter templates.** Turnkey project templates for the five use cases (research, data extraction, multi-agent, RAG, browser automation). Each template is a `git clone && npm start` or `pip install && python main.py` experience.

**20+ named launch partners.** Public commitments from organizations across the agent ecosystem. Target mix: 5 tool-server providers (MCP ecosystem), 5 enterprise agent builders (A2A ecosystem), 5 developer-tools companies (IDE and CLI integrators), and 5 research groups (academic credibility). Named partners, not anonymous "we have interest from..." claims.

**NeurIPS 2026 paper by May 6 (binding deadline).** Abstract submitted by the deadline. Scope: a single claim with reproducible experiments. The categorical-foundations claim (compositional generalization as a theorem, not an empirical observation) is the strongest candidate because it is mathematically novel and empirically validatable on ARC-AGI-2.

**Companion arXiv preprint on launch day.** A longer paper covering all four claims, published on arXiv simultaneously with the protocol launch. This provides immediate citability for press, blog posts, and partner communications.

**MCP-server adapter and A2A AgentCard extension on day one.** The protocol ships as a layer above MCP and A2A, not a replacement. Any existing MCP server can be wrapped as a protocol primitive with a one-line adapter. Any A2A AgentCard can be extended with protocol metadata. This eliminates the "rip and replace" objection.

### Days 31-60: Prove Claims in Production

The goal of the second 30 days is to generate empirical evidence for all four claims using real production workloads from design partners.

**10 design-partner organizations.** Target organizations across three segments: developer tools (2-3), enterprise SaaS (3-4), and crypto/DeFi (3-4). Each design partner commits to running at least one production workflow on the protocol for 30 days and sharing anonymized performance data. In exchange, design partners get priority support, early access to new features, and co-marketing opportunities.

**Model-held-constant Pareto comparisons.** For each design partner's workflow, run the five-KPI measurement suite: CPCA, T2NW, 50% time horizon, self-improvement slope, and pairwise win rate. Publish Pareto charts comparing the protocol against each partner's existing orchestration (typically LangGraph, custom Python, or bare API calls). Model is held constant in all comparisons.

**Security-first wedge.** Enterprise design partners will not deploy without security assurance. Ship scoped credentials (time-limited, capability-restricted tokens), default-deny grant policies (no destructive actions without explicit human grant), approval gates (configurable human-in-the-loop for high-impact actions), OWASP ASI06 defenses (input validation, output sanitization, prompt injection detection per the OWASP Agentic Security Initiative's sixth priority), and principal-versus-delegate headers (every request carries both the human principal's identity and the agent's identity).

**Auto-import into developer tool templates.** Work with Lovable, Cursor, Bolt, v0, and Claude Code to include the protocol as a default option in their agent project templates. When a developer starts a new agent project in any of these tools, the protocol appears as a selectable orchestration option alongside LangGraph and CrewAI.

**Begin SOC 2 Type II audit.** Engage an audit firm and begin the 12-month observation period. The audit covers: access controls, encryption at rest and in transit, incident response procedures, change management, and vendor risk management. Early start means certification is achievable by Month 13.

**ISO 42001 readiness assessment.** Hire a consultant to perform a gap analysis against ISO 42001's AI management system requirements. Identify gaps and create a remediation plan. Full certification is a 6-12 month process; the readiness assessment establishes the roadmap.

**EU AI Act conformity-assessment documentation.** Publish a mapping document that shows how protocol features satisfy Article 50 transparency requirements (audit trails, decision logging, human oversight mechanisms) and Annex III risk assessment requirements (risk categorization, mitigation documentation, testing records). This document enables design partners with EU operations to proceed with deployment before enforcement.

**Pre-court cross-lab endorser.** Secure at least one statement of support from a frontier lab researcher (Anthropic, OpenAI, Google DeepMind, or Meta FAIR). Not a formal endorsement from the company -- a named individual researcher who publicly states the protocol addresses a real problem. This is the minimum credibility bar for enterprise sales conversations.

### Days 61-90: Trigger Inflection

The goal of the final 30 days is to trigger the inflection point -- the moment when adoption becomes self-reinforcing because the ecosystem has enough mass to attract new participants without active outreach.

**Land first frontier-tier endorsement.** A formal integration or public statement from Anthropic, OpenAI, or Google. This is the single highest-leverage event in the 90-day window. It converts the protocol from "interesting open-source project" to "industry infrastructure." The path: demonstrate protocol value to a frontier lab's developer relations team via a joint blog post, reference implementation, or conference talk.

**3 production case studies with Pareto charts.** Written case studies from design partners, each including: the business problem, the previous solution, the protocol-based solution, and a Pareto chart showing improvement on at least 3 of the 5 KPIs. Case studies are published on the protocol website, shared with press, and cited in sales conversations.

**Multi-agent governance primitives.** Ship the coordination primitives that enterprises need for production multi-agent deployments: deadlock detection (automatic identification of circular dependencies in agent communication graphs), hop-count budgets (maximum inter-agent message exchanges per workflow), trace TTL and provenance (every message carries a time-to-live and a provenance chain showing its origin), and model-provider mixing (a single workflow can use agents backed by different model providers without protocol-level incompatibilities).

**Public 100-agent demo.** A live demonstration of 100 agents coordinating on a structured software engineering task (e.g., implementing a feature across 10 microservices simultaneously). The demo shows the protocol's coordination primitives maintaining quality above the 64-agent plateau identified by Dochkina (arXiv:2603.28990) via hierarchical sharding. Real-time dashboards show token costs, task completion rates, and inter-agent communication patterns.

**Announce intent to donate to AAIF/LF within 12 months.** A public commitment to transfer protocol governance to a neutral foundation (AI Alliance, Linux Foundation, or Apache Software Foundation) within 12 months. This signals that the protocol is infrastructure, not a vendor lock-in play. The donation timeline gives the founding team 12 months to establish the spec and governance before transferring control.

**Second paper submission.** Submit to AAMAS (the premier multi-agent systems venue) or OSDI (the premier systems venue). Scope: the c-factor result or the multi-agent governance primitives. This establishes a publication cadence that maintains academic credibility.

**Day 90 targets.** 100+ external integrations (MCP servers wrapped as protocol primitives, A2A agents extended with protocol metadata, and standalone protocol implementations). 5,000+ GitHub stars. 1+ frontier-lab endorsement. 10+ active design partners with production workloads.

---

## 5. Dark Horses Worth Monitoring

These are organizations and projects that are not direct competitors today but could become competitive threats or acquisition targets within 6-12 months.

**Meta OpenEnv + TorchForge.** Meta's open-source evaluation infrastructure could commoditize the benchmarking layer that the protocol depends on for credibility. If Meta ships a turnkey agent evaluation suite that includes Pareto-frontier analysis, CPCA computation, and paired-difference testing, the protocol's measurement advantage disappears. The defense: measurement is necessary but not sufficient; the protocol's value is in the coordination primitives, not the metrics.

**Sakana ShinkaEvolve (ICLR 2026).** Sakana AI's evolutionary scaffold discovery system auto-generates agent scaffolding code. If ShinkaEvolve is integrated into Claude Code or OpenAI Codex as a default optimization pass, every agent framework gets self-improvement for free. The protocol's self-improvement claim becomes table stakes rather than a differentiator. The defense: ShinkaEvolve optimizes individual scaffolds; the protocol optimizes coordination across multiple agents, which is a higher-order problem.

**VERSES AXIOM.** VERSES Research published AXIOM (arXiv:2505.24784), a production-grade active inference implementation that achieves 60% better performance than DreamerV3 with a 400x smaller model (no neural network -- pure Bayesian updates on mixture models). If AXIOM ships as an SDK, any framework can add principled exploration-exploitation tradeoffs. The defense: the protocol integrates AXIOM-style active inference at three timescales (gamma/theta/delta), which is architecturally deeper than a bolt-on SDK.

**Tenstorrent Galaxy Blackhole.** Tenstorrent's open-source RISC-V AI accelerator targets 350+ tokens per second at consumer price points. If inference becomes effectively free, the cost-reduction claim loses its urgency. The defense: even at zero inference cost, orchestration overhead, tool-call costs, and multi-agent coordination costs remain. The protocol optimizes total workflow cost, not just inference cost.

**NeoCognition.** $40M seed round, Ion Stoica on the cap table (co-founder of Databricks, creator of Spark and Ray). Stoica's involvement signals serious infrastructure ambition. NeoCognition's specific technical direction is not yet public, but Stoica's track record is in distributed systems infrastructure that becomes industry standard. The defense: launch before NeoCognition announces; establish ecosystem momentum that makes NeoCognition a potential partner rather than competitor.

**Berkeley Sky Computing Lab (Stoica/Zaharia).** Ion Stoica and Matei Zaharia's research group has produced SkyRL (reinforcement learning at scale), Agentica (agent framework), NovaSky (distributed training), and the KISS framework (Keep It Simple, Stupid -- minimal agent coordination). The lab's philosophy of radical simplicity could produce a protocol that is less capable but dramatically easier to adopt. The defense: composability and self-improvement require architectural depth that radical simplicity trades away; the question is whether the market values depth or simplicity more.

**Chinese ecosystem as default open-weight base layer.** DeepSeek, Qwen (Alibaba), and other Chinese AI labs are releasing open-weight models at a pace that establishes them as the default base layer for the next 24 months. Any protocol that is model-agnostic benefits from this trend (more models means more routing options). Any protocol that is model-locked (e.g., dependent on Anthropic's API) is disadvantaged. The defense: the protocol is model-agnostic by design, with support for 7+ backends. Open-weight models from Chinese labs are first-class citizens.

---

## Summary of Key Citations

| Reference | Authors/Source | Venue/Date | Identifier |
|---|---|---|---|
| AI Agents That Matter | Kapoor, Stroebl, Narayanan | Princeton, 2024 | -- |
| Algorithmic Efficiency of AI Inference | Epoch AI | arXiv, Mar 2026 | arXiv:2511.23455 |
| Princeton HAL Pareto Leaderboard | Princeton NLP | Ongoing | hal.cs.princeton.edu |
| AgentAssay | -- | 2026 | arXiv:2603.02601 |
| METR Time Horizon 1.1 | METR | Jan 2026 | -- |
| Darwinian Godel Machine (DGM) | Zhang, Hu, Lu, Lange, Clune (Sakana) | 2025 | arXiv:2505.22954 |
| Huxley-Godel Machine (HGM) | metauto-ai | 2025 | arXiv:2510.21614 |
| SPICE (information-symmetry collapse) | Meta FAIR | 2025 | arXiv:2510.24684 |
| Live-SWE-agent | -- | 2025 | arXiv:2511.13646 |
| Woolley c-factor | Woolley et al. | Science, 2010 | Vol 330 |
| Dochkina 64-agent plateau | Dochkina, MIPT | 2026 | arXiv:2603.28990 |
| Kim et al. optimal comm density | Kim et al., DeepMind | 2025 | arXiv:2512.08296 |
| Project Sid | Altera.AI | 2024 | arXiv:2411.00114 |
| MAST failure catalog | Cemri et al. | NeurIPS 2025 | arXiv:2503.13657 |
| METR -19% productivity | METR | 2025 | arXiv:2507.09089 |
| SWE-bench contamination (SWE-ABS) | -- | 2026 | arXiv:2603.00520 |
| Sleeper Agents | Hubinger et al. | 2024 | arXiv:2401.05566 |
| Alignment Faking | Greenblatt et al. | 2024 | arXiv:2412.14093 |
| Emergent Misalignment | MacDiarmid et al., Anthropic | 2025 | arXiv:2511.18397 |
| Nayebi corrigibility | Nayebi | 2025 | arXiv:2507.20964 |
| CoT faithfulness limit | Anthropic | 2025 | arXiv:2505.05410 |
| Emergent collusion | Lin et al. | 2024 | arXiv:2410.00031 |
| AXIOM | Heins et al., VERSES | 2025 | arXiv:2505.24784 |
| Parametric Lenses | Cruttwell, Gavranovic | 2024 | arXiv:2404.00408 |
| Categorical Deep Learning | Gavranovic, Lessard, Velickovic | ICML 2024 | arXiv:2402.15332 |
| Kernel-Additivity Ceiling | Lippl, Stachenfeld | ICLR 2025 | arXiv:2405.16391 |
| Yang et al. heterogeneity bound | Yang et al. | ICML 2026 | arXiv:2602.03794 |
| Li et al. ring topology | Li et al., Google | EMNLP 2024 | arXiv:2406.11776 |
| MacNet scaling | Qian et al. | ICLR 2025 | arXiv:2406.07155 |
| TRM | Jolicoeur-Martineau et al. | 2025 | arXiv:2510.04871 |
| MCP CVE-2025-6514 | -- | 2025 | CVSS 9.6 |
| EU AI Act | European Parliament | Aug 2, 2026 enforcement | -- |
| Habermas Machine | Tessler, Bakker et al. | Science, 2024 | doi:10.1126/science.adq2852 |
| Agora protocols | Marro et al., Oxford | 2024 | arXiv:2410.11905 |
