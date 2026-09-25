# Tool Profiles and Configuration

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- profile-based
> tool loading, configuration hierarchy, environment variables, and domain
> profile bundles.

---

## 1. Overview

Tool profiles control which tools are loaded at agent boot. A profile is a
named selection of tool categories -- it determines the agent's structural
capabilities. An agent with a read-only profile is **structurally unable** to
write: the write tool handlers do not exist in its registry. This is not a
runtime policy check; it is a structural absence.

Profiles are set in `roko.toml` or via the `TOOL_PROFILE` environment
variable. They compose -- `TOOL_PROFILE=trader,vault` activates both.

---

## 2. Built-in Tool Access Model

The 16 built-in tools use **role-based access** via
`StaticToolRegistry.for_role()`, not profile-based filtering. All 16 tools
are available regardless of domain configuration. The role determines which
subset is visible:

| Role | Sees | Does Not See |
|---|---|---|
| Implementer | All 16 | -- |
| Reviewer | read_file, glob, grep, ls, web_fetch, web_search | write/exec tools |
| Researcher | read + web tools | write/exec tools |
| Architect | read + search tools | write/exec tools |
| Scribe | read + write tools | exec tools |
| Auditor | read-only tools | write/exec tools |

---

## 3. Domain Profile Bundles

Domain plugins ship installable profile bundles that package:

- Profile defaults and role presets
- Domain-specific tool sets
- Gates and heuristics
- Starter templates
- Typed context schema and custody expectations

The bundle pattern is domain-agnostic. A chain profile, a coding profile, and
a research profile all use the same merge mechanism:

- Tools merge by union
- Roles merge by union (collision warnings)
- Gates stack unless explicitly scoped
- Heuristics coexist and are routed by fit
- Profile priority resolves key conflicts

---

## 4. Configuration Hierarchy

Precedence (highest first):

1. **CLI flags** -- `--profile trader --disable tool_name`
2. **Environment variables** -- `TOOL_PROFILE=trader`, `ROKO_TOOL_DISABLE=...`
3. **Config file** -- `roko.toml` `[tools]` section
4. **Defaults** -- `active` profile if nothing specified

### roko.toml Configuration

```toml
[tools]
profile = "active"
enable = ["intel_compute_vpin"]
disable = ["uniswap_submit_uniswapx_order"]

[tools.safety]
max_writes_per_minute = 10
require_simulation = true
```

Per-tool `enable`/`disable` overrides take precedence over the profile's
category selection. There is no shipping `[tools.cache]` configuration --
the dispatcher executes every call against current authorization.

### Environment Variables

| Variable | Purpose | Example |
|---|---|---|
| `TOOL_PROFILE` | Profile selection | `trader,vault` |
| `ROKO_TOOL_DISABLE` | Disable specific tools | comma-separated names |
| `ROKO_TOOL_ENABLE` | Enable specific tools | comma-separated names |
| `ROKO_MEMORY_ENABLED` | Enable Neuro integration | `true` |

---

## 5. Profile Filtering Mechanism

Profile filtering uses the `ToolDef.category` field. Filtering happens once
at initialization:

```rust
fn resolve_profile_categories(profile: &str) -> HashSet<Category> {
    match profile {
        "active" | "full" => ALL_CATEGORIES.iter().copied().collect(),
        // ... domain-specific profiles
        _ => [Category::Data].into(), // fallback
    }
}

let allowed = resolve_profile_categories(profile);
let tools: Vec<&ToolDef> = ALL_TOOL_DEFS
    .iter()
    .filter(|t| allowed.contains(&t.category))
    .collect();
```

For domain bundles, filtering is the first pass. The runtime also binds a
domain-specific `TypedContext` and routes custody-aware writes through the
domain's provenance rules.

---

## 6. Profile Interaction with Cognitive Subsystems

The profile affects not just tool availability but how cognitive subsystems
behave:

| Profile | Daimon Modulation | Dream Frequency | Neuro Priority |
|---|---|---|---|
| active | Full PAD range | Standard | Balanced |
| observatory | Low arousal (watching) | High (more dreaming) | Intelligence |
| trader | High arousal (executing) | Standard | Trading episodes |
| learning | Moderate (exploring) | High | Memory self-improvement |
| data | Low (monitoring) | Low | Data quality |

The Daimon reads the profile to calibrate its PAD vector baselines. Lower
arousal means more T0 probes and fewer T2 deep-reasoning ticks -- appropriate
for passive observation roles.

---

## 7. Capability Gating

Three capability gates control domain tool registration. A tool requiring a
capability that is not present is silently skipped during registration:

| Capability | Required By | How Satisfied |
|---|---|---|
| `wallet` | All chain write tools | Signer configured |
| `uniswap_api` | API-backed tools | `ROKO_UNISWAP_API_KEY` set |
| `memory` | Memory tools | `ROKO_MEMORY_ENABLED=true` |

Capability checking happens once at boot. A read-only profile without a
wallet loads all read tools without error.

---

## 8. Error Taxonomy

Tool errors follow a structured taxonomy:

| Error Code | Category | Retryable | Example |
|---|---|---|---|
| `CHAIN_NOT_SUPPORTED` | Configuration | No | Unsupported chain |
| `WALLET_NOT_CONFIGURED` | Configuration | No | Write without wallet |
| `RATE_LIMITED` | Safety | Yes (after cooldown) | Too many ops |
| `CAPABILITY_EXPIRED` | Safety | Yes (re-preview) | Permit timeout |
| `POLICY_REJECTED` | Safety | No | Policy blocked |

---

*Derived from: v1/18-tools/05-tool-profiles.md. Chain-domain reference profiles
(13 profiles, 423+ tools) moved to chain domain plugin documentation. Domain
bundle composition pattern retained as the general mechanism.*
