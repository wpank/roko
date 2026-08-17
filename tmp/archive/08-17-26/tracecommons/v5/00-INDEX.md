# TraceCommons v5 — Strategic Documents

**Date**: August 2026
**Synthesized from**: v3 (10 docs, full depth) + v4 (6 docs, tight focus) + research2 findings + new deep research queries

---

## Documents

| # | Document | Pages | Focus |
|---|---|---|---|
| [01](01-user-acquisition-and-growth.md) | **User Acquisition & Growth** | ~300 lines | Core problem, 3 must-ship changes, single-player hooks, Error Hub flywheel, skill publishing as viral distribution, cold-start playbook, growth milestones, 10 deep research queries |
| [02](02-scoring-and-quality-pipeline.md) | **Scoring & Quality Pipeline** | ~400 lines | Bake-off confound fix, TokenRarity wiring, MinHash dedup, human annotation, multi-layer pipeline (5 layers), 8 research-backed upgrades with full citations and verification, dependency graph, risk assessment, 5 deep research queries |
| [03](03-integrations-and-ecosystem.md) | **Integrations & Ecosystem** | ~350 lines | OTel ingest (full attribute mapping), Error Hub schema, skills publishing, protocol events (MCP/A2A/W3C), trajectory replay, IronClaw status (shipped PRs, 4 critical fixes with technical detail, 5 opportunities), 10 deep research queries |
| [04](04-production-hardening.md) | **Production Hardening** | ~250 lines | Full v3 depth: observability (OTel, Prometheus, 14 metrics, SLOs), reliability (graceful shutdown, health/ready, migration extraction, redaction fuzzing, cell suppression), scale readiness (containers, dedup persistence, gate extraction, sleep-time). 18 items across 4 tiers. |
| [05](05-strategy-and-grants.md) | **Strategy & Grants** | ~300 lines | Market context (consolidation wave, OTel, EU AI Act, A2A), four-pillar moat, competitive matrix, 5 grant opportunities (NLnet lead, NEAR, Mozilla, Protocol Labs, NSF) with specific milestones and stacking strategy, 5 deep research queries |
| [06](06-research-paper-index.md) | **Research Paper Index** | ~300 lines | ~122 papers across 9 categories with citations, URLs, and TC relevance. Conference watchlist. |

---

## What This Version Is

v5 is the synthesis of v3's depth and v4's focus. It preserves:

**From v3:**
- Full technical detail on every production hardening item (LOC counts, metric names, SLO targets, intersecting PRs)
- Full IronClaw integration status (shipped PRs, all 4 critical fixes with trust-path analysis)
- Complete research paper index (~122 papers, 9 categories)
- Full scoring pipeline technical detail (trait names, confound analysis, validation approaches)
- Detailed OTel attribute mapping table
- The "What Not to Do" section with reasoning
- Cold-start playbook with evidence
- All effort estimates

**From v4:**
- Tighter organization (6 docs instead of 10)
- Grant opportunities rewritten as complementary, practical, user-oriented (not project-takeover)
- Research findings woven into implementation context (not standalone reports)
- Cross-document priority order
- Unified competitive matrix

**New in v5:**
- **30 deep research queries** across 5 documents, targeting: growth/acquisition tactics (10), scoring/quality methods (5), integrations/ecosystem opportunities (10), strategy/market intelligence (5)
- Error Hub as a growth flywheel (not just a feature)
- Explicit cold-start playbook with evidence-backed tactics
- Expanded single-player hooks section
- Risk assessment table for scoring pipeline items
- Dependency graph for scoring pipeline sequencing

---

## Deep Research Queries Summary

All queries are copy-paste-ready for Perplexity Pro, Google Scholar, or general web search.

| Doc | Query IDs | Topics |
|---|---|---|
| 01 User Acquisition | Q-G1 through Q-G10 | One-click integrations, viral loops, AI cost transparency, failure databases, cross-harness comparison, data donation UX, background telemetry, agent skills growth, Show HN strategy, EU AI Act compliance market |
| 02 Scoring Pipeline | Q-S1 through Q-S5 | Label-free quality scoring at scale, trace dedup, novelty for sequential data, real-time quality gates, trace compression SOTA |
| 03 Integrations | Q-I1 through Q-I10 | Auto-instrumentation, OTel GenAI adoption, agent plugin architectures, cross-agent stitching, session backup tools, LangSmith format, emerging harnesses, IDE extensions, CI/CD integration, mobile/web agents |
| 05 Strategy | Q-M1 through Q-M5 | EU AI Act tools market, AI observability market size, data commons governance, agent quality standards, privacy-preserving sharing platforms |

---

## Priority Order (Across All Documents)

### Now (Weeks)

1. Prebuilt binaries + one-line install (01, 2-3 days)
2. Self-service registration (01, 1-2 days)
3. Wire TokenRarityScorer (02, hours)
4. MinHash dedup via Rensa (02, 1-2 days)
5. Prometheus metrics + tower-http TraceLayer (04, ~80 LOC + 1 line)
6. Graceful shutdown + /health/ready (04, ~150 LOC)
7. `tc scan` with immediate local insights (01, 1-2 weeks)

### Next (1-3 Months)

8. OTel-native ingest (03, 2-4 weeks)
9. Fix bake-off corpus (02, ~1 week)
10. Start human annotation (02, 40-80 person-hours)
11. IronClaw critical fixes (03, 4 items)
12. NLnet application (05, submit Nov 3)
13. NEAR DevHub application (05, rolling)
14. Error Hub MVP (03, 6-8 weeks)
15. First analysis post (01, 1 week)
16. Cell suppression replacing DP (04, ~100 LOC)
17. Immediate scoring feedback in IronClaw (03)

### Then (3-6 Months)

18. Skill publishing — manual curation (03, 1-2 weeks)
19. Multi-layer novelty pipeline (02)
20. Trajectory replay prototype (03, 8-10 weeks)
21. Mozilla Tech Fund application (05)
22. Container image (04)
23. Compound system auto-optimization (02)
24. NSF PESOSE preparation if PI secured (05)
