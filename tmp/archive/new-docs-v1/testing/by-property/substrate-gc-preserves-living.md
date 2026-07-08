# GC Preserves Living Engrams

> Garbage collection must never delete an Engram whose decay score is above the GC threshold.

**Crate**: `roko-fs`
**Test type**: Unit test
**Enforcement**: `GarbageCollector::collect`
**Last reviewed**: 2026-04-19

---

## Statement

For all substrates S and GC threshold θ in [0, 1]:

After `gc(S, θ)`: for all Engrams E in S_before where `E.decay_score(now) > θ`: E is still in S_after.

GC may only remove Engrams where `E.decay_score(now) ≤ θ`.

---

## Why It Matters

GC is the only operation that permanently removes data from the substrate. A GC that incorrectly removes living Engrams would cause:
- Knowledge loss: a Neuro item promoted to Persistent could be GC'd if the decay model were not respected.
- Broken lineage: a parent Engram that is GC'd leaves orphaned children.
- Silent data loss: the orchestrator might look up a task record that no longer exists.

---

## Property Test

```rust
#[test]
fn gc_never_deletes_living_engram() {
    let ctx = TestContext::new();
    let substrate = ctx.file_substrate();
    let gc_threshold = 0.1;

    // Write 20 engrams with varying decay scores
    let living: Vec<_> = (0..10).map(|i| {
        let e = ctx.engram_with_decay_score(0.5 + i as f32 * 0.05); // scores 0.5..0.95
        substrate.write(&e).unwrap();
        e
    }).collect();

    let dead: Vec<_> = (0..10).map(|i| {
        let e = ctx.engram_with_decay_score(0.05 + i as f32 * 0.005); // scores 0.05..0.095
        substrate.write(&e).unwrap();
        e
    }).collect();

    substrate.gc(gc_threshold).unwrap();

    // All living engrams must still be present
    for e in &living {
        assert!(substrate.read(e.id()).unwrap().is_some(),
            "Living engram {:?} must survive GC", e.id());
    }

    // Dead engrams must be gone
    for e in &dead {
        assert!(substrate.read(e.id()).unwrap().is_none(),
            "Dead engram {:?} must be removed by GC", e.id());
    }
}
```

---

## Related Properties

- [substrate-idempotence.md](substrate-idempotence.md)
- [decay-monotonicity.md](decay-monotonicity.md)

## See also

- [../by-subsystem/subsystem-fs.md](../by-subsystem/subsystem-fs.md)
