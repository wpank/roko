# Agent Creation

> **v3 depth file** -- `/docs/v3/depth/36-lifecycle/agent-creation.md`
> Canonical source: v1 `docs/v1/17-lifecycle/01-agent-creation.md`
> Status: **Current** (AgentCoreManifest/AgentExtendedManifest live in
> `roko-agent/src/lifecycle.rs`; `roko agent create` and `roko init` wired)

---

## 1. Design Principles

Agent creation follows a three-interaction pattern: **Describe**, **Review**,
**Confirm**. A user who has never heard of roko should produce a running agent
in under three minutes. An experienced user should deploy from a TOML config
in under sixty seconds.

Three entry points converge on a single artifact:

```
CLI (roko init / roko agent create)  ---+
                                        +--> AgentExtendedManifest --> Provisioning
API (POST /v1/agents) ------------------+    (see provisioning.md)
TOML file (roko.toml) ------------------+
```

**Progressive disclosure.** The creation wizard shows only what is essential
by default. Advanced configuration -- model routing, knowledge-sharing
policies, tool profiles, inference provider selection -- is available behind
explicit flags. Users who want control get it; users who do not never see it.

**Confirm before commit.** Every irreversible action -- compute provisioning,
resource allocation, on-chain registration (chain-domain) -- gets explicit
confirmation with a cost breakdown. No surprise charges.

---

## 2. AgentCoreManifest

The minimal manifest sufficient for the happy path. A user who types a single
prompt and accepts defaults produces exactly this:

```rust
// crates/roko-agent/src/lifecycle.rs

pub struct AgentCoreManifest {
    /// Free-text description of what the agent should do.
    pub prompt: String,
    /// Deployment mode: Hosted or SelfHosted.
    pub mode: DeploymentMode,
    /// Domain plugin to activate (Chain, Coding, Research, Custom).
    pub domain: Option<DomainPlugin>,
    /// Schema version for forward compatibility. Current: 1.
    pub schema_version: u32,
}
```

`AgentCoreManifest::new(prompt)` defaults to `SelfHosted`, no domain plugin,
schema version 1. These four fields are all a new user needs.

---

## 3. AgentExtendedManifest

The full manifest with all optional overrides resolved. The provisioning
pipeline never works with a partial manifest:

```rust
pub struct AgentExtendedManifest {
    pub core: AgentCoreManifest,
    pub name: Option<String>,
    pub strategy_md: Option<String>,
    pub model_routing: Option<ModelRoutingConfig>,
    pub neuro: Option<NeuroConfig>,
    pub mesh: Option<MeshConfig>,
    pub tool_profile: Option<String>,
    pub template_id: Option<String>,
    pub template_params: Option<HashMap<String, String>>,
    pub autofill: Option<AutofillProvenance>,
    pub inference: Option<InferenceConfig>,
    pub budget: Option<BudgetConfig>,
    pub lineage_id: Option<String>,
    pub generation: u32,
    pub successor: Option<SuccessorConfig>,
    pub creation_metadata: Option<AgentCreationMetadata>,
}
```

Key additions over the core manifest:

- **lineage_id / generation** -- optional lineage tracking across
  backup-delete-create-restore cycles. See `new-agent-creation.md`.
- **creation_metadata** -- persists `--skills`, `--tier`, `--reputation`, and
  `--max-concurrent-jobs` CLI flags so they survive server-absent creation.
- **successor** -- elevated exploration configuration for agents created as
  replacements (generation > 0).

---

## 4. Manifest Resolution

`resolve_manifest()` fills in all missing fields before provisioning begins:

1. Start with sane defaults for the selected mode and domain.
2. If `template_id` is set, expand the template with `template_params`.
3. If no template, run AI autofill for `strategy_md`, `name`, and
   domain-specific fields.
4. Apply explicit overrides from the extended manifest.
5. Validate the final manifest against the domain's feature set.
6. Compute resource estimates.

The provisioning pipeline (see `provisioning.md`) receives the fully-resolved
manifest and never encounters missing fields.

---

## 5. Domain Plugins

Roko's kernel is domain-agnostic. Domain-specific behavior is injected via
plugins that configure which tools, gates, and knowledge types are active:

| Plugin | Crate | What it adds |
|--------|-------|-------------|
| `Chain` | `roko-chain` | Wallet management, on-chain tools, ERC-8004 identity |
| `Coding` | `roko-agent` | File system tools, compiler gates, VCS integration |
| `Research` | `roko-agent` | Citation tools, paper retrieval, synthesis |
| `Custom` | user-provided | Plugin-specific `id` + key/value `params` |

Each plugin's config struct carries domain-specific fields:

```rust
pub struct ChainConfig {
    pub network: String,       // "base", "base-sepolia", "anvil"
    pub custody_mode: String,  // "delegation", "embedded", "local-key"
}

pub struct CodingConfig {
    pub workspace_path: String,
    pub language: Option<String>,
}

pub struct ResearchConfig {
    pub topic: Option<String>,
    pub citations_enabled: bool,
}
```

---

## 6. Strategy Templates

Five curated templates provide instant, deterministic, auditable alternatives
to AI autofill:

| Template ID | Domain | Description |
|------------|--------|-------------|
| `rust-coding` | Coding | Plan-to-implementation pipeline with gate validation |
| `research` | Research | Deep research with citations and synthesis |
| `code-review` | Coding | PR review, bug detection, improvement suggestions |
| `monitoring` | General | Metric watching, anomaly detection, alerting |
| `chain-trading` | Chain | On-chain trading with wallet and strategy execution |

Each template includes pre-written `STRATEGY.md` content with parameter
placeholders. Templates and AI autofill are mutually exclusive.

---

## 7. AI Autofill

When the user submits a free-text prompt, the wizard calls the configured
inference provider to transform the prompt into a complete extended manifest:

- **Model**: Claude Haiku 4.5 (fast, cheap, sufficient for config generation).
- **Estimated cost**: ~$0.0003 per generation (~700-1350 tokens).
- **Security**: AI autofill output is treated as untrusted. The generated
  manifest is always displayed for user review and never auto-submitted to
  the provisioning pipeline.

---

## 8. CLI Entry Points

```bash
# Initialize workspace and generate roko.toml template
roko init

# Create from config file (non-interactive if all fields present)
roko init --config ./roko.toml

# Create from prompt (interactive for missing fields)
roko init --prompt "Weekly code review agent for our Rust codebase"

# Create from template
roko init --template rust-coding --param crate_path=crates/roko-core

# Dry-run: validate and estimate without executing
roko init --config ./roko.toml --dry-run

# Create agent with metadata
roko agent create --name reviewer --domain coding \
    --skills "rust,code-review" --tier Verified
```

Config loading merges four sources in priority order:
CLI flags > environment variables > TOML config file > built-in defaults.

---

## 9. Naming and Identity

Agent names use cryptographic randomness: `agent-{nanoid(12)}`. The `nanoid`
alphabet (A-Za-z0-9_-) produces 64^12 = 4.7 * 10^21 possible names.
Names are not derivable from user ID, strategy type, or any other input.

For chain-domain agents, creation also registers an ERC-8004 on-chain identity
(ERC-721 soulbound token) on the Korai chain, including capability bitmask,
domain stakes, reputation tracks, and system prompt hash (ventriloquist
defense). Non-chain agents do not require on-chain identity.

---

## 10. FIPA-Informed Lifecycle States

Roko's creation maps to the FIPA Agent Management specification (FIPA00023,
2002). FIPA defines six agent states: `INITIATED`, `ACTIVE`, `SUSPENDED`,
`WAITING`, `TRANSIT`, `DELETED`. Roko adds cloud-native extensions:

| Roko state | FIPA equivalent | When |
|-----------|-----------------|------|
| Initiated | INITIATED | Manifest accepted, process not running |
| Provisioning | *(no FIPA)* | Infrastructure allocating, stores initializing |
| Active | ACTIVE | Cognitive loop running, accepting tasks |
| Suspended | SUSPENDED | Halted by operator; state preserved |
| Waiting | WAITING | Self-blocked on external event |
| Hibernated | *(no FIPA)* | Serialized to cold storage |
| Metamorphosing | ~TRANSIT | Mid-transition between roles/capabilities |
| Degraded | *(no FIPA)* | Budget-constrained operation |
| Deleted | DELETED | Process terminated, resources released |

---

## 11. Creation Flow Summary

```
User provides intent (prompt, template, or config file)
  |
  v
Manifest generated (AI autofill or template expansion)
  |
  v
User reviews manifest (edit any field, change domain, adjust budget)
  |
  v
User confirms
  |
  v
Provisioning pipeline
  1. Validate manifest against domain feature set
  2. Allocate resources
  3. Initialize knowledge store
  4. Configure model routing
  5. Load tool profile
  6. Register with coordination layer if enabled
  7. Start cognitive loop
  |
  v
Agent is running
```

The entire flow is designed so that the user never needs to understand the
internal architecture. The user sees: describe, review, confirm, running.

---

## 12. Implementation Sources

| Surface | File | What |
|---------|------|------|
| `AgentCoreManifest` | `crates/roko-agent/src/lifecycle.rs` | Core manifest struct |
| `AgentExtendedManifest` | `crates/roko-agent/src/lifecycle.rs` | Full manifest with overrides |
| `resolve_manifest()` | `crates/roko-agent/src/lifecycle.rs` | Resolution pipeline |
| `roko agent create` | `crates/roko-cli/src/agent_serve.rs` | CLI subcommand |
| `roko init` | `crates/roko-cli/src/main.rs` | Workspace init |

---

## Cross-References

- [provisioning.md](provisioning.md) -- Type-state provisioning pipeline
- [configuration-operator-model.md](configuration-operator-model.md) -- Four-file config model
- [funding-and-budgets.md](funding-and-budgets.md) -- Budget allocation
- [agent-onboarding-flow.md](agent-onboarding-flow.md) -- Interactive onboarding
