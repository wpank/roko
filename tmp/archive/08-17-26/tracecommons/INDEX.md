# TraceCommons: Ideas & Research Notes

> **Date**: 2026-08-10
> **Repo**: [TraceCommons/trace-commons-server](https://github.com/TraceCommons/trace-commons-server)
> **Tone**: Brainstorming and research notes, not proposals. Ideas from reading the codebase, open PRs, and academic literature.

---

## v3 — Current Iteration (~3,400 lines)

Synthesized from v2 + [research notes](research/reserach1.md). Each doc is self-contained. Focused on three priorities: **getting users**, **fixing novelty detection**, and **production hardening**.

| # | Document | Lines | What |
|---|----------|-------|------|
| 1 | [Getting First Users](v3/01-getting-first-users.md) | 429 | User acquisition as a DevEx problem. OTel ingest, trajectory replay, Error Hub, SKILL.md distribution. Quick wins, anti-patterns, milestones. |
| 2 | [Novelty Detection Models](v3/02-novelty-detection-models.md) | 590 | Short-term fixes (TokenRarityScorer, MinHash, corpus fix), medium-term pipeline (NCD, process mining, failure attribution, LEGOMem), long-term valuation (LoGra/LogIX, skill extraction, VET). Decision framework. |
| 3 | [Production Hardening](v3/03-production-hardening.md) | 226 | Observability (OTel, Prometheus, SLOs), reliability (graceful shutdown, /health/ready, redaction fuzzing), scale readiness (containers, gate extraction). Priority order. |
| 4 | [Integration Opportunities](v3/04-integration-opportunities.md) | 403 | OTel-native ingest, Agent Skills output, Error Hub, trajectory replay, MCP/A2A protocols. |
| 5 | [Research Innovations](v3/05-research-innovations.md) | 337 | 16 innovations in 3 tiers by implementability. Each grounded in 2025-2026 papers. |
| 6 | [Deep Research Queries](v3/06-deep-research-queries.md) | 255 | 15 copy-paste search queries, 10 under-explored directions, 15-venue watchlist. |
| 7 | [Competitive Positioning](v3/07-competitive-positioning.md) | 151 | 2026 consolidation wave. Four strategic imperatives. EU AI Act now live. |
| 8 | [IronClaw Integration](v3/08-ironclaw-integration.md) | 251 | Status, critical fixes, 5 opportunities, user acquisition via IronClaw. |
| 9 | [Grant Opportunities](v3/09-grant-opportunities.md) | 331 | NSF PESOSE ($300K), NLnet (EUR 48K), NEAR DevHub ($120K), plus 4 others. |
| 10 | [Research Paper Index](v3/10-research-paper-index.md) | 400 | ~122 papers across 9 categories. [v3] tags on new additions. |

---

## v2 — Focused Iteration (~2,500 lines, uploaded as gists)

Focused on TC's actual priorities. Brainstorming tone, grounded in codebase analysis and open PRs.

| # | Document | Lines | Gist |
|---|----------|-------|------|
| 1 | [Novelty Detection Ideas](v2/ideas-novelty-detection.md) | 534 | [gist](https://gist.github.com/wpank/23f07eb8cb1ec07db6dcea176e31c74a) |
| 2 | [Getting First Users](v2/ideas-first-users.md) | 584 | [gist](https://gist.github.com/wpank/5b8ae17221b269fadb5610fb2428a883) |
| 3 | [Production Notes](v2/ideas-production-notes.md) | 496 | [gist](https://gist.github.com/wpank/d4c81c94e70bd1bc1e47354ee7ea7114) |
| 4 | [Competitive Positioning](v2/ideas-competitive-positioning.md) | 150 | [gist](https://gist.github.com/wpank/7989a7f48d1029ce5b5173d1227efa39) |
| 5 | [IronClaw Integration Notes](v2/notes-ironclaw-integration.md) | 293 | [gist](https://gist.github.com/wpank/c1a372b003538f31a77d8471db301a3d) |
| 6 | [Research Paper Index](v2/research-paper-index.md) | 432 | [gist](https://gist.github.com/wpank/1cd3a83a80308aba9a9b6349beebcbda) |

---

## v1 — Comprehensive (~13,300 lines, uploaded as gists)

Original comprehensive document set. More prescriptive. Kept for reference and grant material.

| Document | Lines | Gist |
|----------|-------|------|
| [Implementation Roadmap](v1/tc-implementation-roadmap.md) | 2,865 | [gist](https://gist.github.com/wpank/70d122a6701166d13c46f8ee8f106a3a) |
| [Grant Proposals](v1/tc-grant-proposals.md) | 1,524 | [gist](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8) |
| [Privacy & Security](v1/tc-privacy-security.md) | 2,224 | [gist](https://gist.github.com/wpank/536e761778dfb5bfc769f86eb384a0c5) |
| [Novel Research Ideas](v1/tc-novel-research-ideas.md) | 2,573 | [gist](https://gist.github.com/wpank/164cf6fa340c1a98cdbd3bda0c73a09d) |
| [Competitive Landscape](v1/tc-competitive-landscape.md) | 1,034 | [gist](https://gist.github.com/wpank/27ced0dd009062464f304ac1925a9e07) |
| [UX & Dashboard Design](v1/tc-ux-design.md) | 1,481 | [gist](https://gist.github.com/wpank/6cf0b1dca0d28250485297305559760b) |
| [IronClaw Integration](v1/tc-ironclaw-integration.md) | 1,605 | [gist](https://gist.github.com/wpank/d04c688c8b088852a26cd33817bb827a) |
| [Original Analysis](v1/tracecommons-roko-analysis.md) | 1,522 | — |

---

## Research Notes

| Document | What |
|----------|------|
| [research/reserach1.md](research/reserach1.md) | Net-new AI-agent research for TC: LoGra influence functions, skill distillation, OTel GenAI, Agent Skills, VET, AgentGUI, AgentDebugX, evidence tracing, LEGOMem, compound AI optimization. Recommendations in 3 stages. |

---

## Grant Deadlines

| Program | Amount | Deadline | Where |
|---------|--------|----------|-------|
| NSF PESOSE Track 1 | $300,000 | ~Sep 1, 2026 | [v3 notes](v3/09-grant-opportunities.md), [v1 draft](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8#proposal-3-nsf-pesose-track-1) |
| NLnet Restack | EUR 48,000 | Nov 3, 2026 | [v3 notes](v3/09-grant-opportunities.md), [v1 draft](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8#proposal-1-nlnet-foundation----ngi-zero-restack) |
| NEAR Foundation DevHub | $120,000 | Rolling | [v3 notes](v3/09-grant-opportunities.md), [v1 draft](https://gist.github.com/wpank/2cde449f7dc002b3eebc0a4a492475a8#proposal-2-near-foundation-developer-hub-grants) |

---

## What I Learned From Reading the Codebase

**The engineering quality is exceptionally high.** RLS on every table with FORCE. Hash-only logging. 17 operational drill endpoints. Multi-round adversarial review with mutation testing.

**The novelty detection problem is real and well-understood internally.** PR #216 demonstrates the bake-off was confounded. PR #173 proposes human-judgment validation. The team knows this is broken and is being methodical about fixing it.

**The contributor experience pipeline is the current focus.** PRs #244 (daemon), #248 (GTK shell), #247 (Windows support), #241 (private insight) are all about making contribution passive, platform-native, and personally valuable.

**A third contributor (brapse) appeared on Aug 10** with PR #250.

---

## Quick Reference: What's Being Worked On

Based on open PRs as of 2026-08-10:

| Priority | Open PRs | Status |
|----------|----------|--------|
| **Novelty detection** | #216 (baseline dominance floor), #173 (human annotation infrastructure) | Foundational |
| **Contributor platform** | #244 (daemon), #248 (Linux GTK), #247 (Windows named pipe), #241 (private insight) | Active |
| **Privacy/trust** | #238 (honest DP accounting), #246 (revocation fix), #201 (redaction re-scan) | Critical fixes |
| **Production** | #227 (column-scoped grants), #226 (atomic settlement) | Hardening |
