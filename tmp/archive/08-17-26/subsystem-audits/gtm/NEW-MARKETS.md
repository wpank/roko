# New Markets & Novel Integrations

24 integration categories that unlock user segments, workflows, and revenue streams beyond
the core developer-agent loop. Each category includes market sizing, specific integrations,
roko's unique differentiator, and competitive positioning in the April 2026 landscape.

Last updated: 2026-04-29.

---

## Why "Beyond Code Generation" Is Roko's Strategic Wedge

The AI coding assistant market ($6.8B in 2025) has crystallized: 7 companies have crossed
$100M ARR, the top 3 capture 70%+ market share, and valuations have reached $29-60B. Roko
cannot win by being a better Cursor or a better Claude Code. It wins by being the agent
orchestration platform that serves workflows no coding assistant touches.

Roko's architecture -- 18 crates, adapter-trait composition, 7-rung gate pipeline, 4
compounding learning loops -- is domain-agnostic. The same plan -> execute -> gate -> learn
loop that develops software can also audit security posture, manage infrastructure, process
compliance checklists, triage customer support, and orchestrate data pipelines. Every use
case below uses the same engine with different role templates, tool sets, and gate rungs.

---

## Category 1: Infrastructure / DevOps / SRE Self-Healing

**Market**: AIOps market $3B+ (2026), projected $34B+ by 2030. 57% of organizations have
deployed multi-step agent workflows in production.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| PagerDuty | Alert -> auto-diagnosis -> fix PR -> deploy | `AlertSource` |
| Datadog | Anomaly detection -> auto-scaling / fix | `MonitoringAdapter` |
| Terraform | Infrastructure-as-code changes via agents | `InfraAdapter` |
| Kubernetes API | Pod health -> restart / rollback / scale | `OrchestrationAdapter` |
| AWS CloudWatch | Metric alarms -> automated remediation | `AlertSource` |

**Roko's unique differentiator**: Other AIOps tools generate fixes but do not verify them.
Roko's gate pipeline validates every infrastructure change before applying it. The agent
cannot `kubectl apply` without passing compile (syntax check), test (dry-run), and review
(LLM judge) gates.

**Competitive landscape**: PagerDuty has basic AI triage. Datadog has AI-powered anomaly
detection. Neither has an agent that generates and verifies fixes autonomously. Shoreline.io
(auto-remediation) was acquired by Datadog but operates at the script level, not the
agent-orchestration level.

**Unlock**: Roko moves from "development tool" to "operations tool." Same architecture,
different role configs and tool sets.

---

## Category 2: Security Operations & Vulnerability Management

**Market**: Application security testing market $15B+ by 2028. Post-Shai-Hulud demand (npm
worm Sep-Nov 2025, Bitwarden CLI attack Apr 22-27 2026) has turned supply-chain security
from nice-to-have to board-level priority.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| Semgrep | SAST scanning with SARIF output | `SecurityScanner` |
| Snyk | Dependency vulnerability scanning | `SecurityScanner` |
| CodeQL | GitHub Advanced Security analysis | `SecurityScanner` |
| Wiz | Cloud security posture management | `SecurityScanner` |
| Sigstore/in-toto | Agent-action-time verification | `AuditAdapter` |

**Roko's unique differentiator**: Security scanners find problems. Roko's agents fix them,
then the gate pipeline re-scans to verify the fix does not introduce new issues. The
closed loop -- scan -> agent fix -> re-scan -> gate pass -- does not exist in any competing
product.

**Beyond code**: The same pattern applies to infrastructure security (Terraform
misconfigurations), container security (Dockerfile best practices), and secrets management
(detected -> rotated -> verified).

**Revenue signal**: Sigstore has 101M+ Rekor entries, 33K+ projects. Baking Sigstore/SLSA L3
into roko's release pipeline is the cheapest enterprise-procurement unblocker.

---

## Category 3: Compliance & Audit Automation

**Market**: GRC (Governance, Risk, Compliance) software market $50B+ by 2028. EU AI Act
Article 50 enforcement begins August 2, 2026.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| Vanta | SOC 2 / ISO 27001 evidence collection | `ComplianceAdapter` |
| Drata | Continuous compliance monitoring | `ComplianceAdapter` |
| EU AI Act registry | Article 50 transparency reporting | `RegulatoryAdapter` |
| NIST AI RMF | Risk management framework checklists | `ComplianceAdapter` |

**Roko's unique differentiator**: Vanta ($220M ARR, $4.15B valuation) and Drata automate
evidence collection for human-driven processes. Roko's gate pipeline produces machine-readable
compliance artifacts for agent-driven processes. Every agent action has a signed gate result
that serves as audit evidence.

**The Vanta playbook**: Vanta built $220M ARR on SOC 2 enforcement timing. OneTrust built
$5.3B on GDPR timing. Article 50 enforcement (Aug 2, 2026) puts roko in the equivalent
build window.

---

## Category 4: Data Engineering & Pipeline Orchestration

**Market**: Data engineering tools market $25B+ by 2027. dbt Labs: $4.2B valuation (2022).

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| dbt | Data transformation model generation | `DataPipelineAdapter` |
| Airflow | DAG-based workflow orchestration | `WorkflowAdapter` |
| Dagster | Asset-based pipeline orchestration | `WorkflowAdapter` |
| Snowflake | Data warehouse schema management | `DatabaseAdapter` |
| BigQuery | Analytics query optimization | `DatabaseAdapter` |

**Roko's unique differentiator**: dbt models are code. Roko's agent can generate, test, and
gate dbt models with the same pipeline used for application code. Schema changes go through
compile (syntax), test (dbt test), and review (data quality checks) gates.

**Novel workflow**: Schema drift detection via Postgres logical replication (`pgwire-
replication` 0.2) -> agent generates migration -> gates validate -> deploys. No competing
product does schema-drift-to-fix autonomously.

---

## Category 5: Customer Support & Ticket Resolution

**Market**: AI in customer service $20B+ by 2028. 68% of organizations already use AI for
customer interactions.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| Zendesk | Ticket triage and auto-response | `TicketSource` |
| Intercom | Conversation resolution | `TicketSource` |
| Freshdesk | Multi-channel support automation | `TicketSource` |
| Confluence | Knowledge base for agent context | `ContextProvider` |

**Roko's unique differentiator**: Existing AI support tools (Zendesk AI, Intercom Fin)
generate responses. Roko can resolve tickets that require code changes: bug report -> diagnose
-> fix -> gate -> deploy -> resolve ticket. The gate pipeline prevents deploying a broken fix
to production.

---

## Category 6: Document & Content Generation

**Market**: AI content generation $15B+ by 2027.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| Notion | Document generation and management | `DocumentAdapter` |
| Google Docs | Collaborative document creation | `DocumentAdapter` |
| Confluence | Technical documentation generation | `DocumentAdapter` |
| Storybook | UI component documentation | `DocumentAdapter` |

**Roko's unique differentiator**: Content generation is not just LLM output. Roko's gate
pipeline can verify factual claims (gate rung: fact-check), check style guides (gate rung:
linting), and validate links (gate rung: link-check). Verified documentation is a category
no content AI addresses.

---

## Category 7: Financial Services & RegTech

**Market**: RegTech market $19B by 2028. Financial institutions face 70+ regulatory changes
per day on average.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| Bloomberg Terminal | Market data for quantitative analysis | `DataSource` |
| Plaid | Financial data integration | `DataSource` |
| Compliance.ai | Regulatory change tracking | `RegulatoryAdapter` |
| DTCC | Trade reporting and settlement | `ReportingAdapter` |

**Roko's unique differentiator**: Financial services require audit trails for every decision.
Roko's gate pipeline + episode logging produces the evidence trail that compliance officers
need. No agent framework provides this natively.

**Revenue signal**: Goldman Sachs, Citi, and Nubank are listed as Devin customers. The
willingness to pay for agent tooling in finance is proven. Roko's open-source, on-premises
deployment model is a procurement advantage for regulated industries that cannot use SaaS.

---

## Category 8: Healthcare & Life Sciences

**Market**: AI in healthcare $45B+ by 2028. Highly regulated, requires audit trails.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| FHIR APIs | Healthcare data interoperability | `HealthDataAdapter` |
| Epic/Cerner | EHR system integration | `HealthDataAdapter` |
| FDA 21 CFR Part 11 | Electronic records compliance | `ComplianceAdapter` |
| Clinical trial registries | Trial data management | `DataSource` |

**Roko's unique differentiator**: Healthcare AI requires explainability and audit trails.
Roko's gate pipeline provides verification evidence for every agent decision. The episode
log creates a complete provenance chain from input to output.

**Deployment model**: On-premises deployment is a hard requirement for HIPAA compliance.
Roko's self-hosted architecture is a structural advantage over SaaS-only competitors.

---

## Category 9: Legal Tech & Contract Analysis

**Market**: Legal AI market $4B+ by 2027.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| Clio | Legal practice management | `WorkSource` |
| Relativity | eDiscovery document review | `DocumentAdapter` |
| DocuSign | Contract execution and tracking | `DocumentAdapter` |
| LegalSifter | Contract analysis and extraction | `AnalysisAdapter` |

**Roko's unique differentiator**: Legal work requires confidence levels and citation trails.
Roko's gate pipeline can include domain-specific verification rungs (jurisdiction check,
citation verification, conflict-of-interest scan) that generic LLM wrappers cannot.

---

## Category 10: Education & Training Content

**Market**: EdTech AI market $10B+ by 2028.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| Canvas LMS | Course content management | `ContentAdapter` |
| Coursera/Udemy | Online course creation | `ContentAdapter` |
| Assessment platforms | Quiz and exam generation | `AssessmentAdapter` |

**Roko's unique differentiator**: Educational content requires pedagogical accuracy and
difficulty calibration. Gate rungs for fact-checking, difficulty assessment, and learning
objective alignment. The knowledge store retains what works across course iterations.

---

## Category 11: IoT & Edge Computing

**Market**: IoT platform market $25B+ by 2028.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| AWS IoT Core | Device management and data processing | `IoTAdapter` |
| Azure IoT Hub | Device-to-cloud messaging | `IoTAdapter` |
| Balena | Edge device deployment | `DeployAdapter` |
| Home Assistant | Home automation orchestration | `AutomationAdapter` |

**Roko's unique differentiator**: IoT firmware updates require verification before
deployment to prevent bricking devices. Roko's gate pipeline can validate firmware changes
in emulation before pushing to devices.

---

## Category 12: Research & Scientific Computing

**Market**: Research AI tools $5B+ by 2027.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| arXiv API | Paper search and analysis | `ResearchSource` |
| Semantic Scholar | Citation graph traversal | `ResearchSource` |
| Jupyter | Notebook generation and execution | `ComputeAdapter` |
| Weights & Biases | Experiment tracking | `MLAdapter` |

**Roko's unique differentiator**: Research requires reproducibility. Roko's content-
addressable storage (CAS) pattern ensures that identical inputs produce verifiable identical
outputs. The episode log creates a complete experimental record.

---

## Category 13: Supply Chain & Logistics

**Market**: Supply chain AI $12B+ by 2028.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| SAP | Enterprise resource planning | `ERPAdapter` |
| Oracle SCM | Supply chain management | `ERPAdapter` |
| ShipStation | Shipping and fulfillment | `LogisticsAdapter` |

**Roko's unique differentiator**: Supply chain decisions require multi-factor optimization
(cost, time, reliability) with audit trails. The CascadeRouter's bandit-based selection
naturally extends to supplier/route optimization.

---

## Category 14: Real Estate & Property Tech

**Market**: PropTech AI $3B+ by 2027.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| MLS APIs | Property listing management | `DataSource` |
| Yardi | Property management | `PropertyAdapter` |
| DocuSign | Lease and contract management | `DocumentAdapter` |

---

## Category 15: Marketing & Growth

**Market**: AI in marketing $40B+ by 2028.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| HubSpot | CRM and marketing automation | `CRMAdapter` |
| Mailchimp/Loops | Email campaign management | `CampaignAdapter` |
| Google Analytics | Website analytics and optimization | `AnalyticsAdapter` |
| PostHog | Product analytics | `AnalyticsAdapter` |

**Roko's unique differentiator**: Marketing automation generates content but does not verify
its effectiveness before sending. Roko's gate pipeline can include A/B test gates, brand
consistency checks, and compliance gates (CAN-SPAM, GDPR) before deployment.

---

## Category 16: Game Development

**Market**: Game AI tools $2B+ by 2027. Bevy (Rust game engine) ecosystem growing fast.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| Unity | Game asset and script generation | `GameDevAdapter` |
| Bevy | Rust-native game development | `GameDevAdapter` |
| Godot | Open-source game engine | `GameDevAdapter` |

**Roko's unique differentiator**: Bevy uses the same Rust trait-based plugin architecture
that roko's adapter system is modeled on. The `#[derive(RokoAdapter)]` macro is inspired by
Bevy's Plugin trait. Natural ecosystem affinity.

---

## Category 17: Embedded Systems & Firmware

**Market**: Embedded AI $15B+ by 2028. Ferrocene (Ferrous Systems) validates Rust for
safety-critical embedded.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| Embassy | Rust async embedded framework | `EmbeddedAdapter` |
| probe-rs | Debug probe interface | `DebugAdapter` |
| defmt | Efficient logging for embedded | `LogAdapter` |

**Roko's unique differentiator**: Embedded firmware changes are high-stakes (bricked devices,
safety implications). The gate pipeline provides the verification layer that no embedded
development tool currently offers at the agent level.

**Berlin connection**: Ferrous Systems (Ferrocene, IEC 61508 SIL 2) shares office space with
Will at Wallstr. 59. Co-presenting at EuroRust October 14-17 is structurally natural.

---

## Category 18: Blockchain & Web3

**Market**: Web3 developer tools $5B+ by 2028.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| Foundry | Smart contract development and testing | `Web3Adapter` |
| Hardhat | Ethereum development environment | `Web3Adapter` |
| ERC-8004 | Agent identity and reputation | `IdentityAdapter` |
| x402/USDC | Autonomous agent payments | `PaymentGateway` |

**Roko's unique differentiator**: Smart contract bugs are irreversible and high-value targets.
The gate pipeline can include formal verification (gate rung: SMTChecker), fuzz testing (gate
rung: Echidna), and security audit (gate rung: Slither) before deployment. No competing agent
framework integrates blockchain-specific verification.

---

## Category 19: Robotics & Physical AI

**Market**: Robotics AI $25B+ by 2028.

**Roko's unique differentiator**: Robotic systems require simulation-before-execution. Roko's
digital twins pattern (ADVANCED-PATTERNS.md #8) maps naturally to robot simulation: plan ->
simulate -> gate (verify in simulation) -> execute on hardware.

---

## Category 20: Media & Entertainment

**Market**: AI in media $8B+ by 2027.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| Runway ML | Video generation and editing | `MediaAdapter` |
| ElevenLabs | Voice synthesis | `MediaAdapter` |
| Canva | Design generation | `DesignAdapter` |
| Figma | UI/UX design | `DesignAdapter` (MCP native) |

---

## Category 21: Agriculture & Environmental

**Market**: AgTech AI $5B+ by 2028.

**Roko's unique differentiator**: Agricultural decisions (irrigation, pest management,
planting) require verification against weather data, soil analysis, and crop models. The
gate pipeline can validate recommendations before expensive field operations.

---

## Category 22: Government & Public Sector

**Market**: GovTech AI $15B+ by 2028. Sovereign cloud requirements are strict.

**Roko's unique differentiator**: On-premises, open-source, Berlin-built (EU jurisdiction),
CRA-aligned. No US-cloud control plane required. This is the structural advantage for
government procurement that SaaS competitors cannot match.

---

## Category 23: Telecommunications

**Market**: AI in telecom $10B+ by 2028.

**Roko's unique differentiator**: Network configuration changes require rollback capability.
The gate pipeline validates config changes in test environments. The event sourcing pattern
enables configuration rollback to any previous state.

---

## Category 24: Workflow Automation (No-Code/Low-Code)

**Market**: Workflow automation $15B+ by 2027. n8n has 6,234 nodes, Zapier has 7,000+
integrations.

**Key integrations**:

| Integration | What It Does | Adapter |
|---|---|---|
| n8n | Workflow automation (self-hosted) | `WorkflowAdapter` |
| Zapier | Cloud workflow automation | `WorkflowAdapter` |
| Make (Integromat) | Visual workflow builder | `WorkflowAdapter` |
| Retool | Internal tool builder | `AppBuilder` |

**Roko's unique differentiator**: Workflow automation tools chain steps but do not verify
outputs. n8n's top templates are trigger -> enrich -> write+notify (3-system minimum).
Roko adds verification between each step: trigger -> enrich -> **gate** -> write -> **gate**
-> notify. The gate pipeline makes automated workflows safe to run unattended.

---

## Market Prioritization Matrix

| Category | Market Size | Pull Strength | Integration Effort | Gate Pipeline Value | Priority |
|---|---|---|---|---|---|
| 1. DevOps/SRE | $3B+ (2026) | Strong | Medium | Very High | **P0** |
| 2. Security Ops | $15B+ (2028) | Very Strong (post-Shai-Hulud) | Low | Very High | **P0** |
| 3. Compliance/Audit | $50B+ (2028) | Strong (Article 50) | Medium | Very High | **P0** |
| 4. Data Engineering | $25B+ (2027) | Medium | Medium | High | **P1** |
| 5. Customer Support | $20B+ (2028) | Medium | Medium | Medium | **P2** |
| 6. Document/Content | $15B+ (2027) | Low-Medium | Low | Medium | **P2** |
| 7. Financial Services | $19B+ (2028) | Strong | High | Very High | **P1** |
| 8. Healthcare | $45B+ (2028) | Medium | Very High | Very High | **P2** |
| 17. Embedded/Firmware | $15B+ (2028) | Medium | Medium | Very High | **P1** |
| 24. Workflow Automation | $15B+ (2027) | Strong | Low | High | **P1** |

**P0 categories** share a pattern: the gate pipeline provides uniquely high value because
verification is either regulatorily required (compliance), security-critical (security ops),
or operationally essential (infrastructure changes). These are markets where "generate and
hope" is not acceptable.

---

## The Common Thread: Verified Agent Output

Across all 24 categories, roko's differentiator is consistent: **verified agent output**.

- Coding assistants (Cursor, Codex, Claude Code) generate code and hope for the best
- Agent frameworks (LangGraph, CrewAI, AutoGen) orchestrate agents without verification
- Roko generates, verifies, learns, and improves

This is not a feature. It is a category. Every market listed above has the same structural
need: autonomous agents that produce trustworthy output. The gate pipeline is the horizontal
platform; the vertical markets are the applications.

---

## Sources

- AI coding market: $6.8B in 2025, $8.5B projected 2026 (multiple research firms)
- AIOps: Gartner, IDC market projections
- Security: Sigstore 101M+ Rekor entries, Shai-Hulud incidents
- Compliance: EU AI Act Article 50 enforcement August 2, 2026
- Vanta: $220M ARR, $4.15B valuation (TechCrunch July 2025)
- dbt Labs: $4.2B valuation (2022)
- n8n: 9,487 templates, 6,234 nodes, 3-system minimum pattern
- Devin customers: Goldman Sachs, Citi, Nubank (Cognition blog)
- Ferrocene: IEC 61508 SIL 2 (Dec 2025), Wallstr. 59 Berlin
- JetBrains: 76%+ developers using AI tools (2026 survey)
