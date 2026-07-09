# Daimon No Terminal State

> From any of the 6 Daimon behavioral states, there exists at least one valid transition to another state. There are no terminal (absorbing) states.

**Crate**: `roko-daimon`
**Test type**: Unit test
**Enforcement**: Behavioral state machine graph
**Last reviewed**: 2026-04-19

---

## Statement

For all behavioral states S in {Engaged, Struggling, Coasting, Exploring, Focused, Resting}:
`∃ state S' ≠ S such that S can transition to S'`

---

## Why It Matters

A terminal state would mean the agent gets "stuck" in a behavioral mode with no way out. For example, if `Struggling` were terminal, an agent that entered the struggling state would remain there indefinitely — perpetually allocating high compute for a task it can never complete differently.

The cycling property ensures the agent's affect is responsive: all states are transient.

---

## Test

```rust
#[test]
fn daimon_no_state_is_terminal() {
    let states = [
        BehavioralState::Engaged,
        BehavioralState::Struggling,
        BehavioralState::Coasting,
        BehavioralState::Exploring,
        BehavioralState::Focused,
        BehavioralState::Resting,
    ];

    for state in &states {
        let outgoing: Vec<_> = BehavioralStateMachine::outgoing_transitions(state).collect();
        assert!(!outgoing.is_empty(),
            "State {:?} must have at least one outgoing transition", state);
    }
}
```

---

## See also

- [../by-subsystem/subsystem-daimon.md](../by-subsystem/subsystem-daimon.md)
- [pad-vector-bounds.md](pad-vector-bounds.md)
