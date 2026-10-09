Status: draft · budget 550 words · owner spec-ce1484

# 10 Related work

This section places Roko among seven groups of related work. Its mechanisms all have precedents; what differs is how it
connects them around the executable verdict that every loop reads. Sources were accessed between 2026-09-28 and
2026-10-02.

## 10.1 Agent-building frameworks

AutoGen [@wu2024autogen] and its successor, the Microsoft Agent Framework [@microsoft2026agentframework], CrewAI
[@crewai2026docs], LangGraph [@campos2025building], the OpenAI Agents SDK [@openai2025agentssdk] and Google's ADK
[@google2026adk] supply parts for agent applications: conversations, graphs with checkpoints, guardrails on what an
agent returns. Roko shares their checkpoints, graph-shaped control and provider neutrality, but decides that
work is done by commands written before the work and run on the artifact, not by a guardrail on the agent's
answer.

## 10.2 Agent loops as Roko's workers

Coding agents run the inner tool-using loop: Claude Code and Codex, which also run headless
[@anthropic2026cchheadless; @openai2026codexexec], OpenHands [@wang2025openhands], SWE-agent [@yang2024sweagent],
Goose [@aaif2026goose] and Aider [@aider2024architect]. Roko runs such loops as workers, one adapter per provider
kind, and adds what a single session does not hold (§3.1).

## 10.3 Harness-level orchestrators

Agentless lets tests select the patch [@xia2025agentless], and Magentic-One's orchestrator, itself a model, keeps
progress ledgers and replans [@fourney2024magentic]. Claude Code's dynamic workflows choose each subagent's model and
worktree [@shihipar2026harness], Kiro runs a spec's independent tasks in dependency waves [@kiro2026specs], and
Factory's router moves a session to a stronger model or a healthy provider [@factory2026router], the two axes of
Roko's escalation ladder and provider failover. Roko shares the idea that the harness, not the agent, decides by
tests, and differs in who regulates: code acting on the executable verdict and the blame it records.

## 10.4 Harness engineering

Practitioners call everything around the model the harness [@trivedy2026anatomy]. Böckeler divides it into
feedforward guides and feedback sensors [@bockeler2026harness]; in those terms, Roko's planned checks are guides and
its gates computational sensors. Systems that optimize the harness itself search its code offline [@lee2026meta] or
commit changes behind regression guards [@kang2026harness]. Roko does not search its own code: its settings move one
notch at a time under guarded commit, and the loop audit tests whether each learning loop helps.
The nearest precedent we found tests offline whether stored outcomes change an agent's decisions [@xu2026replaylens].

## 10.5 Routing and cascades

Cascades [@chen2024frugalgpt] and learned routers [@ong2025routellm] trade cost against quality, and their value
depends on the quality estimate [@dekoninck2024unified]. Roko claims no new routing algorithm: its ladder is a cascade
scored by the task's own check, and its learned model router learns from verdict records. The nearest precedent we
found learns cross-vendor routing from benchmark outcomes [@li2026rsirouter].

## 10.6 Verification and audits

LLM judges show systematic biases [@zheng2023judging], weak tests inflate results [@yu2025utboost], and agents exploit
visible tests, up to editing them [@zhong2025impossiblebench]. Acceptance error has been estimated from planted mutants
[@bhardwaj2026bounded] and from audit samples with logged selection probabilities [@gao2026pinsieve]. Roko's
acceptance check is executable and out of the actor's reach, and random deep audits estimate how often it passes
wrong work.

## 10.7 Self-adaptive systems and cybernetics

Roko follows autonomic computing [@kephart2003vision] and self-adaptive software [@weyns2020introduction] in
separating a managed system, the agents, from a managing one, the harness loops. Most work joining LLMs with such
loops makes the model the manager [@nascimento2023selfadaptive]; in Roko, models are managed workers and the
regulators are code. The closest framing we know of, *Agent Cybernetics* [@wang2026agent], is a position paper that
maps cybernetic laws, Ashby's ultrastability among them, onto principles for a single agent. Roko applies kindred
ideas to a multi-agent harness and claims no priority over it.
