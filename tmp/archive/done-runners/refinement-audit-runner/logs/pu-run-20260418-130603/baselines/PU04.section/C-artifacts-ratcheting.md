# C — Artifacts & Ratcheting (Docs 04, 05)

Parity analysis of `docs/04-verification/04-artifact-store.md` and
`docs/04-verification/05-ratcheting.md` vs the actual codebase.

---

## C.01 — `ArtifactStore` in-memory BLAKE3 content-addressed store

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §2 — `pub struct ArtifactStore { items: HashMap<ContentHash, Vec<u8>> }` with BLAKE3 hashes keying byte vectors.
**Reality**: `crates/roko-gate/src/artifact_store.rs:21-23` — `pub struct ArtifactStore { inner: HashMap<ContentHash, Vec<u8>> }`. Field named `inner` not `items`; minor cosmetic drift. `ContentHash` imported from `roko_core` (not redefined locally). Re-exported from `lib.rs:41`.

---

## C.02 — Store/retrieve/exists/len operations

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §3 — `store(&[u8]) -> ContentHash`, `get(&ContentHash) -> Option<&[u8]>`, `contains(&ContentHash) -> bool`, `len() -> usize`.
**Reality**: `artifact_store.rs:38-66` implements:
- `store(&mut self, content: &[u8]) -> ContentHash` — uses `ContentHash::of(content)` from roko-core, `.entry().or_insert_with()` for deduplication
- `retrieve(&self, hash: &ContentHash) -> Option<&[u8]>` (doc calls this `get`, code calls it `retrieve`)
- `exists(&self, hash: &ContentHash) -> bool` (doc calls this `contains`, code calls it `exists`)
- `len()`, `is_empty()` both present

Naming drift (`retrieve`/`exists` vs doc's `get`/`contains`) is cosmetic.

---

## C.03 — Deduplication semantics

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §5 — storing the same content twice returns the same hash without writing a second copy.
**Reality**: `artifact_store.rs:38-42` uses `entry(hash).or_insert_with(|| content.to_vec())`. Test `artifact_store_deduplicates` at `artifact_store.rs:102-109` confirms: two stores of same content → same hash, `len() == 1`.

---

## C.04 — Append-only / no-delete semantics

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §4 — no `delete`, `update`, or `clear` in the public API.
**Reality**: `artifact_store.rs:21-67` exposes only `new`, `store`, `retrieve`, `exists`, `len`, `is_empty`. Confirmed: no delete/update/clear methods. Append-only is structural, not just conventional.

---

## C.05 — Persistent artifact store (doc 04 §7 layout)

**Status**: NOT DONE (MEDIUM severity)
**Doc claim**: Doc 04 §7.1 describes `.roko/artifacts/{ab,cd}/{hash}` two-prefix-directory layout with `manifest.jsonl` mapping hashes to metadata. §7.3 mentions garbage collection after 30 days.
**Reality**: Zero filesystem persistence. The store is `HashMap<ContentHash, Vec<u8>>` in-memory only. Grep verification:
- `grep -rn '.roko/artifacts' crates/` → no matches
- `grep -rn 'artifact_manifest\|artifacts/manifest' crates/` → no matches
- No GC code for artifacts anywhere

**Fix sketch**: Either implement the `.roko/artifacts/` layout as a one-afternoon build (hash prefix directory + manifest.jsonl + time-based GC), or explicitly scope `ArtifactStore` to in-memory per-plan and mark doc 04 §7 "design — not started".

---

## C.06 — `ContentHash` is BLAKE3 (choice of hash)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §2.1 — BLAKE3 chosen for speed (5–15x faster than SHA-256), streaming support, keyed mode.
**Reality**: `ContentHash` imported from `roko_core` (actual hash impl lives in `crates/roko-core/src/hash.rs` or `kind.rs`; `ContentHash::of(bytes)` is the constructor used at `artifact_store.rs:39`). Test `artifact_store_hash_deterministic` at `artifact_store.rs:149-155` confirms `ContentHash::of(b"deterministic") == store.store(b"deterministic")` — meaning hashing is byte-identical across call paths (consistent with BLAKE3's determinism).

---

## C.07 — `GateRatchet` monotonic per-plan watermark

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 §3, §4 — `HashMap<String, u8>` keyed by plan_id, recording highest rung; `record_pass`, `highest_pass`, `can_regress`, `plan_count`, `clear`.
**Reality**: `crates/roko-gate/src/ratchet.rs:16-73`. Struct: `pub struct GateRatchet { passes: HashMap<String, u8> }`. All five public methods match doc exactly (including named parameter `rung: u8`). Re-exported from `lib.rs:53`. 13 tests at `ratchet.rs:78-205` cover every edge case in doc §9 and §11.

---

## C.08 — `GateRatchet` has zero runtime callers

**Status**: PARTIAL (HIGH severity)
**Doc claim**: Doc 05 §1, §2, §5 — ratchet "protects against convergence thrashing"; orchestrator "checks `can_regress` and decides what to do"; describes integration pattern `if verdict.passed { ratchet.record_pass(plan_id, rung) }`.
**Reality**: `grep -rn 'record_pass\|can_regress\|GateRatchet' crates/` returns matches **only in**:
- `crates/roko-gate/src/lib.rs` (re-export)
- `crates/roko-gate/src/ratchet.rs` (impl + tests)

**No callers** in `crates/roko-cli/src/orchestrate.rs`, no callers in any other crate. The ratchet is dead code from the runtime's perspective. Convergence thrashing — the specific failure mode doc 05 §2 describes — is not actually being prevented in the current wiring.
**Fix sketch**: Either:
1. Wire `record_pass` after each successful rung verdict and `can_regress` before escalation in orchestrate.rs; persist to `.roko/state/gate-ratchet.json` on executor snapshot.
2. Or delete `ratchet.rs` (206 LOC + 13 tests) and mark doc 05 as "design only".

---

## C.09 — Ratchet persistence to disk

**Status**: NOT DONE (MEDIUM severity)
**Doc claim**: Doc 05 §10 sketches `.roko/state/gate-ratchet.json` with `{plan-id: rung}` JSON map, loaded on `--resume`.
**Reality**: No `save`/`load_or_new` methods on `GateRatchet`. No `.roko/state/gate-ratchet.json` references anywhere. Ratchet state dies with the process. (Contrast with `AdaptiveThresholds.save/load_or_new` which does exist — see D.06.)
**Fix sketch**: Add `save(path)` / `load_or_new(path)` methods on `GateRatchet` mirroring `AdaptiveThresholds` pattern. Hook into executor snapshot write.

---

## C.10 — Content-addressed chain wiring (doc 04 §8)

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 04 §8 lists 4 places content-addressing appears: `ArtifactStore` / `Signal` / `FileSubstrate` / Episode logs, all BLAKE3 except episodes.
**Reality**:
- `ArtifactStore` → BLAKE3 via `ContentHash::of` (confirmed above).
- `Signal`/`Engram` → uses `ContentHash` via `roko-core`; orchestrate at `orchestrate.rs:11175-11185` builds `Kind::GateVerdict` engrams with `.derive(...)` (hash chain).
- `FileSubstrate` → see G.05 for signals.jsonl path; content hashing there via `Engram::builder` lineage.
- Episodes → `.roko/episodes.jsonl` is sequential (no hash), confirmed in `roko-learn/src/episode_logger.rs:90-217`.

Doc 04 §8's claim is correct. Content addressing is consistent across the three hashed paths. The chain is real; what's missing is the pipeline that *uses* the chain end-to-end (see G.08 for replay gap).

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 6 |
| PARTIAL | 2 (C.08 ratchet zero-callers, C.10 chain partial usage) |
| NOT DONE | 2 (C.05 artifact persistence, C.09 ratchet persistence) |

The foundations of content-addressing and ratcheting exist as well-tested modules (ArtifactStore 171 LOC, GateRatchet 206 LOC, 13 ratchet tests). Both are **completely unused from the runtime path** — no orchestrate.rs calls `record_pass`, `can_regress`, or `ArtifactStore::store` outside the gate verdict emission loop. Doc 04 §7 (persistent artifact layout) and Doc 05 §10 (ratchet persistence) are design sketches. The #2 wiring gap (after B.04 hardcoded dispatch) is the ratchet: convergence thrashing prevention is advertised but not active.

## Agent Execution Notes

### C.05 — Persistent Artifact Store

Keep this narrower than the doc.

Recommended slice:

1. add a content-addressed disk layout under `.roko/artifacts/`,
2. persist enough metadata to find artifacts again,
3. wire one real runtime producer before worrying about browsers or GC.

Acceptance criteria:

- artifacts survive restart,
- hashes remain authoritative,
- at least one production path writes artifacts to disk.

### C.08 / C.09 — GateRatchet Activation

Treat ratchet as long-running-runtime state, not as a theory exercise.

Recommended slice:

1. load / save ratchet state,
2. call `record_pass` on real successful progress,
3. consult `can_regress` where lower-rung backsliding would otherwise happen.

If runtime activation proves impossible, leave a concrete blocker note. Do not quietly keep dead code while claiming convergence protection is live.
