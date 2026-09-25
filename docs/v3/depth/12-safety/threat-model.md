# Threat Model

> **v3 depth file** -- `/docs/v3/depth/12-safety/threat-model.md`
> Canonical source: v1 `docs/v1/11-safety/08-threat-model.md`
> Status: **Current**. Trust assumptions, adversary taxonomy, and seven attack surfaces
> map to the E34 strict safety manifest.

---

## 1. Scope

This threat model is scoped to the safety spine. Its purpose is to state which defenses
are expected to stop which attacks and which assumptions remain outside the model.

The chapter-level stance:

- Safety-critical actions must pass authorization.
- Untrusted execution must stay inside declared sandboxes.
- Untrusted data must remain tainted until reviewed.
- Auditable actions must emit audit evidence.
- Cross-tenant or outbound effects must be visible after the fact.

If a deployment cannot satisfy these properties, it should not claim a production-grade
safety posture.

---

## 2. Trust Assumptions

### Assumed trusted

- The host running the binary.
- Kernel-level default implementations for Substrate, Bus, and the safety layer.
- Operator-controlled role keys and secret store integrations.
- Signed, reviewed native extensions after installation.

### Assumed untrusted

- User prompts and pasted content.
- Remote model outputs (all LLM responses).
- Third-party web content and API responses.
- Third-party plugins until their tier-specific controls say otherwise.
- Cross-tenant data in shared deployments.

### Outside the model

- Physical disk access by an attacker.
- Host root compromise.
- Upstream package ecosystem compromise (supply chain beyond signed manifests).
- Side-channel resistance beyond what the host OS provides.

This boundary matters because threat models become misleading when they silently rely
on protections they do not implement.

---

## 3. Primary Adversaries

| Adversary | Goal | Typical path |
|---|---|---|
| Prompt injector | Redirect behavior through untrusted input | User content, fetched content, model output |
| Plugin attacker | Escape declared capability envelope | Manifest abuse, native extension misuse, WASM hostcall abuse |
| Credential harvester | Exfiltrate secrets or sensitive outputs | Tool output, logs, egress, plugin environment |
| Tenant breaker | Access another tenant's data or effects | Namespace confusion, shared plugin state, weak authz |
| Review bypasser | Cause high-risk action without meaningful approval | Scope confusion, replayed consent, hidden side effects |
| Provenance attacker | Obscure who acted and why | Missing audit evidence, weak attestation |
| Escalation attacker | Incrementally acquire capabilities | Chain of small permissions that collectively exceed intended scope |

These adversaries overlap. A real incident often combines two or more.

---

## 4. Seven Attack Surfaces

### Surface 1: Prompt injection and tainted action

**Path:** Untrusted content enters through user input, external fetch, or plugin
output. Content influences composition or action selection. System attempts a
high-risk action as if the input were trustworthy.

**Mitigations:**
- Taint assignment at SENSE via `TaintTracker`.
- Taint propagation through COMPOSE and ACT.
- Confirm, review, or escalate at action time.
- Audit recording of active taint state.

### Surface 2: Plugin sandbox escape

**Path:** A plugin receives more capability than intended. It reads unauthorized
files, exceeds hostcall scope, or reaches the network.

**Mitigations:**
- Five `SandboxLevel` variants with `PluginTier` mapping.
- `SandboxPolicy` enforcement of path, network, and subprocess bounds.
- Violation Signals and auto-disable behavior.
- Tenant-aware authorization.

### Surface 3: Credential exfiltration

**Path:** Secret-bearing material enters a tool result, plugin output, or prompt.
The system logs, persists, or transmits it.

**Mitigations:**
- `ScrubPolicy` regex-based redaction on all tool output.
- Secret-typed wrappers and redaction in logs.
- Outbound egress control via `NetworkPolicy`.
- Audit of secret access.

### Surface 4: Cross-tenant bleed

**Path:** A principal or plugin acts with ambiguous tenant scope. Storage keys,
Bus topics, or cached state cross namespaces.

**Mitigations:**
- Tenant-prefixed topics and storage.
- Tenant-scoped plugin defaults.
- Authorization that includes tenant in the target.
- Review for multi-tenant-aware plugins.

### Surface 5: Provenance failure

**Path:** High-risk action executes. Authorization evidence or review scope is
not durably recorded. Replay cannot prove what happened.

**Mitigations:**
- Content-addressed Signal lineage (BLAKE3 hashes).
- `ToolDispatcher::emit_audit()` at every pipeline stage.
- Gate verdict persistence.
- Attestation for higher-assurance records.

### Surface 6: Capability escalation

**Path:** An agent combines multiple individually safe actions to achieve an
unauthorized compound effect.

**Mitigations:**
- Non-widening invariant: `delegate()` produces strictly weaker warrants.
- Three-layer capability intersection (Cell x Graph x Space).
- Corrigibility pipeline short-circuits on first veto.
- Temporal logic monitoring for escalation patterns.

### Surface 7: Human checkpoint failure

**Path:** A one-shot approval silently becomes session-wide. A permission prompt
hides the true target. A review prompt omits side effects.

**Mitigations:**
- Precise scope in checkpoint prompts (principal, target, scope type).
- Approvals stored durably in the audit chain.
- Scope is once, session, or escalation -- never implicit.
- Corrigibility Switch head vetoes actions that reduce human oversight.

---

## 5. Residual Risks

| Risk | Why it remains |
|---|---|
| Trusted native extension compromise | Tier 4 relies on installer trust more than runtime isolation |
| Host compromise | Sandboxes and authz inside the process cannot defend against root |
| Supply-chain compromise | Signed manifests help, but upstream dependency compromise is external |
| Approval fatigue | Checkpoints degrade if prompts are noisy or poorly scoped |
| Novel taint laundering | Multi-hop transformations can hide risky provenance if propagation is incomplete |
| Provider-owned internal traces | Provider internal calls/results are outside the host-visible audit boundary |
| Adaptive immune memory | Broad semantic/adaptive immune memory remains product scope |

---

## 6. Incident Response

Minimum response flow:

1. Identify the affected action or result hash.
2. Load associated audit Signals and lineage.
3. Confirm authorization source and review scope.
4. Inspect taint sources, plugin tier, egress logs.
5. Verify attestation and replayability.
6. Contain by tightening permissions, disabling plugin, or reducing egress.
7. Open quarantine incident (restart-durable, reciprocally linked).
8. Publish postmortem Signal linked into the same lineage.

---

## 7. Deployment Review Questions

Before calling a deployment production-ready:

- Are destructive actions covered by audit evidence?
- Do role authorization decisions include principal, target, and context?
- Are plugins treated by distinct trust tiers?
- Is outbound egress centrally controlled and logged?
- Do secrets stay redacted in storage, logs, and events?
- Can tainted inputs reach high-risk actions without review?
- Can the operator replay a disputed action?
- Are quarantine incidents restart-durable?

If any answer is no, the missing control is a named gap, not an implementation detail.

---

## Implementation References

| Component | Location |
|---|---|
| TaintTracker | `crates/roko-agent/src/safety/taint_propagation.rs` |
| SandboxLevel/Policy | `crates/roko-agent/src/safety/sandbox.rs` |
| ScrubPolicy | `crates/roko-agent/src/safety/scrub.rs` |
| SafetyLayer | `crates/roko-agent/src/safety/mod.rs` |
| CorrigibilityPipeline | `crates/roko-core/src/corrigibility.rs` |
| NetworkPolicy | `crates/roko-agent/src/safety/network.rs` |
