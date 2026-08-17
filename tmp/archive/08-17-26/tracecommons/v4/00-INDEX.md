# TraceCommons v4 — Strategic Documents

**Date**: August 2026
**Synthesized from**: v3 documents (01-10) + research round 2

---

## Documents

| # | Document | Focus |
|---|---|---|
| [01](01-user-acquisition.md) | **User Acquisition & Growth** | Getting first users, single-player value, distribution channels, Error Hub, skill publishing, growth milestones |
| [02](02-scoring-pipeline.md) | **Scoring Pipeline** | Fixing the confounded bake-off, multi-layer novelty pipeline, research-backed upgrades (label-free scoring, causal attribution, marginal value, compression) |
| [03](03-production-and-integrations.md) | **Production & Integrations** | Production hardening tiers, OTel ingest, Error Hub, skills, protocol events, trajectory replay, IronClaw status + fixes |
| [04](04-competitive-positioning.md) | **Competitive Positioning** | Market context, consolidation wave, four-pillar moat, competitive matrix, strategic imperatives |
| [05](05-grant-opportunities.md) | **Grant Opportunities** | NLnet, NEAR DevHub, Mozilla, Protocol Labs, NSF PESOSE -- practical angles, milestones, stacking strategy |
| [06](06-research-and-references.md) | **Research & References** | 7 highest-impact findings, tiered innovations, 122-paper index summary, research queries, verification ledger |

---

## What Changed from v3

**Consolidated** 10 documents → 6 by merging overlapping content:
- Getting first users + integration opportunities → **01 User Acquisition**
- Novelty detection models + research innovations → **02 Scoring Pipeline**
- Production hardening + integration opportunities + IronClaw → **03 Production & Integrations**
- Competitive positioning → **04 Competitive Positioning** (tightened)
- Grant opportunities → **05 Grant Opportunities** (completely rewritten)
- Deep research queries + research paper index + research2 findings → **06 Research & References**

**Grant opportunities rewritten from scratch.** v3 framed grants as ambitious platform takeovers (NSF $300K lead, Horizon Europe EUR 5M, Open Phil $1M). v4 reframes:
- Leads with NLnet (most accessible, no PI, EUR 48K, submit first)
- NEAR DevHub as developer tooling that brings users to NEAR (not blockchain infrastructure)
- Mozilla as local-first trustworthy AI tools (not a competing platform)
- NSF deferred to March 2027 with concrete preparation steps
- All milestones rewritten as practical, user-facing deliverables: prebuilt binaries, OTel ingest, Error Hub, `tc scan` insights, skill safety scoring
- Framing is complementary (TC enhances the ecosystem) rather than ambitious (TC replaces existing tools)

**Research findings integrated into actionable context.** The 7 highest-impact findings from research2 are woven into the scoring pipeline document with specific TC implementation guidance rather than standing as a separate research report.

---

## Priority Order (Across All Documents)

### Now (Weeks)

1. Prebuilt binaries + one-line install
2. Self-service registration (drop invite-code)
3. Prometheus metrics + tower-http TraceLayer + graceful shutdown
4. Wire TokenRarityScorer (hours)
5. MinHash dedup via Rensa (1-2 days)
6. `tc scan` with immediate local insights

### Next (1-3 Months)

7. OTel-native ingest + MCP tool-call events
8. Fix bake-off corpus + start human annotation
9. IronClaw critical fixes (TLS, quarantine, redaction_hash, behavioral tests)
10. NLnet application (submit Nov 3)
11. NEAR DevHub application
12. Error Hub MVP

### Then (3-6 Months)

13. Skill publishing (manual curation)
14. Trajectory replay prototype
15. Multi-layer novelty pipeline
16. Mozilla Tech Fund application
17. NSF PESOSE preparation (if PI secured)
18. First analysis post ("What we learned from N AI coding sessions")
