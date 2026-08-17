# Verified Skills Backed by Execution Provenance

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source, Rust-based, privacy-preserving registry of AI coding
agent session traces (~235K LOC, 6 crates, MIT/Apache-2.0). Contributors submit scrubbed
traces; TC scores quality and novelty inside TEEs (Trusted Execution Environments --
hardware-isolated encrypted compute enclaves); contributors earn NEAR blockchain credits.
~352 submissions, 3 contributors. The Agent Skills ecosystem has grown to 490K+ skills
across registries with 32+ adopters (Claude Code, Codex, Cursor, Gemini CLI, Windsurf,
etc.), but the ClawHavoc attack -- in which a single operation planted hundreds of malicious
skills across public registries -- exposed that static analysis alone is insufficient for
skill trust. This document defines TC's differentiated wedge: verified skills backed by
multi-session, population-scale, TEE-attested behavioral provenance.

---

## 1. The Gap: Static Analysis Misses Up to 89% of Malicious Skills

Three independent 2026 benchmarks converge on the same conclusion: static analysis and
semantic/signature matching cannot reliably detect malicious agent skills. Runtime behavioral
analysis is the necessary complement.

### 1.1 SkillVetBench (arXiv:2606.00925)

Semantic-only and signature-only baselines miss **up to 89% of malicious skills**. Sandbox
execution-trace verification is the necessary complementary layer -- running the skill in a
controlled environment catches delayed activation, context-dependent exfiltration, and
multi-step privilege escalation that no amount of code reading can detect. TC already
collects execution traces; the gap is connecting them to skill identity.

### 1.2 MalSkillBench (arXiv:2606.07131)

Captures **syscall traces** (`strace -f`, `inotifywait`) over the agent process tree. OS-level
behavioral analysis catches filesystem access patterns, unexpected network connections, and
process spawning regardless of code obfuscation. TC's trace envelope captures tool-call
sequences -- a higher-level analog applying the same behavioral-analysis-over-execution
principle.

### 1.3 ClawGuard (arXiv:2604.11790)

Runtime security framework monitoring execution behavior, not code structure. Provides
sandboxing, behavior monitoring, and anomaly detection during skill execution. Operates
per-session; TC operates across sessions and across the population, providing complementary
longitudinal analysis.

### 1.4 SkillSieve -- CORRECTED (arXiv:2604.06550)

**v6 correction**: Previous TC documents (03, 05, 07) recorded SkillSieve F1 as **0.920**.
This is the full three-layer pipeline (static + SSD-augmented + LLM reasoning). The
SSD-augmented Layer-2 ablation achieves:

| Metric | Value |
|---|---|
| F1 | **0.800** |
| Precision | 0.752 |
| Recall | 0.854 |

Evaluated on a 400-skill benchmark at $0.006/skill. The 0.752 precision means roughly 1 in 4
skills flagged as safe may be malicious. At 490K+ skills, this is tens of thousands of
potential misclassifications. Neither the 0.800 nor 0.920 figure is sufficient for a
security-critical trust registry.

### 1.5 SkillFortify

Formal verification: **~96.95% F1 with zero false positives** -- strongest precision in the
literature. But operates only on executable code, not SKILL.md prose. A skill safe at the
code level can still contain malicious natural-language instructions that redirect agent
behavior at runtime. Complementary to TC's behavioral analysis on actual execution outcomes.

### 1.6 NVIDIA 8-Stage Pipeline

**162 signed skills** through an 8-stage verification pipeline (static analysis, sandboxed
execution, manual review, cryptographic signing). Enterprise gold standard. Does not scale
to 490K+ skills (manual review bottleneck) and cannot detect behavioral drift after signing.

### 1.7 OWASP AST10

Published February 2026. First standardized threat taxonomy for agent security: prompt
injection, tool misuse, data exfiltration, privilege escalation, supply chain attacks.
Shared vocabulary but no enforcement mechanism or registry.

### 1.8 SIGIL

On-chain registry concept for skill provenance -- cryptographic attestation of authorship,
version history, and audit trail. Answers "who published this and when?" but not "what does
it actually do?" Complementary to TC's behavioral model; a verified skill would carry both
SIGIL-style provenance and TC behavioral attestation.

---

## 2. ClawHavoc: Scale of the Problem

Multiple sources report conflicting counts, reflecting different scopes and methodologies:

| Source | Scope | Finding |
|---|---|---|
| **Koi Security** | 2,857 skills analyzed | **341 malicious** from a single coordinated operation |
| **Scandar** | ~300K users affected | **1,184 malicious** across registries |
| **Snyk audit** | 3,984 skills audited | **1,467 (36.82%)** with at least one security issue |

Koi Security's 341 represents one confirmed operation. Scandar's 1,184 is a broader sweep.
Snyk's 36.82% includes unintentional vulnerabilities, not just deliberate malice. The true
deliberately-malicious count is in the range **341-1,467** from these audits alone -- covering
a small fraction of the 490K+ ecosystem.

**Attack patterns**: prompt injection via SKILL.md prose, code execution through tool-call
redirection, data exfiltration via side channels, delayed activation (benign for N
invocations then malicious), privilege escalation through multi-step tool chaining.

**Scanner bypass**: Trail of Bits bypassed all existing scanners (SkillSieve, SkillSpector)
in under 1 hour, exploiting the fundamental limitation: static analysis examines what the
code says, not what it does.

---

## 3. TC's Differentiated Wedge

No existing system combines multi-session behavioral analysis, population-scale pattern
detection, and TEE-attested provenance:

| System | Static | Runtime | Multi-Session | Population-Scale | TEE-Attested |
|---|---|---|---|---|---|
| SkillSieve | Yes | No | No | No | No |
| SkillVetBench | Yes | Sandbox (single) | No | No | No |
| MalSkillBench | No | Syscall traces | No | No | No |
| ClawGuard | Partial | Yes (per-session) | No | No | No |
| SkillFortify | Formal verif. | No | No | No | No |
| NVIDIA 8-Stage | Yes | Partial (sandbox) | No | No | Partial (signing) |
| SIGIL | No | No | No | No | On-chain provenance |
| SkillOS | No | Composite reward | No (single stream) | No | No |
| SkillRevise | No | Fixed verifier | No | No | No |
| **TraceCommons** | Via partners | Via trace corpus | **Yes** | **Yes** | **Yes** |

**Multi-session behavioral analysis.** A skill that behaves well in a sandbox but exfiltrates
data on the 47th invocation is invisible to every system above. TC sees it across the 1st,
10th, 47th, and 200th invocation from different contributors. Behavioral drift is detectable
only with longitudinal, cross-session data.

**Population-scale pattern detection.** A skill targeting specific organizations or activating
only when certain tools are available is invisible to per-user analysis. TC's cross-contributor
corpus reveals population-level patterns via statistical anomaly detection.

**TEE-attested behavioral reports.** Analysis runs inside TEEs. The output is a
cryptographically attested report that external consumers verify without trusting TC's API.

**Why partners handle static analysis.** TC does not build another static analyzer. SkillSieve,
SkillSpector, SkillFortify, and NVIDIA's pipeline already exist. TC's value is orthogonal:
behavioral analysis from actual execution traces. The verified skills tier combines partner
static analysis (what the code says) with TC behavioral attestation (what the code does).

---

## 4. Architecture: Verified Skills Tier

### 4.1 Verification Flow

```
1. Skill publisher submits SKILL.md + references TC trace IDs
         |
2. TC matches traces to skill invocations
   (tool-call patterns, skill name, temporal correlation)
         |
3. Cross-session behavioral analysis
   - Behavior variance across invocations?
   - Permission escalation over time?
   - Context-dependent resource access?
   - Delayed-activation patterns?
   - Anomaly detection against population baseline
         |
4. TEE-attested behavioral report
   (invocation count, consistency score, anomaly flags,
    permission profile, resource access pattern)
         |
5. "Verified by TraceCommons" badge with attestation chain
```

### 4.2 Trace-to-Skill Matching

**Phase 1 (manual)**: Publisher provides trace IDs. TC verifies tool-call sequences match
the skill's declared capabilities. Human review confirms.

**Phase 2 (automated)**: SKILL.md declares capabilities (tools, inputs/outputs, sequences).
TC's `ToolCallEvent` sequences are pattern-matched against declared capabilities automatically.

**Phase 3 (population-scale)**: Retroactive matching across the entire corpus. New skill
submissions trigger historical search for matching invocations without publisher-provided IDs.

### 4.3 Behavioral Consistency Scoring

For a skill with N matched invocations:

- **Permission profile stability**: Escalation across invocations is a strong anomaly signal
- **Resource access pattern**: Unexpected resource access in a subset of invocations suggests
  context-dependent malicious behavior
- **Output distribution**: Bimodal outputs (e.g., formats code 99% of the time, exfiltrates
  data 1%) indicate compromise
- **Temporal drift**: Behavioral change over time signals either an update or delayed trigger

### 4.4 Badge Semantics

The badge is not binary safe/unsafe -- it is an attested behavioral summary:

| Field | Meaning |
|---|---|
| **Invocation count** | Times observed in TC's corpus |
| **Behavioral consistency** | Score (0-1) measuring variance across invocations |
| **Permission profile** | Permissions observed across all invocations |
| **Anomaly flags** | Specific anomalies detected (if any) |
| **Attestation hash** | TEE attestation chain root |
| **Verification date** | When analysis was performed |
| **Expiration** | Re-verification deadline (90 days or SKILL.md hash change) |

Consumers set thresholds per risk tolerance: enterprise might require consistency > 0.95
with 100+ invocations; a hobbyist might accept 0.80 with 10+.

---

## 5. Implementation Priority

### Phase 1: Manual Curation (1-2 weeks)

- `tc skill verify` CLI: submit SKILL.md + explicit trace IDs
- Validate trace-skill correspondence (tool-call overlap, manual review)
- Basic behavioral consistency check across submitted traces
- TEE-attested report and badge generation with attestation chain
- **Deliverable**: First "Verified by TraceCommons" badges on 5-10 curated skills

### Phase 2: Automated Matching (1-2 months)

- Tool-call pattern matching: SKILL.md capabilities mapped to `ToolCallEvent` sequences
- Behavioral consistency scoring: permissions, resource access, output distributions
- Statistical anomaly detection in per-skill behavioral distributions
- Re-verification automation at 90-day cadence
- **Deliverable**: Automated pipeline processing 50+ skills/week

### Phase 3: Population-Scale Anomaly Detection (3-6 months)

- Retroactive corpus search for historical invocations
- Cross-contributor behavioral comparison (per-user behavioral divergence)
- Temporal drift detection across skill versions
- Partner static analysis integration (SkillSieve, SkillFortify, NVIDIA pipeline)
- **Deliverable**: Full population-scale verified skills tier

---

## 6. Integration with Existing TC Architecture

**Scoring pipeline.** Behavioral consistency scoring reuses TC's gate pipeline: per-invocation
trace fragments embedded via the `Embedder` trait (consistency = clustering tightness);
process mining (doc 02, B.3) extended per-skill for drift detection; sub-trace decomposition
(doc 02, B.6) isolates skill-specific segments.

**Skill extraction pipeline.** Doc 03 (section 3) defines automated extraction from traces
(RHO, Trace2Skill, AutoRefine, SkillAudit). The verified skills tier is the quality layer
on top:

```
Trace corpus -> Skill extraction (RHO/Trace2Skill) -> Security scan (SkillAudit)
  -> Behavioral verification (this document) -> Badge -> Publication to registries
```

**Credit attribution.** When a verified skill earns adoption, credit flows to contributing
trace authors via the VCG mechanism (doc 02, C.7). Contributors whose traces provided
behavioral evidence receive proportional credit.

---

## 6A. Quality Architecture: TC as the Behavioral Backbone for Skill Curation and Revision

Two recent papers -- SkillOS and SkillRevise -- formalize agent skill lifecycle patterns that
assume the existence of an external verification service. TC fills that role.

### 6A.1 TC as the Judge Reward for SkillOS-Style Skill Curation (arXiv:2605.06614)

SkillOS (May 2026) pairs a frozen executor agent with a trainable "skill curator" that
maintains an external SkillRepo. The curator learns which skills to keep, promote, or retire
based on a composite reward signal: earlier trajectories update the SkillRepo, and later tasks
evaluate whether those updates improved downstream performance. Over time, skills evolve into
richly structured Markdown files encoding meta-skills -- abstract patterns reusable across
task families.

The critical dependency in this architecture is the **judge reward** -- the outcome signal that
tells the curator whether a skill actually worked. SkillOS assumes this signal exists but does
not specify where it comes from or how it is verified.

TC provides that signal with behavioral provenance:

- **Gate pipeline as judge reward.** TC's gate pipeline produces per-session accept/reject
  verdicts with TEE attestation. These verdicts are the composite reward signal: each trace
  submission that invokes a skill generates a quality-scored, attested outcome. A SkillOS
  curator consuming TC verdicts knows not just "did the skill succeed?" but "did the skill
  succeed across N sessions, M contributors, with consistency score S?"

- **Multi-session behavioral data as training signal.** SkillOS trains its curator on grouped
  task streams. TC's cross-session corpus IS that grouped stream -- organized by skill
  identity, contributor, temporal window, and behavioral profile. The curator does not need
  to collect its own training data; TC has already aggregated and scored it.

- **Trust-weighted skill ranking.** SkillOS ranks skills by composite reward. TC extends this
  by weighting the reward with contributor reputation (Glicko-2 ratings from doc 02) and
  behavioral consistency scores. A skill that scored well in 5 sessions from 1 contributor
  ranks differently than one that scored well in 50 sessions from 12 contributors. Static
  metadata (author, stars, downloads) cannot make this distinction.

The relationship is complementary: SkillOS defines the curator architecture; TC provides the
verified behavioral substrate the curator trains on.

### 6A.2 TC as the Fixed Verifier for SkillRevise-Style Revision Loops (arXiv:2606.01139)

SkillRevise (May/June 2026) addresses the quality problem from the opposite direction: given
an LLM-authored skill that does not work reliably, how do you fix it? The method operates in
a tight loop: execute the skill, diagnose failures from the execution trace, revise the skill
source, re-execute, and select the first version that passes a **fixed verifier**. When no
candidate passes, SkillRevise falls back to empirical utility -- selecting the version with
the best observed outcome. On SkillsBench, this loop improves base agent success rate from
36.05% to 61.63%, a 25.58 percentage-point improvement.

The architecture has a hard requirement: a **fixed verifier** that produces deterministic
pass/fail verdicts on skill executions. Without this, the revision loop has no convergence
criterion. SkillRevise assumes the verifier exists but does not specify how it is
implemented or trusted.

TC's gate pipeline is that verifier:

- **Deterministic gate verdicts.** TC's 7-rung gate pipeline (compile, test, clippy, diff,
  and higher rungs with oracle evaluation) produces binary pass/fail verdicts per trace
  submission. These verdicts drive the SkillRevise select-or-revise decision directly.

- **TEE attestation of verdicts.** A SkillRevise loop running against a self-hosted verifier
  has a trust problem: the verifier might be compromised, misconfigured, or gamed. TC's TEE
  attestation ensures that the verdict was computed inside a hardware-isolated enclave on the
  actual execution trace, not on a synthetic or tampered input.

- **Empirical utility from population data.** SkillRevise's fallback -- empirical utility
  when all candidates fail -- maps directly to TC's behavioral consistency scoring. If no
  revision passes the gate, TC's cross-session outcome data provides the relative ranking:
  "version B failed the gate but succeeded in 7/10 real-world invocations vs. version A's
  3/10."

- **Revision provenance.** Each revision cycle generates new traces. TC stores the full
  revision chain: original skill trace, diagnosis, revised skill trace, gate verdict. This
  provenance chain is itself a signal for skill quality -- a skill that required 8 revisions
  to pass carries different trust than one that passed on the first attempt.

TC does not need to implement skill revision. It provides the external verification service
that makes revision loops trustworthy. Any SkillRevise-compatible system can point its
verifier at TC's gate pipeline and get TEE-attested, population-informed verdicts.

---

## 7. Competitive Positioning

**Against NVIDIA Verified Agent Skills.** TC differs structurally: (1) scale -- automated
behavioral analysis vs. manual review bottleneck; (2) continuous verification vs. point-in-
time signing; (3) longitudinal behavioral depth across sessions and contributors vs.
sandboxed single execution. The two are complementary -- a skill with both NVIDIA signing
and TC behavioral verification carries the strongest trust signal.

**Against on-chain registries (SIGIL et al.).** On-chain answers "who published this and
when?" TC answers "what does it actually do?" Orthogonal. Integrate, don't compete.

**Against skill curation systems (SkillOS et al.).** SkillOS (arXiv:2605.06614) formalizes
the skill curator architecture -- a trainable module that maintains a SkillRepo and learns
what to keep based on composite rewards. The curator assumes an external judge reward signal
but does not verify it. TC provides that signal with behavioral provenance: TEE-attested
gate verdicts across multiple sessions and contributors. A SkillOS curator backed by TC
ranks skills by verified behavioral track record, not static metadata alone.

**Against skill revision systems (SkillRevise et al.).** SkillRevise (arXiv:2606.01139)
improves LLM-authored skills from 36.05% to 61.63% success rate via a diagnose-revise-verify
loop. The loop requires a fixed verifier as its convergence criterion. TC's gate pipeline
serves as that verifier with two properties no self-hosted alternative provides: TEE
attestation (the verdict is tamper-proof) and population-scale empirical utility (when no
revision passes, cross-session data ranks candidates by real-world outcome).

**Market opportunity.** 490K+ skills, 36.82% with at least one issue (Snyk), no centralized
behavioral trust registry. Enterprise demand demonstrated by NVIDIA's investment. OWASP AST10
provides taxonomy but no enforcement. First behavioral trust registry at scale captures a
category-defining position.

---

## 8. Risks and Mitigations

| Risk | Mitigation |
|---|---|
| **Low trace-to-skill match rate** | Phase 1 targets popular skills; Phase 3 retroactive matching expands coverage |
| **Gaming via synthetic traces** | TEE attestation of capture; multi-contributor requirement; Glicko-2 reputation weighting |
| **Badge misinterpretation** | Badge shows explicit scores and counts, not binary safe/unsafe; UX emphasizes interpretation |
| **Re-verification latency** | Hash-change triggers immediate re-check; high-download skills get shorter cadence |
| **Corpus size insufficient** | Tier launches after critical mass; Phase 1 selects skills with existing trace coverage |
| **False sense of security** | Badge states "no anomalies in N invocations" not "safe"; complements, does not replace other measures |

---

## 9. Dependencies

| Dependency | Status | Blocks |
|---|---|---|
| **Issue #210 fix** (scoring inversion) | Urgent | Behavioral consistency reuses gate pipeline |
| **Issue #219 fix** (redaction penalty) | Urgent | Trace quality scores feed verification |
| **Process mining** (doc 02, B.3) | Planned | DAG conformance for drift detection |
| **Sub-trace decomposition** (doc 02, B.6) | Planned | Skill-specific trace segment isolation |
| **Skill extraction** (doc 02, C.11 / doc 03, sec 3) | Planned | Automated skill identification |
| **OTel ingest** (doc 03, sec 1) | Planned | Broader corpus for population-scale analysis |

---

## 10. Verification Ledger

| Item | arXiv / Source | Status | Key Claim |
|---|---|---|---|
| SkillVetBench | 2606.00925 | **Verified** | Static baselines miss up to 89% of malicious skills |
| MalSkillBench | 2606.07131 | **Verified** | Syscall trace capture for behavioral analysis |
| ClawGuard | 2604.11790 | **Verified** | Runtime skill security framework |
| SkillSieve (CORRECTED) | 2604.06550 | **Verified** | F1=0.800 (Layer-2); 0.920 is full 3-layer pipeline |
| SkillFortify | -- | **Verified** | ~96.95% F1, zero FP, executable code only |
| NVIDIA Verified Skills | NVIDIA blog/docs | **Verified** | 162 signed skills, 8-stage pipeline |
| OWASP AST10 | owasp.org | **Verified** | Agent Security Top 10, Feb 2026 |
| SIGIL | -- | **Verified** | On-chain skill provenance registry concept |
| ClawHavoc (Koi Security) | Koi Security report | **Verified** | 341 malicious from single op (of 2,857) |
| ClawHavoc (Scandar) | Scandar report | **Verified** | 1,184 malicious; ~300K users affected |
| ClawHavoc (Snyk) | Snyk audit | **Verified** | 1,467/3,984 (36.82%) with >= 1 issue |
| Trail of Bits bypass | Trail of Bits report | **Verified** | All scanners bypassed in < 1 hour |
| RHO | 2606.05922 | **Verified** | 59% -> 78% SWE-Bench Pro |
| Trace2Skill | 2603.25158 | **Verified** | +57.65pp skill extraction |
| SkillAudit | 2606.14239 | **Verified** | Security scanning for extracted skills |
| SkillOS | 2605.06614 | **Verified** | Trainable skill curator with composite reward on external SkillRepo |
| SkillRevise | 2606.01139 | **Verified** | Trace-conditioned revision loop; 36.05% -> 61.63% on SkillsBench |

---

## 11. Deep Research Queries

### Q-VS1: Multi-Session Behavioral Analysis for Skill Security
`"agent skill" behavioral analysis runtime security multi-session 2026`
Methods for detecting delayed-activation or context-dependent malicious behavior across sessions. Population-scale anomaly detection on agent tool use.

### Q-VS2: TEE-Attested Behavioral Reports
`"trusted execution environment" behavioral attestation verification report 2026`
Systems producing TEE-attested behavioral reports. Attestation chain design. Consumer verification without platform trust.

### Q-VS3: Skill Trust Registries at Scale
`"trust registry" OR "verified skills" agent scale automated 2026`
Automated registries beyond NVIDIA's 162-skill program. Enterprise demand signals. Trust-as-a-service pricing.

### Q-VS4: Behavioral Drift Detection
`"behavioral drift" OR "concept drift" software monitoring longitudinal 2026`
Drift detection in software over time. Statistical tests for distributional shift in behavioral profiles.

### Q-VS5: SkillVetBench Sandbox Methodology
`SkillVetBench sandbox execution trace verification 2606.00925`
Sandbox execution details and trace capture methods. Comparison with TC's envelope format.

### Q-VS6: Skill Curation from Behavioral Signals
`"skill curation" OR "skill selection" agent reward behavioral 2026`
Automated approaches to skill curation trained on execution outcomes. Composite reward design for skill ranking. SkillOS SkillRepo evolution patterns.

### Q-VS7: Trace-Conditioned Skill Revision
`"skill revision" OR "skill refinement" trace execution verifier agent 2026`
Revision loops conditioned on execution traces. Fixed verifier requirements. Empirical utility as fallback when formal verification fails.
