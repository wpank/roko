# Grant Opportunities

**Date**: August 2026

TraceCommons (TC): open-source Rust AI trace registry, ~235K LOC, TEE-scored quality/novelty, NEAR credit settlement. MIT/Apache-2.0. 3 contributors, 41 PostgreSQL migrations, 8 CI gates. Built by Zaki Manian (Cosmos SDK, IBC, Sommelier).

This document covers practical, accessible grant opportunities. Framing: TC is complementary infrastructure for the AI agent ecosystem -- developer tooling, compliance, and quality scoring -- not a competing platform.

---

## 1. NLnet NGI Zero Restack — Most Accessible, Submit First

| | |
|---|---|
| **Amount** | Up to EUR 48,000 |
| **Deadline** | November 3, 2026 (opens Sep 3) |
| **URL** | https://nlnet.nl/restack/ |
| **Format** | Application form (not a full proposal -- can be done in a day) |
| **PI requirement** | None. Zaki applies directly. |

### Angle: EU AI Act Compliance Tooling

Article 12 (mandatory logging for high-risk AI) took effect August 2, 2026. TC is the first open-source compliance infrastructure that preserves contributor sovereignty. NLnet is EU-funded -- this is the strongest framing.

### What to emphasize

- **"As of this month, Article 12 is law."** TC provides compliant logging infrastructure that's open-source and privacy-preserving.
- **User sovereignty over AI behavioral data.** Model providers capture session data unilaterally; TC reverses this.
- **Privacy architecture.** Client-side redaction, TEE scoring, cell suppression, hash-only audit. Structural, not cosmetic.
- **3rd contributor** (brapse, Aug 10) -- organic growth signal matters to NLnet.

### Milestones (EUR 12K x 4)

1. **OTel-native ingest**: Accept OTel GenAI spans so any instrumented agent can contribute without custom SDK
2. **Error Hub MVP**: Searchable failure-diagnosis-repair bundles with scrubbing and consent
3. **Skill safety scoring**: Quality/security scores for SKILL.md artifacts derived from trace corpus
4. **Self-service onboarding**: Prebuilt binaries, GitHub OAuth registration, `tc scan` with local insights

### Realistic assessment

Most accessible grant. Low-effort application, no PI requirement, strong EU AI Act angle. **Submit this one first.**

---

## 2. NEAR Foundation DevHub — Ecosystem Home

| | |
|---|---|
| **Amount** | Up to $120,000 |
| **Deadline** | Rolling |

### Angle: Developer Tooling That Brings Users to NEAR

Frame TC as **user-facing developer tooling** that makes NEAR accessible to AI developers through abstraction (device keys, credit settlement), not as blockchain infrastructure.

### What to emphasize

- **Organic chain activity.** Every accepted trace produces a NEAR transaction -- real utility, not speculation.
- **New user category.** AI developers interacting with NEAR through TC's abstraction. Mainstream adoption via developer tooling.
- **IronClaw synergy.** 12.6K-star project already shipping TC integration. WASM fuel metering as quality signal.
- **Practical deliverables.** Focus on things IronClaw users and AI developers actually touch: scoring feedback, credit visibility, cross-provider comparison analytics.

### Proposed milestones (3 phases, $40K each)

**Phase 1: Developer experience** (3 months)
- Immediate scoring feedback in IronClaw (quality score + credits earned inline after each session)
- Prebuilt binaries + self-service registration
- `tc scan` with local insights for IronClaw users

**Phase 2: Ingest expansion** (3 months)
- OTel-native ingest endpoint (any OTel-instrumented agent → TC → NEAR settlement)
- WASM fuel as quality signal in scoring pipeline
- Cross-provider comparison analytics (private to contributors)

**Phase 3: Ecosystem growth** (3 months)
- Error Hub with searchable failure bundles
- SKILL.md publishing from corpus (manually curated, scored, with provenance)
- Contributor leaderboard and founding contributor recognition

### Practical considerations

- Rolling deadline: no rush, but no reason to delay.
- Get IronClaw team buy-in. A warm introduction from their maintainers matters more than a cold application.

---

## 3. Mozilla Technology Fund — Trustworthy AI

| | |
|---|---|
| **Amount** | $50K-$150K |
| **Deadline** | Monitor MTF call schedule for AI accountability themes |
| **Fit** | Strong alignment with "Trustworthy AI" initiative |

### Angle: Open Alternative to Surveillance-Based AI Data Collection

TC's "user-owned data commons" maps directly to Mozilla's values. Common Voice is a close analogy: crowd-sourced, quality-gated, openly licensed.

### What to propose

Focus on the **local-first developer tool** aspect: session analytics, quality scoring, failure debugging that works entirely on the developer's machine before anything is shared. This inverts the surveillance model -- you get value locally, sharing is opt-in.

**Deliverables**:
- Local-first `tc scan` with personal analytics (cost breakdown, efficiency patterns, quality trends)
- Cross-agent session comparison (Claude Code vs Codex vs Cursor -- data developers care about)
- Privacy-preserving contribution flow with TEE-scored quality gates
- "Agent Skills Safety Score" -- quality/security scoring for SKILL.md artifacts addressing the 37% security flaw rate

### Practical considerations

- Monitor call schedule. Mozilla opens themed rounds periodically.
- Privacy-first framing resonates. Lead with TEE scoring, client-side redaction, contributor-controlled consent.

---

## 4. Protocol Labs / Filecoin DevGrants — Provenance

| | |
|---|---|
| **Amount** | $10K-$100K |
| **Deadline** | Rolling |
| **Fit** | Moderate -- specific integration angle |

### Angle: Content-Addressable Trace Storage

TC's trace envelopes are content-addressable (deterministic hashes). IPFS/Filecoin backend for TC's encrypted artifact store is a natural fit. C2PA, SCITT provenance story aligns.

### What to propose

A focused integration grant, not a full platform grant:
- IPFS/Filecoin backend for cold trace storage (content-addressed, verifiable)
- C2PA content provenance integration for trace authenticity marking
- Encrypted artifact pinning with contributor-controlled access

**Effort**: Small, focused. $10-25K range is realistic.

---

## 5. NSF PESOSE Track 1 — Target March 2027

| | |
|---|---|
| **Amount** | Up to $300,000 / 2 years |
| **Deadline** | ~September 1, 2026 (3 weeks -- too tight without a PI) |
| **URL** | https://www.nsf.gov/pubs/2024/nsf24594/nsf24594.htm |

### Why defer

PESOSE requires a PI at a US institution. 3 weeks is too tight for a first NSF submission without an existing PI relationship. A weak submission wastes the opportunity.

### What to do now

- **Start the PI conversation.** Natural fits: CMU (LoGra/LogIX authors -- their data-valuation method applied to a real trace commons), Berkeley (sleep-time compute, Letta), Stanford (AI safety).
- **Use NLnet/NEAR grants as evidence.** Prior funding from smaller programs strengthens the NSF application.
- **Sharpen ecosystem framing.** PESOSE funds governance, community, and sustainability -- not features. Emphasize TSC formation, contributor pipeline, academic partnerships.
- **Target March 2027 cycle** with stronger metrics, PI relationship, and prior grant track record.

---

## 6. Other Opportunities (Lower Priority)

### Open Philanthropy ($100K-$1M+)

**Angle**: TC as research infrastructure for empirical AI safety. Researchers studying real-world agent failures need a curated corpus. Failure-attribution potential strengthens the case.

**Action**: Worth a conversation with their AI safety team after Error Hub ships.

### EU Horizon Europe (EUR 500K+)

Strong conceptual alignment but requires multi-country consortium, coordinating institution, months of preparation. **File for later** once NLnet establishes European academic connections.

---

## 7. Stacking Strategy

Grants are not mutually exclusive. Ideal outcome:

| Grant | Amount | Angle | Timeline |
|---|---|---|---|
| NLnet Restack | EUR 48K | EU AI Act compliance, privacy | Nov 2026 |
| NEAR DevHub | $120K | Developer tooling, ecosystem | Dec 2026 |
| Mozilla Tech Fund | $50-150K | Trustworthy AI, local-first tools | H1 2027 |
| NSF PESOSE | $300K | Governance, community, sustainability | Mar 2027 |

Total ~$500K+ over 2 years. Each funds different aspects with no double-dipping. NLnet and NEAR serve as evidence for NSF.

---

## 8. Cross-Cutting Themes

Every proposal should hit 3-4 of these, tailored to the funder:

| Theme | NLnet | NEAR | Mozilla | NSF |
|---|---|---|---|---|
| EU AI Act compliance | **Lead** | Mention | Mention | Supporting |
| Privacy architecture | **Lead** | Supporting | **Lead** | Supporting |
| Developer tooling / UX | Supporting | **Lead** | **Lead** | Supporting |
| Ecosystem growth (NEAR) | -- | **Lead** | -- | Mention |
| Governance / sustainability | Supporting | -- | Supporting | **Lead** |
| Agent Skills safety scoring | Supporting | Supporting | Supporting | Mention |
| Open-source commons model | Supporting | Mention | **Lead** | **Lead** |

### Founder credibility

Zaki Manian: co-created Cosmos SDK (~$50B+ in blockchain value), designed and shipped IBC (cross-chain interoperability protocol), built Sommelier (DeFi protocol with real TVL). Demonstrated ability to design, ship, and maintain critical open-source infrastructure.

---

## 9. TC Stats for Applications (August 2026)

| Metric | Value |
|---|---|
| Language | Rust (edition 2024, MSRV 1.92) |
| License | MIT OR Apache-2.0 |
| Crates | 6 |
| LOC | ~235,000 |
| Migrations | 41 (PostgreSQL, forced RLS) |
| Binaries | 8 |
| CI gates | 8 |
| Deployment | GCP (pilot), NEAR AI Cloud (TEE-hosted vLLM) |
| Contributors | 3 (incl. brapse, Aug 10, 2026) |
| IronClaw integration | 3 PRs merged, 20K+ lines |
| Scoring model | Qwen 3.6 35B-A3B-FP8 (AUC > 0.93) |

---

## 10. Timeline

| When | Action |
|---|---|
| Aug 10-15 | Decide NSF: do you have a PI? If no, defer to Mar 2027. |
| Sep 3 | NLnet Restack opens. Begin application. |
| Sep-Oct | Prepare NEAR DevHub application. Ship prebuilt binaries + self-service registration. |
| Nov 3 | Submit NLnet. |
| Nov-Dec | Submit NEAR DevHub. Monitor Mozilla call schedule. |
| 2027 Q1 | Mozilla application. Begin NSF PESOSE preparation if PI secured. |
