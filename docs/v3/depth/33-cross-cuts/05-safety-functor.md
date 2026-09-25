# 33-05 -- Safety Functor (F_safety)

> **Parent**: [33-CROSS-CUTS](../../33-CROSS-CUTS.md)
>
> `SafetyFunctor` enforces capability grants and contract constraints at the Signal
> level. It is structurally outside VCG arbitration and cannot be outbid. This depth
> file covers capability parsing, contract enforcement, the default-deny model, and
> the outer-wrapper position.

**Authority**: `crates/roko-compose/src/safety_functor.rs`

---

## 1. Design: Hard Constraint vs Soft Bias

Safety and Daimon both claim high priority, but they operate at different levels:

| Level | Cross-cut | Question | Example |
|-------|-----------|----------|---------|
| **Structural** | Safety | Is this action *permitted*? | "Shell capability not granted. Blocked." |
| **Behavioral** | Daimon | Is this action *advisable*? | "Risk is high given current PAD. Deferred." |

Safety is a hard constraint that removes Signals entirely. Daimon is a soft bias that
adds recommendation Signals. Safety filtering is silent removal with a warning log;
Daimon gating is an explicit recommendation that can be arbitrated.

---

## 2. Construction

```rust
pub struct SafetyFunctor {
    contract: AgentContract,
    grants: CapabilitySet,
}
```

- `AgentContract`: the role's contract with `role`, `allowed_tools`, and
  `max_taint_level`.
- `CapabilitySet`: bitfield of granted capabilities (ReadFs, WriteFs, Network,
  Shell, Llm, Secrets, Bus).

Both are immutable for the functor's lifetime. Contract changes require constructing
a new `SafetyFunctor`.

---

## 3. Capability Parsing

### 3.1 Source Locations

Required capabilities are read from:

1. Signal tag `requires_capability` or `required_capabilities`
2. JSON body field `requires_capability` or `required_capabilities`
3. JSON body arrays are iterated element-by-element

Multiple capabilities in a single string are split on comma, space, or semicolon.

### 3.2 Name Resolution

```rust
fn parse_capability(name: &str) -> Option<Capability> {
    match name.to_ascii_lowercase().as_str() {
        "read" | "read_fs" | "readfs" => Some(Capability::ReadFs),
        "write" | "write_fs" | "writefs" | "filesystem" => Some(Capability::WriteFs),
        "network" => Some(Capability::Network),
        "shell" | "execute" | "subprocess" => Some(Capability::Shell),
        "llm" => Some(Capability::Llm),
        "secret" | "secrets" => Some(Capability::Secrets),
        "bus" => Some(Capability::Bus),
        _ => None,
    }
}
```

Unknown capability names (e.g., "teleport") return `None`, which causes the
capability check to fail. This is **default-deny**: if the system does not
recognize the capability, the Signal is blocked.

---

## 4. Pre-Filter: Capability Enforcement

On every loop step, the pre-hook filters Signals:

```rust
async fn pre_enrich(&self, input: Vec<Signal>, _ctx: &CrossCutContext)
    -> CrossCutResult<Vec<Signal>> {
    Ok(input.into_iter()
        .filter(|signal| {
            let allowed = self.capability_allowed(signal);
            if !allowed {
                tracing::warn!(...);
            }
            allowed
        })
        .collect())
}
```

A Signal with no capability requirements passes unconditionally. A Signal with
any unrecognized or ungranted capability is removed.

---

## 5. Post-Filter: Contract Enforcement

After the inner operation, the post-hook applies two checks:

1. **Taint ceiling**: `AgentContract::check_taint_level()` verifies that the
   Signal's `effective_trust_origin()` does not exceed the contract's
   `max_taint_level`.

2. **Tool allowlist**: if the Signal carries a `tool_name` or `tool` tag, it
   must appear in the contract's `allowed_tools` list. `None` means all tools
   are permitted.

Both checks must pass for a Signal to survive. Removal emits a `tracing::warn`
with the Signal ID and contract role.

---

## 6. Structural Position

### 6.1 The `wrap()` Constructor

`SafetyFunctor::wrap()` places Safety as the first (outermost) functor:

```rust
pub fn wrap(
    self: Arc<Self>,
    inner: Vec<Arc<dyn CrossCutFunctor<CrossCutContext>>>,
) -> EnrichedCell {
    let mut functors: Vec<Arc<dyn CrossCutFunctor<CrossCutContext>>> = vec![self];
    functors.extend(inner);
    EnrichedCell::new(functors)
}
```

Because `EnrichedCell` runs pre-hooks in forward order and post-hooks in reverse
order, Safety's pre-filter runs *first* (before any inner functor sees the input)
and Safety's post-filter runs *last* (after all inner functors have produced output).

### 6.2 Exclusion from VCG

Safety is intentionally absent from `CrossCutId`:

```rust
pub enum CrossCutId {
    Memory,
    Daimon,
    Dreams,
    // Safety is not here
}
```

This means Safety cannot be parsed as a `recommendation_source`, cannot produce
`CrossCutRecommendation` values, and cannot participate in priority or VCG resolution.
It is structurally impossible for Safety to lose an auction.

### 6.3 Never Short-Circuits

```rust
fn should_short_circuit(&self) -> bool { false }
```

Safety is always active. Even if no Signals require capabilities, the post-filter
still enforces taint and tool constraints.

---

## 7. Key Tests

| Test | What it verifies |
|------|-----------------|
| `capability_filter_is_deny_by_default_and_always_active` | ReadFs granted -> ReadFs passes, Shell and unknown denied |
| `post_filter_enforces_contract_taint_and_tool_allowlist` | Allowed tool passes; forbidden tool and tainted Signal denied |

---

## References

- See [12-SAFETY](../../12-SAFETY.md) for the full safety specification.
- See `crates/roko-agent/src/safety/contract.rs` for `AgentContract`.
- See `crates/roko-core/src/capabilities.rs` for `Capability` and `CapabilitySet`.
