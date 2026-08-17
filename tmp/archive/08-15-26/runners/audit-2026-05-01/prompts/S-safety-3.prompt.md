# S-safety-3: Per-session SafetyOverlay with intersect semantics

## Task
Add `SafetyOverlay` (per-session restriction layer) that intersects with the role contract. Sessions can narrow further than the contract; never widen.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-safety-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/28-safety-agent-hardening.md` § S-3.

## Exact changes

```rust
// crates/roko-agent/src/safety/overlay.rs (new)

use super::AgentContract;

#[derive(Debug, Clone, Default)]
pub struct SafetyOverlay {
    /// Tools to additionally deny on top of the contract.
    pub additional_denies: Vec<ToolPattern>,
    /// Network hosts to additionally deny.
    pub additional_network_denies: Vec<HostPattern>,
    /// Filesystem paths to additionally deny.
    pub additional_fs_denies: Vec<PathPattern>,
}

impl SafetyOverlay {
    pub fn read_only() -> Self {
        Self {
            additional_denies: vec![ToolPattern::all_writes()],
            ..Default::default()
        }
    }
}

impl AgentContract {
    /// Intersect: produce a new contract that's at most as permissive as `self` and `overlay`.
    pub fn intersect(&self, overlay: &SafetyOverlay) -> AgentContract {
        let mut narrowed = self.clone();
        narrowed.tool_denies.extend(overlay.additional_denies.iter().cloned());
        narrowed.network_denies.extend(overlay.additional_network_denies.iter().cloned());
        narrowed.fs_denies.extend(overlay.additional_fs_denies.iter().cloned());
        narrowed
    }
}
```

Sessions construct an overlay (default empty) at start, pass to dispatch:

```rust
let effective = base_contract.intersect(&session_overlay);
safety_layer.with_contract(effective)
```

### Tests

```rust
#[test]
fn overlay_narrows_but_never_widens() {
    let base = AgentContract::restricted("agent");
    let overlay = SafetyOverlay::read_only();
    let effective = base.intersect(&overlay);
    assert!(effective.is_more_restrictive_than(&base));
}

#[test]
fn empty_overlay_is_identity() {
    let base = AgentContract::restricted("agent");
    let overlay = SafetyOverlay::default();
    let effective = base.intersect(&overlay);
    assert_eq!(effective.tool_denies.len(), base.tool_denies.len());
}
```

## Write Scope
- `crates/roko-agent/src/safety/overlay.rs` (new)
- `crates/roko-agent/src/safety/mod.rs` (re-export + intersect impl)

## Verify

```bash
rg 'SafetyOverlay|fn intersect' crates/roko-agent/src/safety/
# Expect: at least 3 hits
```

## Do NOT

- Do NOT widen via overlay (whole point is narrowing).
- Do NOT bundle with other S-safety batches.
- Do NOT change `AgentContract` storage shape.
