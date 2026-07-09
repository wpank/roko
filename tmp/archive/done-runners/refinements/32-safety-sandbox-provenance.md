# Safety, Sandbox, and Provenance

> **TL;DR**: Safety in Roko isn't a single module — it's a spine that
> runs orthogonally across every layer. Role-based tool authorization,
> per-tier plugin sandboxes, pre/post check pairs, taint propagation,
> cryptographic attestation, chain-of-custody records, and a permission
> gradient for human-in-the-loop checkpoints all share one vocabulary
> and one point of enforcement. This doc pulls those threads out of 17,
> 23, 24, 25, and 26 into a single defensive story. The goal: an
> operator can answer "who did what, with what authorization, with
> what consequence?" for any action Roko took.

> **For first-time readers**: This is the doc for security reviewers,
> compliance officers, and anyone deploying Roko in a regulated context.
> It consolidates the safety material scattered across the earlier
> refinements into one defensive spine. Start with §2 (the permission
> model), §4 (human-in-the-loop), and §5 (chain of custody); the rest
> adds depth.

## 1. What "safety" means here

Three distinct concerns, all of which belong in this doc:

1. **Authorization**: who (user, agent, plugin) is allowed to do
   what. Tool calls, Engram writes, Pulse publishes, topic
   subscriptions, substrate reads. Enforcement is trait-level.
2. **Isolation**: when untrusted code runs (tier-3 tool, tier-4
   native extension, tier-5 WASM extension), it can't violate its
   declared capability envelope. Enforcement is layer-level
   (process, container, WASM sandbox).
3. **Provenance**: for every action that mattered, a durable record
   exists of who initiated it, what they were trying to do, what
   heuristics influenced them, what gates approved, and what
   resulted. Enforcement is Substrate-level with optional chain
   witnesses.

The spine stitches all three so operators configure them in one
place and the runtime enforces them consistently.

## 2. The permission model

Every permission-gated action names:

- **Principal**: user id, agent id, or plugin id.
- **Action**: a verb from a controlled vocabulary.
- **Target**: Engram kind, Bus topic, tool id, substrate region,
  file-system path, network endpoint.
- **Context**: the TypedContext (25 §8.1) at call time.
- **Authorization source**: role grant, session approval, one-shot
  approval, or plugin manifest.

Authorization is checked in `roko-agent/src/safety/` (extending
today's layer) using a deterministic function:

```rust
pub fn authorize(
    principal: &Principal,
    action: &Action,
    target: &Target,
    ctx: &TypedContext,
    env: &SafetyEnv,   // role grants, session approvals, manifest permissions
) -> AuthzDecision;

pub enum AuthzDecision {
    Allow,
    AllowWithConfirm { prompt: String },
    AllowOnce,
    Deny { reason: String },
    Escalate { to: EscalationTarget },
}
```

The decision never silently allows — `AllowWithConfirm` bubbles
to the UI; `Escalate` pages a human.

## 3. Default permission table

Each action has a default decision per principal role. Custom
profiles (25) override. The base table:

| Action | researcher | planner | implementer | reviewer | ops |
|---|---|---|---|---|---|
| Read file under workspace | Allow | Allow | Allow | Allow | Allow |
| Read file outside workspace | Deny | Deny | Deny | Deny | Confirm |
| Write file under workspace | Deny | Deny | Allow | Deny | Confirm |
| Write file outside workspace | Deny | Deny | Deny | Deny | Escalate |
| Run shell command | Deny | Deny | Confirm | Deny | Confirm |
| Make network request | AllowOnce | Deny | Deny | Deny | Confirm |
| Install a dependency | Deny | Deny | Confirm | Deny | Escalate |
| Delete a file | Deny | Deny | Escalate | Deny | Escalate |
| Git commit | Deny | Deny | Confirm | Deny | Deny |
| Git push | Deny | Deny | Escalate | Deny | Escalate |
| Call external API with user credential | Confirm | Deny | Deny | Deny | Confirm |
| Publish Pulse on safety.* topic | Deny | Deny | Deny | Deny | Allow |

Rows extend per-domain (blockchain profile adds "sign transaction,"
ops profile adds "modify kubernetes resource," etc.). Profile
bundles ship their own tables that merge with the base.

## 4. Human-in-the-loop checkpoints

Three categories of checkpoint, each with a UX expectation:

### 4.1 Permission checkpoint

Before an action whose default is `AllowWithConfirm` or
`AllowOnce`. Presented as a modal-style prompt (CLI: inline Y/n;
TUI: popup; Web: dialog with details).

```
Allow: install crate "serde_json 1.0.108"
  Principal: agent:implementer-01
  Source:    heuristic h.042 "When parsing JSON, prefer serde_json"
  Confirm:   [Y]es once  [A]llow this session  [N]o  [E]scalate
```

Remember within session: `AllowOnce` becomes `Allow` for the rest of
the session scoped to the same action+target.

### 4.2 Ambiguity checkpoint

When the agent's confidence is below threshold (per
`30-rich-ux-primitives.md` §2.5). The user chooses between two or
more options; the choice feeds back as a heuristic signal.

### 4.3 Review checkpoint

Before destructive or visible-to-others actions: delete, publish,
send, post. Always ask even if prior approval was granted. "Approval
once doesn't mean approval always" is load-bearing.

```
Review: create PR against upstream/main
  Files: src/core.rs, src/net.rs
  Commits: 1
  Labels: bug, refactor
  [V]iew diff  [E]dit body  [C]reate  [X]ancel
```

This is the "permission stands for the scope specified, not beyond"
rule applied.

## 5. Chain of custody

Every auditable action produces a `Custody` Engram (25 §8.2):

```
Custody {
    action:      ActionHash,           // what was done
    principal:   PrincipalId,          // who did it
    when:        Timestamp,
    authorized:  AuthzEvidence,        // role grant, confirm, escalation?
    why_heuristics: Vec<HeuristicId>,  // priors applied
    why_claims:  Vec<ClaimId>,         // research backing
    simulation:  Option<SimHash>,      // for blockchain / ops
    gates_passed: Vec<GateVerdict>,
    result:      Option<ResultHash>,
    witness:     Option<ChainWitness>, // Phase 2+
}
```

Every domain profile declares which actions require custody records
(25 §4 blockchain: every signed tx; 25 §6 ops: every production
write; 25 §3 research: every external fact-claim).

Custody is queryable:

```bash
roko custody list --action transfer --after 2026-04-01
roko custody show <action-hash>
roko custody verify <action-hash>   # re-check heuristic calibrations
roko custody export --signed > audit.jsonl
```

The chain witness integration (when Phase-2 lands) appends a
signature trail that cross-deployment auditors can verify
independently.

## 6. Plugin sandboxes

Per-tier rules (expanding 17 §8):

### 6.1 Tier 1 & 2 — pure data

No code runs from the plugin. Content is parsed as TOML/YAML/Markdown
and validated against a schema. A malicious tier-1 plugin can at most
propose a bad prompt; the safety layer catches it when the agent
tries to act on it.

### 6.2 Tier 3 — declarative tool manifest

Tool runs as subprocess or MCP server. Enforced:

- `cwd` fixed to a declared directory.
- `env` scrubbed to a declared whitelist.
- `args` templated against `TypedContext` with validation.
- `files_read`, `files_write` glob patterns enforced by pre-call
  path canonicalization + deny-by-default.
- `network` toggle — when false, tool runs in a network-less
  namespace (Linux) or without network entitlement (macOS).
- `timeout_ms` enforced; process killed on expiry.
- `role_allow` restricts which roles can invoke.

A single `safety.pre_call` and `safety.post_call` gate wraps every
tool invocation.

### 6.3 Tier 4 — native extension

Loaded as `cdylib` against the `roko-spi` ABI (17 §5.1). Enforced:

- Extension compiled against a frozen ABI version; incompatible
  ABI = refusal to load.
- Extension runs in the host process; *no runtime isolation*. The
  safety guarantee is purely "the author is trusted."
- Manifest declares permissions; the safety layer rejects calls
  outside declared scope.
- Crash in the extension doesn't propagate thanks to panic-catch
  wrappers around every SPI entry.

Tier 4 should require a signed manifest + reputation signal before
the installer accepts it (17 §7).

### 6.4 Tier 5 — WASM sandbox

Extension runs in a WASM host (17 §13). Enforced:

- CPU time limit per call (default 100 ms).
- Memory limit per instance (default 64 MB).
- No file system access.
- No direct network access. Only hostcalls.
- Hostcalls permission-checked against manifest before dispatch.
- Pulse rate and Substrate query rate limits per instance.

Violations kill the instance, publish `plugin.violation` Pulse,
flag the plugin in the UI as "violated sandbox." Repeated violations
auto-disable.

## 7. Taint tracking

Data from untrusted sources (web scrape, user paste, plugin output)
carries a `taint: Taint` field on the Engram:

```
enum Taint {
    None,
    UserInput,              // pasted prompt, uploaded file
    ExternalFetch(Source),  // HTTP GET, API call
    ThirdPartyPlugin(PluginId),
    LegacyImport,           // imported from another deployment
}
```

Propagation rule: any Engram whose *input* is tainted is itself
tainted. A Composer reads tainted Engrams and produces a tainted
composed prompt; an LLM turn reads the prompt and produces a tainted
output.

Safety gates at step 4 (per `05-loop-retold.md`) and at action points
read the taint and may:
- Require additional confirmation before acting.
- Refuse entirely for high-risk destinations (e.g. signing a
  blockchain tx with tainted recipient address — always escalates).
- Attach taint metadata to custody records so auditors can trace.

Taint is one-way: it only propagates; it doesn't "clean" without
explicit human action (a reviewer approves the output with sign-off).

## 8. Attestation

Some Engrams deserve cryptographic commitment:

```
Attestation {
    signer: PublicKey,
    signature: Ed25519Signature,
    signed_hash: ContentHash,
    timestamp: i64,
    level: AttestationLevel,
}

enum AttestationLevel {
    LocalAgent,       // signed by this agent's session key
    OrgRole,          // signed by a human-owned role key
    ChainWitness,     // committed to on-chain (Phase 2+)
}
```

Attestation is always opt-in per-kind. Defaults:

- GateVerdict: `LocalAgent` (low-friction auditability).
- Custody for destructive action: `OrgRole` (requires human sign-off).
- Heuristic commons contribution: `ChainWitness` (Phase 2+, for
  cross-deployment trust).

Verification is `roko attest verify <hash>` which walks the chain of
attestations along the Engram's lineage.

## 9. Network egress control

All outbound network calls go through a single shim:

```rust
pub trait Egress: Send + Sync {
    async fn get(&self, url: &Url, ctx: &SafetyCtx) -> Result<Response>;
    async fn post(&self, url: &Url, body: &[u8], ctx: &SafetyCtx) -> Result<Response>;
    fn allow(&self, url: &Url, principal: &Principal) -> bool;
}
```

Default implementation denies any URL whose host isn't on an allow-list.
The allow-list is populated from:

- Profile defaults (researcher: arxiv, semantic scholar; coder:
  crates.io, github; ops: cloud provider APIs).
- Plugin manifests (tier-3 can declare hosts they need).
- User-approved additions during session.

Every request logs source principal + URL + response status to
`network.egress.*` Pulses; the safety-events projection surfaces them
in the dashboard.

## 10. Secrets story

From `24-deployment-ux.md` §3, expanded:

- Secrets never appear in Engrams, Pulses, or logs.
- A `Secret` type wraps values; `Display` and `Debug` print `****`.
- Substrate and Bus both scrub `Secret`-typed fields on the way out.
- Secret rotation is observable: a `secrets.rotated` Pulse fires;
  consumers re-fetch.
- Plugin manifests cannot request `secrets_read` without tier-4 or
  tier-5 plus explicit operator approval.

The secret manager is trait-based; `SecretStore` is pluggable
(OS keychain, Vault, AWS Secrets Manager, 1Password CLI).

## 11. Multi-tenancy isolation

In single-server and clustered deployments (24 §1.2–§1.4):

- Each tenant has a namespace prefix on Bus topics, Substrate keys,
  and plugin scope.
- Cross-tenant data access denied at the Substrate/Bus layer, not at
  the UI layer. Defense in depth.
- Plugins declare whether they are `multi_tenant_aware` (can see
  across tenants) or `tenant_scoped` (default).
- Heuristic commons imports are quarantined per-tenant before
  general availability.

## 12. Evaluator roles and conflict of interest

Some roles evaluate other roles' outputs. Reviewer role evaluates
implementer; compliance role evaluates ops; replication agent
evaluates researcher. Conflict-of-interest rules:

- An agent cannot be both producer and sole reviewer of the same
  action in auto-mode.
- Ambiguous situations escalate to human.
- The `researcher → evaluator` separation from 16 §15 is a specific
  case of this rule.

Enforcement is policy-level — a `ConflictCheckPolicy` watches agent
assignments and flags self-review.

## 13. Threat model

What Roko assumes vs. what it doesn't:

**Assumed trusted**:
- The machine running the binary.
- The kernel crates and the default implementations.
- Signed tier-4 plugins from the registry once installed.
- OS-level secret storage.

**Assumed untrusted**:
- User prompts (could contain prompt injection).
- Remote LLM responses (could mislead tool use).
- Third-party content fetched via tools (web pages, arxiv PDFs,
  MCP server outputs).
- Unsigned tier-4/tier-5 plugins.
- Cross-tenant data in shared deployments.

**Outside the model** (operator must handle):
- Physical access to disk.
- Root compromise of the host.
- Upstream supply-chain attacks on crates.io.

Document the threat model in `docs/security/threat-model.md` so
reviewers know what's in scope.

## 14. Audit tooling

Commands that support a safety review:

```bash
roko custody list --after 7d --principal user:alice
roko custody verify --chain-witness
roko taint show <engram-hash>
roko secret audit              # who accessed what, when
roko plugin audit              # installed plugins, permissions, versions
roko attest list --level OrgRole
roko network log --tail 100
```

All of these hit the same Substrate + Bus primitives as the rest of
the system. Auditor sees the same truth the runtime sees.

## 15. Incident response

When something goes wrong, the combination of custody + attestation
+ taint + replay makes postmortems tractable:

1. Identify the problematic action's custody record.
2. Walk its lineage backward through contributing Engrams.
3. Check which heuristics + claims were cited; note their
   calibration at the time (not now — time-travel via replay).
4. Check taint sources.
5. Replay the decision with the same inputs to confirm
   reproducibility.
6. Publish a postmortem Engram (`Kind::Postmortem`) linked to the
   custody chain.
7. If the root cause was a heuristic, update its calibration; if
   a plugin, update its permissions; if a gate, tighten the pipeline.

This closes the loop: safety incidents become learning signals.

## 16. Staging

Safety is orthogonal to the kernel refactor (06), but it accretes
naturally:

- **Phase C.5**: extend today's safety layer to read from `TypedContext`
  and publish `safety.*` Pulses. One week.
- **Phase C.6**: custody records shipping for destructive actions.
  Two weeks.
- **Phase D**: plugin sandboxes (tier-3 hardening, tier-5 WASM host).
  Three weeks.
- **Phase E**: attestation, taint propagation, audit tooling.
  Three weeks.
- **Phase 2+**: chain witnesses on custody. Depends on `roko-chain`.

Total: two months of focused safety work for a production-grade
defensive spine. Some of it is already in place; this sequences the
rest.

## 17. Cross-references

- Role auth today: `crates/roko-agent/src/safety/`.
- Plugin SPI and tier sandboxes: `17-plugin-extension-architecture.md`.
- Domain-specific custody requirements: `25-domain-specific-agents.md` §8.2.
- Deployment implications (secrets, multi-tenancy): `24-deployment-ux.md`.
- Observability for safety events: `33-observability-telemetry.md` §4.
- Permission UX in each surface: `23-user-ux-running-agents.md` §10,
  `28-cli-parity-familiar-workflows.md` §18.
- Chain witness Phase-2 integration: `09-phase-2-implications.md` §1.
- Replication-ledger adversarial ingestion defense:
  `16-research-to-runtime.md` §15.
