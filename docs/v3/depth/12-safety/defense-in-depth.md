# Defense in Depth

> **v3 depth file** -- `/docs/v3/depth/12-safety/defense-in-depth.md`
> Canonical source: v1 `docs/v1/11-safety/00-defense-in-depth.md`
> Status: **Complete** (E34 8/8 strict). Trust-origin taint, capability intersection,
> immune Graph, corrigibility pipeline, sandbox levels, and audited production hooks live.

---

## 1. Architectural Principle

Safety in Roko is not a subsystem. It is a spine that runs through every layer of the
architecture and every step of the processing loop. No single guard is assumed sufficient.
An action that matters is subject to authorization, pre-call validation, post-call
verification, taint-aware policy, corrigibility ordering, and durable audit evidence.

The three fundamental concerns are distinct but stitched together:

| Concern | Question | Primary enforcement |
|---|---|---|
| Authorization | May this principal perform this action on this target? | Three-layer capability intersection |
| Isolation | If code is untrusted, can it escape its declared envelope? | Five-level sandbox (`SandboxLevel`) |
| Provenance | Can an auditor reconstruct what happened and why? | Signal lineage, taint metadata, audit hooks |

These concerns cannot be collapsed into one mechanism. Authorization without isolation is
trust. Isolation without authorization is a crude jail. Either without provenance leaves
incidents irrecoverable.

---

## 2. The Six Defense Layers

The concrete stack, from lowest to highest:

### Layer 1: Path and workspace boundaries

Filesystem tools operate inside an explicitly authorized worktree. `PathPolicy` in
`roko-agent/src/safety/path.rs` enforces:

- Relative paths resolved against the active worktree root.
- Escape via `..` or absolute paths denied.
- Symlink resolution checked before the tool touches disk.
- Writes outside declared scope fail closed.

Authority is rooted at the canonical workspace, not disposable attempt worktrees. This
distinction matters because plan execution creates temporary worktrees that should not
inherit the safety configuration's write scope.

### Layer 2: Process and runtime controls

`ProcessSupervisor` in `roko-runtime` enforces:

- Per-invocation time budgets.
- Abnormal termination becomes a safety Signal.
- Repeated violations can disable the tool or plugin.
- Post-call checks decide whether the result can persist or broadcast.

### Layer 3: Plugin sandbox tiers

Five `SandboxLevel` variants (None, Observe, Restrict, Isolate, Quarantine) map to
five `PluginTier` trust classes (Kernel, Trusted, Standard, Sandboxed, Untrusted).
See `sandboxing-5-level.md` for the full specification.

### Layer 4: Policy and gate checks

The `SafetyLayer` in `roko-agent/src/safety/mod.rs` is a composite policy that chains:

- `BashPolicy` -- deny dangerous shell commands.
- `GitPolicy` -- deny force-push, protected branches.
- `NetworkPolicy` -- deny private networks, enforce HTTPS.
- `PathPolicy` -- deny paths outside worktree.
- `RateLimiter` -- deny if rate limit exceeded.
- `ScrubPolicy` -- post-execution secret redaction.
- `SandboxPolicy` -- enforce level-appropriate restrictions.

Pre-call checks reason about intent; post-call checks reason about consequence. Both
are required because safe parameters can yield dangerous output, and dangerous intent
should never run because eventual output might be harmless.

### Layer 5: Custody and attestation

Durable evidence of authorization, taint state, gate verdicts, and outcome. Every
`ToolDispatcher` call emits an audit Signal via `emit_audit()` at each pipeline stage.
Content-addressed Signal hashes provide tamper evidence. See `audit-chain.md`.

### Layer 6: Threat monitoring

Residual-risk tracking, incident response, and replay. The `DiagnosisEngine` in
`roko-conductor` performs root-cause analysis when circuit breakers trigger. Transitive
quarantine incidents persist across restarts and link back to causal evidence.

---

## 3. Safety Through the Processing Loop

Safety cuts through the seven-step loop:

| Step | Safety responsibilities |
|---|---|
| SENSE | Attach taint via `TaintTracker`, record trust-origin level |
| ASSESS | Apply role checks, corrigibility ordering, risk-aware routing |
| COMPOSE | Preserve taint in composed prompts, include only context allowed for the principal |
| ACT | Enforce pre-call checks, sandbox limits, egress policy, checkpoint requirements |
| VERIFY | Run gate verdicts through the five-layer immune Graph |
| PERSIST | Persist Signals with provenance; content-addressed hashes ensure integrity |
| REACT | Tighten permissions, disable plugins, open quarantine incidents |

Safety therefore lives at the point of action and in the after-action consequences.

---

## 4. Shared Permission Vocabulary

Every permission-gated action is evaluated against a tuple:

- **Principal:** user id, agent id, or plugin id.
- **Action:** a controlled verb (file read, file write, shell execution, network egress).
- **Target:** file path, tool id, endpoint.
- **Context:** the active `ToolContext` and domain profile.

The `ToolPermission` struct encodes five capability flags:

```rust
pub struct ToolPermission {
    pub read: bool,      // Can read files and query state
    pub write: bool,     // Can modify files
    pub exec: bool,      // Can execute commands
    pub git: bool,       // Can perform git operations
    pub network: bool,   // Can make network requests
}
```

`ToolPermission::satisfied_by()` verifies that the role's granted permissions satisfy
the tool's requirements. The deny list is evaluated before the allow list. A tool on
both lists is blocked.

---

## 5. Three-Layer Capability Intersection

The E34 strict manifest introduced exact capability intersection across three layers:

1. **Cell capabilities** -- what the Cell's type contract permits.
2. **Graph capabilities** -- what the Graph topology authorizes for this position.
3. **Space capabilities** -- what the runtime execution space grants.

An action is permitted only at the intersection of all three. This prevents capability
widening through any single layer: a Cell with write access in a Graph that forbids
writes, or a Graph that allows writes in a Space that denies them, both result in denial.

---

## 6. Human-in-the-Loop Checkpoints

Human checkpoints are part of the permission model, not an exception to it. Three types:

**Permission checkpoint.** Used for `AllowWithConfirm` decisions. The prompt shows the
principal, action, target, and scope of approval (once or session).

**Ambiguity checkpoint.** Used when the choice between options is under-specified or
low-confidence. Prevents accidental commitment under uncertainty and turns the user's
choice into a calibration signal.

**Review checkpoint.** Used before destructive or externally visible actions. Prior
permission does not waive review. The user inspects the diff, parameters, and intended
effect before confirmation.

---

## 7. The Immune System Analogy

The five-layer immune Graph provides a biological analogy for the safety pipeline:

1. **Innate immunity** -- T0 deterministic checks (rate limits, path policy, taint).
2. **Pattern recognition** -- known threat signatures, bash deny patterns.
3. **Adaptive response** -- learned gate thresholds, playbook-based detection.
4. **Quarantine** -- transitive incident isolation, restart-durable.
5. **Memory** -- gate threshold EMA, efficiency tracking, provider health.

Canonical provider primary outputs and every host-visible `ToolDispatcher` result
traverse this fixed five-stage immune Graph. The agent cannot modify its own
verification pipeline.

---

## 8. Fail-Closed Defaults

The system fails closed at every boundary:

- Unknown roles deny all tools.
- Missing safety contracts deny the action.
- Unsupported policy-bearing dispatches are rejected.
- Tool calls without matching registry entries are denied.
- Network requests to unallowed hosts are denied.
- Tainted inputs reaching high-risk destinations escalate or block.

The rationale is simple: the cost of a false negative (allowing a dangerous action)
exceeds the cost of a false positive (blocking a safe action) by orders of magnitude.

---

## 9. Cross-Cutting Controls

### Network egress

All outbound traffic crosses one egress shim. The shim evaluates:

- Whether the principal may perform network egress.
- Whether the destination host is on the allowlist.
- Whether the request crosses a compliance boundary.
- Whether the action should produce a review checkpoint.

### Secrets

Secret-typed values render as redacted. `ScrubPolicy` applies regex-based secret
scrubbing on tool output before it enters the LLM context. Default patterns cover
API keys, AWS credentials, private keys, and bearer tokens.

### Provider isolation

Each LLM provider session is isolated. Provider-owned internal calls and results
do not cross the host-visible audit boundary. This prevents a compromised provider
from using another provider's credentials or context.

---

## 10. What This Chapter Covers

The rest of the safety chapter decomposes the spine:

| Depth file | Concern |
|---|---|
| `capability-tokens.md` | Token-based access enforcement |
| `audit-chain.md` | SafetyHook, audit records, custody |
| `taint-tracking-ifc.md` | Trust-origin lattice, information flow control |
| `permits-allowlists.md` | ToolPermissionPolicy, role matrix |
| `loop-detection.md` | Non-widening validation, circuit breakers |
| `sandboxing-5-level.md` | Five sandbox levels, plugin tier mapping |
| `prompt-security.md` | Hallucination detection, injection defense |
| `threat-model.md` | Trust assumptions, attack surfaces |
| `adaptive-risk.md` | SafetyBudget, Kelly fraction |
| `corrigibility-5-head.md` | Five-head lexicographic ordering |
| `witness-dag.md` | Cryptographic DAG for cognitive traces |
| `temporal-logic.md` | LTL safety and liveness properties |
| `formal-verification.md` | Structural invariants |
| `cognitive-kernel-safety.md` | Namespaces, signals, syscalls |
| `forensic-ai.md` | Replay, evidence integrity, regulatory compliance |

Together they answer the operator-facing question: who did what, with what
authorization, and with what consequence?

---

## Implementation References

| Component | Location |
|---|---|
| SafetyLayer | `crates/roko-agent/src/safety/mod.rs` |
| PathPolicy | `crates/roko-agent/src/safety/path.rs` |
| SandboxPolicy | `crates/roko-agent/src/safety/sandbox.rs` |
| TaintTracker | `crates/roko-agent/src/safety/taint_propagation.rs` |
| CorrigibilityPipeline | `crates/roko-core/src/corrigibility.rs` |
| ToolDispatcher | `crates/roko-agent/src/dispatcher/mod.rs` |
| RateLimiter | `crates/roko-agent/src/safety/rate_limit.rs` |
| ScrubPolicy | `crates/roko-agent/src/safety/scrub.rs` |
| ProcessSupervisor | `crates/roko-runtime/src/process.rs` |
| DiagnosisEngine | `crates/roko-conductor/` |
