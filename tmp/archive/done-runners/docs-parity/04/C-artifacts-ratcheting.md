# C — Artifacts & Ratcheting (Docs 04, 05)

Post-audit refresh for storage and ratcheting status.

---

## Verdict

This section is **partial foundation**, not “missing system” and not “fully shipped runtime.”

That distinction matters:

- the content-addressed primitives are real
- the persisted artifact/ratchet story is still limited

---

## Artifact Store

### Shipped

- `ArtifactStore` exists as an in-memory, content-addressed, append-only store.
- It uses `ContentHash::of(...)` and deduplicates identical content.
- This is sufficient to document the primitive as real.

Key anchor:

- `crates/roko-gate/src/artifact_store.rs:21-66`

### Narrow

Do not describe the following as current runtime facts unless code lands:

- `.roko/artifacts/` disk layout
- persistent artifact manifests
- artifact GC policy
- broad cross-session artifact browsing

The docs should say the current store is an in-memory foundation with a clear future persistence seam.

### Important nuance

`GeneratedTestGate` also carries its own artifact-store trait for generated-test inputs. That is adjacent to, but not the same thing as, the top-level `artifact_store.rs` type.

---

## GateRatchet

### Shipped

- `GateRatchet` exists as a tested monotonic watermark primitive.
- The abstraction is valid and belongs in the architecture story as a real module.

Key anchor:

- `crates/roko-gate/src/ratchet.rs:16-74`

### Narrow

The parity materials should **not** describe ratcheting as an active, persisted orchestration guardrail today.

What is still partial:

- runtime use is not the central documented execution path
- persistence is not part of the current live story
- the docs should not imply full convergence-thrashing protection is already enforced end to end

---

## Post-Audit Wording

Use this posture:

- content-addressed verification foundations are real
- artifact persistence remains limited
- ratchet logic exists as a real primitive
- ratchet persistence/runtime activation remains future work

---

## What To Defer

Defer these from `04`:

- artifact catalog design
- artifact GC policy
- ratchet resume/load-save design
- long roadmap language around convergence economics

This section only needs truthful narrowing.
