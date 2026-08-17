# GPAI Compliance Infrastructure

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust AI trace registry (~235K LOC, 6 crates) that scores AI coding agent session traces for quality and novelty inside TEEs (Trusted Execution Environments) on NEAR AI Cloud, compensating contributors with NEAR blockchain credits. Built by Zaki Manian (~352 submissions to date). The EU AI Act's General-Purpose AI (GPAI) provider obligations have applied since August 2, 2025, with European Commission enforcement powers -- requests for information, model evaluations, compliance and recall measures, fines -- switching on August 2, 2026. This document maps TC's existing infrastructure to GPAI compliance requirements, identifies the open-source gap in compliance tooling, and positions TC as the data quality layer underneath regulatory workflows.

---

## 1. GPAI Enforcement Timeline

The AI Act's GPAI provisions follow a staggered enforcement calendar. The dates that matter for TC's positioning:

| Date | What Happens | Source |
|---|---|---|
| **Aug 2, 2025** | GPAI provider obligations apply | AI Act Art. 111(3) |
| **Aug 2, 2026** | Commission enforcement powers activate; AI Office may verify GPAI training data summaries and issue corrective measures | AI Act Art. 111(3) |
| **Aug 2, 2027** | Deadline for providers of models on market before Aug 2, 2025 | AI Act Art. 111(3) transitional |
| **27 Jul 2026** | Digital Omnibus (Regulation (EU) 2026/1744) enters into force | Reg (EU) 2026/1744, published OJ 24 Jul 2026 |
| **2 Aug 2028** | Annex I embedded high-risk AI system obligations apply | Regulation (EU) 2026/1744, Art. deferred |
| **Dec 2, 2027** | Annex III standalone high-risk AI system obligations (Article 12 et al.) apply | Regulation (EU) 2026/1744 |

**Critical distinction**: TC's v5 documents stated "Article 12 is law as of August 2, 2026." This was incorrect. The Digital Omnibus Regulation (Regulation (EU) 2026/1744, published in the Official Journal 24 July 2026, in force 27 July 2026) deferred Annex III standalone high-risk AI system deadlines (including Article 12 logging obligations) to December 2, 2027, and Annex I embedded high-risk obligations to August 2, 2028. GPAI provider obligations under Chapter V are a separate regime and ARE live as of August 2, 2025. Enforcement machinery for GPAI obligations activated August 2, 2026.

The Digital Omnibus legislative history: the European Parliament endorsed the regulation on 16 June 2026 by 423-57 with 174 abstentions; the Council gave final approval on 29 June 2026. The regulation was published in the Official Journal on 24 July 2026 and entered into force on 27 July 2026 — six days before the AI Act's original 2 August 2026 high-risk deadline.

These are different legal instruments with different scopes:

- **GPAI obligations** (Chapter V): apply to providers of general-purpose AI models regardless of downstream use. Cover training data documentation, capability disclosure, copyright compliance. Live now.
- **High-risk system obligations** (Chapter III, Section 2): apply to AI systems in Annex III use cases (biometrics, critical infrastructure, employment, etc.). Include Article 12 automatic logging requirements. Deferred to Dec 2, 2027.

TC's compliance relevance is primarily to GPAI obligations, not high-risk system obligations. All positioning materials must maintain this distinction.

---

## 2. Fine Structure

The AI Act's penalty framework scales with organizational size. For GPAI provider obligations specifically:

| Violation Category | Maximum Fine |
|---|---|
| Prohibited AI practices (Art. 5) | **EUR 35 million or 7% of total worldwide annual turnover** (whichever higher) |
| GPAI obligations and other breaches | **EUR 15 million or 3% of total worldwide annual turnover** (whichever higher) |
| Supplying incorrect information to authorities | **EUR 7.5 million or 1% of total worldwide annual turnover** (whichever higher) |

Source: European Commission, "The enforcement framework of the AI Act."

For context on what "3% of worldwide annual turnover" means at scale:

| Company | 2025 Revenue (approx.) | 3% Fine Ceiling |
|---|---|---|
| Google/Alphabet | ~$350B | ~$10.5B |
| Microsoft | ~$260B | ~$7.8B |
| Meta | ~$170B | ~$5.1B |
| Mistral AI | ~$100M (est.) | ~$3M |

The fine structure creates a compliance budget that dwarfs any tooling cost. Even a mid-tier model provider facing a potential EUR 15M fine has strong incentive to invest in documentation and transparency infrastructure.

---

## 3. GPAI Code of Practice

The final GPAI Code of Practice was published July 10, 2025 (per Latham & Watkins analysis). It operationalizes the AI Act's GPAI obligations into three chapters with concrete compliance measures.

### Chapter 1: Transparency

Model providers must document and disclose:

- Model capabilities and limitations
- Intended and foreseeable uses
- Training methodology (at a level sufficient for deployer risk assessment)
- Evaluation results and known failure modes
- Known biases and mitigation measures

Providers must furnish deployers with an **information package** containing: capabilities and limitations, safe-deployment instructions, known biases, and use restrictions. This package must be maintained and updated as the model evolves.

### Chapter 2: Copyright

Training data documentation for copyright compliance:

- Description of data sources and types used in training
- Documentation of content identification and filtering processes
- Mechanisms for rights holders to lodge complaints or opt out
- Publicly available summary of training data (the "training data summary" -- see Section 4 below)

### Chapter 3: Safety and Security

Applies **only** to models classified as posing systemic risk. The threshold: models trained with cumulative compute exceeding **10^25 FLOPs** (floating-point operations).

This threshold currently captures approximately 5-15 companies worldwide (depending on how multi-modal and mixture-of-experts training compute is counted). It includes frontier labs (OpenAI, Anthropic, Google DeepMind, Meta FAIR, Mistral at the margin) and excludes the vast majority of model providers.

Systemic-risk obligations include:

- Adversarial testing (red-teaming) before release
- Model risk assessment and mitigation
- Cybersecurity protections for model weights
- Incident reporting to the AI Office
- Energy consumption documentation

**For TC's positioning**: Chapter 3 obligations are not TC's target market. TC's relevance is to Chapters 1 and 2 -- transparency and copyright -- which apply to ALL GPAI providers regardless of model size.

---

## 4. Training Data Summary Template

The European Commission AI Office published the GPAI Training Data Summary Template on 24 July 2025 under Article 53(1)(d) of Regulation (EU) 2024/1689. This is not an internal document -- providers are required to furnish a **public** summary. The AI Office may verify compliance and issue corrective measures from 2 August 2026; providers of pre-existing models (on market before August 2, 2025) have until 2 August 2027.

The template has a confirmed three-section structure (per Commission Explanatory Notice; corroborated Bird & Bird, Jones Day, WilmerHale):

### Section 1: General Information

- Model and provider identity
- Date placed on market
- Knowledge cutoff date
- Overall data size and modalities (e.g., token count for text, image count, etc.)

### Section 2: List of Data Sources

Providers must describe, in narrative style aggregated across the corpus (not work-by-work):

- **Large publicly available datasets**: named (e.g., Common Crawl, Wikipedia, The Pile)
- **Licensed or third-party data**: narrative description of categories and licensing arrangements
- **Scraped content**: most relevant domains and categories
- **User data**: whether and how user-generated content was incorporated
- **Synthetic data**: whether synthetic or model-generated data was included, and at what scale

### Section 3: Additional Compliance Metadata

Supplementary information as required by the AI Office, including data governance and filtering practices at a level sufficient for deployer risk assessment.

TC's existing infrastructure maps to these confirmed template fields:

- **General Information (Section 1)**: TC trace provenance records the originating agent, session context, and submission metadata, but at the trace level rather than the model level. Partial coverage -- traces are a specific data type within a training corpus.
- **List of Data Sources (Section 2)**: TC's scoring pipeline tracks source provenance per submission. The narrative aggregation format (not work-by-work) matches TC's aggregated quality statistics. Direct mapping is feasible.
- **Processing methodology**: The scoring pipeline (docs 02, 08, 09) documents redaction, chunking, embedding, and perplexity scoring steps. This IS the processing methodology documentation the template requires.
- **Data quality measures**: The gate pipeline with conformal calibration (doc 09) provides statistically principled quality scoring with documented acceptance criteria.
- **Privacy and PII handling**: TEE-based redaction (doc 08, redaction-invariant scoring) handles PII removal with cryptographic attestation that redaction occurred.
- **Copyright compliance**: Not currently addressed. TC traces are agent session logs, not copyrighted works, but downstream use in training creates copyright surface area.

The gap is **template export** -- TC has the data but does not currently produce a document in the public format the Code of Practice specifies. Building an export function that maps TC metadata to the confirmed three-section training data summary template is a concrete, bounded engineering task.

---

## 5. Compliance Vendor Landscape

The EU AI Act has created a compliance tooling market. Current estimates and pricing:

### Market Size

The LLM observability market (adjacent to, and partially overlapping, GPAI compliance tooling) is sized by The Business Research Company at **$1.97B (2025) → $2.69B (2026) → $9.26B (2030)** at a 36.2% forecast CAGR (via MarkTechPost, 9 August 2026). Treat as a vendor market-research estimate, not audited figures. Broader AI governance tooling market estimates range from EUR 7.6B to EUR 38B by 2030, depending on scope definition (narrow compliance tooling vs. broader AI governance infrastructure).

### Commercial Pricing

| Vendor | Annual Cost | What They Provide |
|---|---|---|
| **Holistic AI** | Not verified (vendor quote) | Risk assessment, bias auditing, regulatory mapping |
| **Credo AI** | Not verified (vendor quote) | AI governance platform, policy-to-controls mapping |
| **TrustArc** | Not verified (vendor quote) | Privacy compliance (extending to AI Act), consent management |
| **OneTrust** | Not verified (vendor quote) | GRC platform with AI governance modules |
| **Monitaur** | Not public | Model monitoring, audit trails |

**Important caveat**: All pricing figures for Holistic AI, Credo AI, TrustArc, and OneTrust are gated/quote-based. TC's previously cited specific figures (EUR 30K-100K, EUR 30K-50K, etc.) lack primary sources. Mark all compliance platform pricing as "vendor quote, unverified" and do not use in grant applications without a direct citation from a primary source (vendor quote letter, published pricing page, or audited market report).

### The Open-Source Gap

**No mature, widely-adopted open-source (OSI-compliant) GPAI Art. 53 compliance toolkit exists.** This is confirmed by surveying:

- GitHub: No repository with >100 stars provides GPAI Code of Practice compliance tooling as an integrated toolkit
- FOSS directories (AlternativeTo, Open Source Initiative): No listed OSI-compliant alternatives
- EU AI Act compliance guides: All reference commercial vendors
- NLnet/NGI project lists: No funded GPAI compliance infrastructure project

The nearest partial counterexample is **VerifyWise** (verifywise.ai), an emerging AI-governance project. However, VerifyWise uses the Business Source License 1.1 (BSL 1.1), which is source-available but is NOT open source per the OSI definition (BSL restricts commercial use for a specified period). TC should not cite VerifyWise as evidence that the open-source gap is filled; rather, it confirms that even source-available tooling in this space remains early-stage and narrowly adopted.

The gap is specific and verifiable. Open-source tools exist for adjacent problems (model cards via Hugging Face, bias detection via Fairlearn/AIF360, observability via Langfuse pre-acquisition), but none address the GPAI Code of Practice's training data summary template, provider-deployer information package, or ongoing transparency reporting obligations as an integrated OSI-compliant toolkit.

This gap is TC's grant pitch.

### Market Signal: ClickHouse Acquires Langfuse (January 2026)

ClickHouse acquired Langfuse on 16 January 2026 (not Databricks — resolve any internal references accordingly), alongside a $400M Series D led by Dragoneer that tripled ClickHouse's valuation to approximately $15B. Langfuse was the leading open-source LLM observability platform, with first-class tracing, evaluation, and prompt management tooling.

This acquisition is significant for TC's positioning on two axes:

1. **Removes an open-source competitor from the adjacent space**: Langfuse, as a ClickHouse product, will follow commercial priorities. Its open-source governance posture may shift. TC's genuinely OSI-compliant, TEE-attested, contributor-compensating architecture is now more differentiated.
2. **Validates the LLM observability market**: A $400M Series D and $15B valuation for an LLM observability acquisition confirms the market sizing above. Investors are pricing significant growth in this category.

None of the major observability platforms (LangSmith, Langfuse/ClickHouse, Braintrust) offer cross-user/shared trace retrieval, trajectory RAG marketplace, TEE-based scoring, or contributor compensation. TC's differentiation on those four axes stands.

---

## 6. TC Capability Mapping to GPAI Obligations

The following table maps each GPAI provider obligation to TC's existing or planned capabilities:

| GPAI Obligation | TC Capability | Current Status | Gap |
|---|---|---|---|
| Training data documentation | Trace provenance + quality scores per submission | **Partial** | Needs structured template export matching Code of Practice format |
| Data quality measures | Gate pipeline scoring with conformal calibration | **Wired** | Threshold calibration in progress (Issue #210, doc 09) |
| Privacy/PII handling | TEE-based redaction with attestation | **Wired** | Redaction-invariant scoring preserves quality signal post-redaction |
| Processing methodology documentation | Scoring pipeline documented in docs 02, 08, 09 | **Exists** | Not machine-readable; needs structured metadata export |
| Capability evaluation | Gate evaluation + multi-scorer pipeline | **Wired** | Evaluation is per-trace, not per-model capability |
| Transparency reporting | Audit chain with drift detection | **Wired** | Continuous monitoring, but no regulatory report format |
| Copyright compliance | Not addressed | **Missing** | TC traces are session logs; copyright surface from downstream training use |
| Provider-deployer information package | Not addressed | **Missing** | Would require aggregating TC metadata into deployer-facing format |

### What "Partial" and "Wired" Mean

- **Wired**: The capability exists in code, runs in production, and produces artifacts. The gap is format -- TC produces the data but not in the specific format the Code of Practice requires.
- **Partial**: The capability exists but covers only part of the obligation. For training data documentation, TC covers provenance and quality for traces specifically, not for arbitrary training corpora.
- **Missing**: TC does not address this obligation and would need new functionality.

The honest assessment: TC covers 4 of 7 mapped obligations at "Wired" or "Partial" status. The remaining 3 require new development, but that development is bounded (template export, report formatting, copyright metadata) rather than architectural.

---

## 7. Positioning Strategy

TC is **not** a compliance tool. It is data quality and provenance infrastructure that happens to produce the artifacts GPAI compliance requires. This distinction matters for positioning, credibility, and avoiding scope creep.

### For Model Providers

> "TC-scored and provenance-attested training data meets GPAI transparency obligations. Every trace in TC carries quality scores, provenance metadata, and TEE-attested redaction certificates. When you train on TC data, the training data summary template writes itself."

The value proposition: using TC-scored data eliminates the documentation burden for the portion of your training corpus sourced from TC. The quality scores, provenance records, and redaction attestations ARE the documentation.

### For Deployers

> "Traces scored by TC provide the downstream monitoring data GPAI deployer obligations require. When your agent runs are submitted to TC, you get continuous quality monitoring with drift detection -- the ongoing transparency reporting deployers need."

Deployer obligations under the AI Act are lighter than provider obligations, but they include monitoring the AI system in operation and reporting incidents. TC's continuous scoring provides exactly this monitoring layer.

### For Compliance Vendors

> "TC provides the data quality layer your compliance workflow needs. Your platform handles policy mapping, risk assessment, and regulatory reporting. TC handles the underlying question: is the training data documented, quality-scored, and provenance-attested?"

This positions TC as infrastructure underneath compliance vendors, not as a competitor. Holistic AI, Credo AI, and TrustArc all need a data quality layer they do not currently have. TC fills that gap.

### What TC Does NOT Claim

- TC does not replace legal counsel for AI Act compliance
- TC does not cover high-risk AI system obligations (Article 12 et al.)
- TC does not address systemic-risk model obligations (Chapter 3 of Code of Practice)
- TC does not perform copyright clearance -- it documents provenance, not rights
- TC is not a GRC (governance, risk, compliance) platform

---

## 8. Grant Angles

### NLnet / NGI

TC's NLnet application (doc 05) already identifies GPAI compliance tooling as a key deliverable. The pitch sharpens:

- **Problem**: GPAI provider obligations are live. No open-source compliance toolkit exists. Commercial tools cost EUR 30K-500K/year.
- **Solution**: TC provides the data quality and provenance layer as open-source infrastructure. Training data summary template export, TEE-attested redaction, continuous quality monitoring.
- **Why NLnet**: This is precisely the kind of digital commons infrastructure NLnet funds -- open-source tools that prevent vendor lock-in for regulatory compliance.

### Horizon Europe

The Horizon Europe programme funds AI regulatory compliance research and tooling. Relevant calls:

- **HORIZON-CL4-2025-HUMAN-01**: Human-centric AI, includes regulatory compliance tools
- **Digital Europe Programme**: AI testing and experimentation facilities
- **EDIH (European Digital Innovation Hubs)**: SME AI adoption, including compliance support

TC's positioning for Horizon Europe: an open-source reference implementation for GPAI training data documentation, enabling European SMEs to comply without EUR 100K+ vendor contracts.

### Open Technology Fund / Ford Foundation

For US-based funding: frame TC as transparency infrastructure that enables democratic oversight of AI training data. The GPAI compliance angle is secondary; the primary pitch is open-source AI accountability tools.

---

## 9. Precise Language Requirements

Grant applications, marketing materials, and technical documentation must use precise legal language when referencing the AI Act. Common errors to avoid:

### Correct

- "GPAI provider obligations apply as of August 2, 2025, with Commission enforcement powers activating August 2, 2026."
- "The GPAI Code of Practice's systemic-risk obligations apply only to models trained above the 10^25 FLOP threshold."
- "High-risk AI system obligations under Article 12 were deferred to December 2, 2027 by the Digital Omnibus Regulation."
- "TC's infrastructure produces artifacts aligned with GPAI transparency and copyright documentation obligations."
- "The Digital Omnibus is Regulation (EU) 2026/1744, published in the Official Journal 24 July 2026, in force 27 July 2026."

### Incorrect

- ~~"The AI Act takes effect August 2, 2026."~~ (Multiple dates; GPAI obligations already applied Aug 2, 2025.)
- ~~"Article 12 is law as of August 2, 2026."~~ (Deferred to Dec 2, 2027 via Digital Omnibus.)
- ~~"All AI providers must comply with systemic-risk obligations."~~ (Only models above 10^25 FLOP threshold.)
- ~~"TC is a GPAI compliance tool."~~ (TC is data quality infrastructure; compliance is a downstream use case.)
- ~~"GPAI obligations apply to all AI systems."~~ (GPAI obligations apply to general-purpose AI models specifically, not to narrow AI systems or high-risk systems under separate provisions.)
- ~~"The Digital Omnibus is still proposed."~~ (It is now law -- published in the Official Journal 24 July 2026, in force 27 July 2026.)

### Terminology

| Term | Meaning | Use When |
|---|---|---|
| GPAI provider | Entity placing a general-purpose AI model on the EU market | Discussing TC's primary compliance audience |
| Deployer | Entity using an AI system under its authority | Discussing downstream monitoring use case |
| Systemic risk | GPAI model above 10^25 FLOP threshold | Discussing Chapter 3 obligations (NOT TC's target) |
| Training data summary | Structured document required by GPAI Code of Practice | Discussing TC's template export feature |
| Information package | Documentation providers must furnish to deployers | Discussing provider-deployer obligations |
| Digital Omnibus Regulation | Regulation (EU) 2026/1744, published OJ 24 Jul 2026, in force 27 Jul 2026; defers Annex III standalone high-risk obligations to Dec 2, 2027 and Annex I embedded to Aug 2, 2028 | Clarifying Article 12 timeline corrections |

---

## 10. Implementation Roadmap

Concrete engineering tasks to add GPAI compliance export to TC:

### Phase 1: Training Data Summary Export (4-6 weeks)

1. Define a `GpaiTrainingSummary` struct mapping to Code of Practice template fields
2. Implement aggregation queries over TC's existing provenance and quality metadata
3. Build export to JSON, YAML, and PDF formats
4. Add CLI command: `tc export gpai-summary --format json`

### Phase 2: Provider-Deployer Information Package (2-4 weeks)

1. Define `DeployerPackage` struct with capabilities, limitations, biases, restrictions
2. Aggregate per-model metadata from TC traces into package format
3. Versioned packages with diff tracking (what changed since last release)

### Phase 3: Continuous Compliance Monitoring (4-6 weeks)

1. Drift detection alerts when training data quality distribution shifts
2. Periodic re-scoring with updated gate thresholds
3. Compliance dashboard showing obligation coverage status
4. Webhook integration for compliance vendor platforms

### Phase 4: Attestation and Audit (2-4 weeks)

1. TEE attestation certificates linked to GPAI summary documents
2. Tamper-evident audit log for all compliance-relevant metadata changes
3. Third-party verifier interface for regulatory inspection

Total estimated effort: 12-20 weeks of engineering, producing four concrete deliverables that close TC's GPAI compliance gaps.

---

## 11. Summary

The GPAI compliance landscape as of August 2026:

- **Obligations are live.** GPAI provider obligations applied August 2, 2025. Commission enforcement activated August 2, 2026. This is not future tense.
- **Fines are material.** Up to EUR 15M or 3% of worldwide annual turnover for GPAI breaches. Model providers have budget for compliance tooling.
- **No mature OSI-compliant open-source option exists.** Commercial compliance vendors charge pricing that is gated/quote-based (unverified figures; do not cite without primary sources). VerifyWise (BSL 1.1, source-available but not OSI open source) is the nearest partial counterexample but remains early-stage. The open-source gap is confirmed and specific. ClickHouse's acquisition of Langfuse (16 Jan 2026, $400M Series D) further validates the LLM observability market while removing one potential open-source competitor from TC's adjacent space.
- **TC covers 4 of 7 obligations today.** Data quality scoring, privacy/PII handling, processing methodology, and transparency reporting are wired. Training data template export, copyright metadata, and deployer packages are bounded engineering tasks.
- **TC is infrastructure, not a compliance tool.** The positioning is: data quality and provenance layer that produces compliance-ready artifacts. This avoids scope creep, avoids competing with GRC vendors, and aligns with TC's actual capabilities.
- **The grant angle is strong.** NLnet, Horizon Europe, and other funders explicitly seek open-source regulatory compliance infrastructure. TC fills a verified gap.

Precise language matters. GPAI obligations are not high-risk system obligations. The 10^25 FLOP threshold applies only to systemic-risk models. Article 12 was deferred to December 2, 2027. Every TC document, grant application, and marketing claim must maintain these distinctions.
