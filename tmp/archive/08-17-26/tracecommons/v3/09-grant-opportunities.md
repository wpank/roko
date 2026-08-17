# Grant Opportunities & Funding Strategy

**Date**: August 2026

TraceCommons (TC) is an open-source, privacy-preserving AI trace registry created
by Zaki Manian (Cosmos SDK, IBC, Sommelier). Rust, ~235K LOC across 6 crates,
TEE-hosted scoring on NEAR AI Cloud, credit settlement on NEAR, pilot deployed.
MIT/Apache-2.0 dual-licensed. 3 contributors, 41 PostgreSQL migrations, 8 CI gates.
IronClaw integration substantially merged (3 PRs, 20K+ lines).

This document is a practical reference for grant pursuit: which programs fit,
deadlines, angles, and what needs to happen to submit.

---

## Table of Contents

1. [NSF PESOSE Track 1](#1-nsf-pesose-track-1) --- biggest, hardest
2. [NLnet NGI Zero Restack](#2-nlnet-ngi-zero-restack) --- most accessible
3. [NEAR Foundation DevHub](#3-near-foundation-devhub) --- ecosystem home
4. [Other Strong-Fit Opportunities](#4-other-strong-fit-opportunities)
5. [Cross-Cutting Themes](#5-cross-cutting-themes)
6. [Practical Notes](#6-practical-notes)

---

## 1. NSF PESOSE Track 1

| | |
|---|---|
| **Amount** | Up to $300,000 / 2 years |
| **Deadline** | ~September 1, 2026 (~3 weeks) |
| **URL** | https://www.nsf.gov/pubs/2024/nsf24594/nsf24594.htm |

### Why TC fits

PESOSE funds the *ecosystem* around open-source software --- governance, community,
sustainability --- not more features. TC's code is built; what's missing is the
organizational infrastructure to turn a single-team pilot into a self-sustaining commons.

- **Structural gap.** TC is the PyPI/crates.io of agent traces --- shared infrastructure
  no commercial vendor will build. Observability platforms (Langfuse, Braintrust, Galileo,
  Helicone --- all acquired in 2026) capture traces but never share across organizations.
- **Research-grounded.** LoGra/LogIX (NeurIPS 2025) for influence-function data valuation;
  OTel GenAI semantic conventions for interoperability; Compound AI Systems Optimization
  (EMNLP 2025); VET verifiable execution traces.
- **Privacy architecture as contribution.** Three-layer redaction, TEE scoring, DP
  aggregates, hash-only audit --- a novel privacy-preserving data commons design.
- **Concrete broader impacts.** Academic research infrastructure (3-5 groups using TC),
  contributor compensation via Trace Credits, EU AI Act Article 12 compliance, workforce
  development curriculum.

### What needs to happen (~3 weeks)

1. **Find a PI.** PESOSE requires a PI at a US institution. This is the single biggest
   blocker. Natural fits: CMU (LoGra authors), Berkeley (Letta/sleep-time compute),
   Stanford (AI safety). Without a PI, there is no submission.
2. **Update research citations.** v1 draft cites foundational work; add the 2025-26
   results (LoGra/LogIX, OTel GenAI, AgentDebugX, VET).
3. **Update codebase metrics.** v1 says ~62K LOC; current is ~235K.
4. **Sharpen ecosystem framing.** Emphasize governance charter, TSC formation,
   contributor pipeline, academic partnerships --- not features.
5. **Letters of support.** 2-3 from academic groups, industry pilot partners.
6. **Budget review.** $300K/2yr is tight: 0.5 FTE senior dev, 0.5 FTE junior dev,
   0.25 FTE community manager, infra, travel.

**Realistic assessment**: 3 weeks is very tight for a first NSF submission, especially
finding a PI. If the PI relationship does not already exist, target the next PESOSE
cycle (~March 2027) instead. A weak NSF submission wastes the opportunity.

---

## 2. NLnet NGI Zero Restack

| | |
|---|---|
| **Amount** | Up to EUR 48,000 |
| **Deadline** | November 3, 2026 (opens Sep 3) |
| **URL** | https://nlnet.nl/restack/ |

### Why TC fits

NLnet funds projects that contribute to an open internet: user sovereignty, privacy,
open standards, alternatives to platform monopolies.

- **Anti-monopoly framing.** Model providers capture session data unilaterally; TC
  reverses this by putting data under contributor control.
- **Privacy architecture.** Client-side redaction, TEE scoring, DP aggregates, hash-only
  audit, PostgreSQL RLS, scope-based consent. Structural, not cosmetic.
- **EU AI Act compliance.** Article 12 (mandatory logging for high-risk AI) took effect
  Aug 2, 2026. TC is the first open-source compliance infrastructure that preserves
  contributor sovereignty. NLnet is EU-funded --- this is the strongest angle.
- **No PI requirement.** Zaki can apply directly. Removes the biggest PESOSE blocker.
- **Low effort.** Application form, not a full proposal. Can be done in a day.

### What needs to happen

1. Update v1 milestones with OTel GenAI adoption and failure-attribution labels.
2. Strengthen the EU AI Act angle: "As of this month, Article 12 is law."
3. Mention the 3rd contributor (brapse, Aug 10) --- organic growth matters to NLnet.

Milestone-based disbursement: EUR 12K x 4 milestones over 12 months.

**Realistic assessment**: Most accessible grant. Submit this one.

---

## 3. NEAR Foundation DevHub

| | |
|---|---|
| **Amount** | Up to $120,000 |
| **Deadline** | Rolling |

### Why TC fits

TC is already a NEAR ecosystem project with deep integration:

- **Credit settlement on NEAR**: hash-only attestations, settlement batches, NEAR
  receipt outbox, three modes (disabled/dry_run/http)
- **NEAR AI Cloud scoring**: TEE-hosted vLLM (Intel TDX + NVIDIA GPU TEE)
- **NEAR identity enrollment**: contributor NEAR identities for payout designation
- **IronClaw integration**: 3 PRs merged, 20K+ lines; `ironclaw.trace_contribution.v1`
  envelope is the contract between contributor and server

### What to emphasize

- **Transaction volume.** Every accepted trace produces a NEAR transaction --- organic
  chain activity from real utility, not speculation.
- **New user category.** AI developers interacting with NEAR through TC's abstraction
  (device keys, credit settlement) --- mainstream adoption.
- **IronClaw synergy.** WASM fuel metering as quality signal, deeper TEE attestation
  bridging, IronClaw-native trace capture.
- **Phase 2 federation.** NEAR as coordination layer: instance registry contract,
  cross-instance identity anchoring, data licensing contracts.

### Practical considerations

- Rolling deadline: no rush, but no reason to delay.
- v1 draft has a complete NEAR submission (3 phases, $40K each, 9 months).
- Get IronClaw team buy-in. A warm introduction from their maintainers is more
  effective than a cold grant application.

---

## 4. Other Strong-Fit Opportunities

### Mozilla Technology Fund

| | |
|---|---|
| **Amount** | $50K-$150K |
| **Fit** | Strong |

Mozilla's "Trustworthy AI" initiative aligns directly: AI transparency, data
sovereignty, alternatives to surveillance-based data collection.

- TC's "user-owned data commons" maps to Mozilla's values
- Common Voice is a close analogy: crowd-sourced, quality-gated, openly licensed
- Privacy-first design (TEE scoring, client-side redaction) resonates
- Monitor MTF call schedule for AI accountability or data sovereignty themes

### Open Philanthropy

| | |
|---|---|
| **Amount** | $100K-$1M+ |
| **Fit** | Moderate-Strong |

Angle: TC as research infrastructure for empirical AI safety. Researchers studying
real-world agent failures need a curated corpus. TC provides this. Failure-attribution
potential (AgentDebugX-style) strengthens the case. Open Phil funds infrastructure
that enables safety research, not just research itself.

**Action**: Worth a conversation with their AI safety team.

### EU Horizon Europe

| | |
|---|---|
| **Amount** | EUR 500K-5M+ |
| **Fit** | Strong conceptually, high barrier |

Perfect alignment with calls on AI transparency and trustworthy AI; EU AI Act
compliance angle is very strong. But requires a multi-country consortium (3+
partners), a coordinating institution, and months of proposal preparation.

**Action**: Do not pursue now. File for later once NLnet/NEAR grants establish
TC and European academic connections exist.

### Protocol Labs / Filecoin DevGrants

| | |
|---|---|
| **Amount** | $10K-$100K |
| **Fit** | Moderate |

TC's data provenance story (attestation chain, C2PA, SCITT) aligns. TC's encrypted
artifact store could add an IPFS/Filecoin backend. Content-addressable storage maps
naturally to trace envelopes (each has a deterministic hash).

**Action**: Low priority. Only if other grants do not materialize.

### Priority Summary

| Opportunity | Fit | Amount | Priority |
|---|---|---|---|
| **NLnet Restack** | Strong | EUR 48K | **Submit Nov 3** |
| **NEAR DevHub** | Strong | $120K | **Submit (rolling)** |
| **NSF PESOSE** | Strong | $300K | **Submit if PI found; else Mar 2027** |
| **Mozilla Tech Fund** | Strong | $50-150K | **Investigate now** |
| **Open Philanthropy** | Moderate-Strong | $100K-1M | **Investigate now** |
| **Horizon Europe** | Strong/high barrier | EUR 500K+ | File for later |
| **Filecoin/PL** | Moderate | $10-100K | Low priority |

---

## 5. Cross-Cutting Themes

Every proposal should hit 3-4 of these, tailored to the funder.

### Privacy-preserving data commons

TC's unique combination: client-side redaction + TEE scoring + blockchain settlement +
open source. No other system combines all four.

| Funder | Frame as |
|---|---|
| NSF | Novel architecture for privacy-preserving data commons |
| NLnet | User sovereignty over AI behavioral data |
| NEAR | NEAR-native privacy infrastructure for AI agents |
| Mozilla | Alternative to surveillance-based AI data collection |
| Open Phil | Privacy-preserving research infrastructure for AI safety |

### Regulatory compliance

EU AI Act is law (Aug 2, 2026). Article 12 mandates automatic recording for high-risk
AI systems. No open-source compliance infrastructure exists. TC satisfies the
requirement while preserving contributor sovereignty. Also aligns with NIST AI RMF 1.0
and Singapore IMDA AI Governance Framework.

### Agent ecosystem public goods

No competitor provides cross-organization trace sharing with quality scoring:

| Category | Capture | Cross-org | Incentives | Scoring |
|---|---|---|---|---|
| Observability (Langfuse et al.) | Yes | No | No | No |
| Data marketplaces (Vana, Ocean) | No | Yes | Partial | No |
| Agent frameworks (IronClaw) | Yes | No | No | No |
| Safety orgs (METR, AISI) | No | No | No | Partial |
| **TraceCommons** | **Yes** | **Yes** | **Yes** | **Yes** |

### Research grounding

Cite these to show TC is on the research frontier, not isolated:

- **Data valuation**: LoGra/LogIX (NeurIPS 2025), For-Value (ACL 2026)
- **Interoperability**: OTel GenAI semantic conventions (v1.42.0), Agent Skills (agentskills.io)
- **Safety/debugging**: AgentDebugX, AgenTracer-8B (ICLR 2026), TRAIL/Who&When (ICML 2025)
- **Trust/provenance**: VET verifiable execution traces, C2PA v2.3, SCITT RFC 9943
- **Composability**: Compound AI Systems Optimization (EMNLP 2025), LEGOMem (AAMAS 2026)

### Founder credibility

Zaki Manian: co-created Cosmos SDK (~$50B+ in blockchain value), designed and shipped
IBC (cross-chain interoperability protocol), built Sommelier (DeFi protocol with real TVL).
Demonstrated ability to design, ship, and maintain critical open-source infrastructure.

---

## 6. Practical Notes

### Timeline

| When | Action |
|---|---|
| **Aug 10-15** | Decide NSF PESOSE: do you have a PI? If no, deprioritize this cycle. |
| **Aug 15-Sep 1** | If pursuing NSF, all-out effort. Needs PI, metrics, citations, letters. |
| **Sep-Oct** | Prepare NLnet application (lightweight). |
| **Nov 3** | Submit NLnet. Begin NEAR DevHub application. |
| **Nov-Dec** | Investigate Mozilla and Open Phil. Reach out to program officers. |
| **2027 Q1** | If NSF deferred, prepare for next PESOSE with PI and stronger metrics. |

### PI question (NSF-specific)

- **Ideal**: faculty at a US university (AI safety, privacy engineering, or open-source
  ecosystems) as PI, Zaki as senior personnel.
- **Quick path**: reach out to LoGra/LogIX team at CMU --- natural fit if they want to
  see their data-valuation method applied to a real trace commons.
- PESOSE typically expects academic leadership; industry PIs are uncommon.

### Stacking strategy

Grants are not mutually exclusive. Ideal outcome:

- NLnet (EUR 48K): privacy and gate enhancements (European angle)
- NEAR ($120K): deeper NEAR integration and marketplace (ecosystem angle)
- NSF ($300K): community, governance, sustainability (research angle)

Total ~$500K over 2 years, funding different aspects with no double-dipping.
NLnet and NEAR grants serve as evidence of viability for the NSF application.

### Key references for applications

1. Choe et al. "What is Your Data Worth to GPT?" NeurIPS 2025. arXiv:2405.13954.
2. OpenTelemetry GenAI semantic conventions. v1.42.0 (June 2026).
3. Compound AI Systems Optimization survey. EMNLP 2025.
4. VET: Verifiable Execution Traces. arXiv:2512.15892.
5. AgentDebugX. arXiv:2607.18754.
6. AgenTracer-8B. ICLR 2026. arXiv:2509.03312.
7. EU AI Act, Regulation (EU) 2024/1689, Articles 12 and 50.
8. For-Value. ACL 2026. arXiv:2508.10180.
9. TRAIL / Who&When failure attribution. ICML 2025 Spotlight.
10. LEGOMem. AAMAS 2026. arXiv:2510.04851.

### TC stats for applications (Aug 2026)

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
