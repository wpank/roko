# Signal Lifecycle and Serialization

> From creation through graduation tiers to JSONL persistence and GC.

**Sources**: `crates/roko-core/src/signal.rs`, `crates/roko-fs/src/file_substrate.rs`

---

## 1. Creation

Signals are created via `Signal::builder(kind)`:

```rust
let signal = Signal::builder(Kind::Task)
    .body(Body::text("implement login"))
    .provenance(Provenance::agent("planner"))
    .build();
```

At build time, the builder:
1. Sets `created_at_ms` to `chrono::Utc::now().timestamp_millis()` unless
   pinned explicitly.
2. Computes `id` via `Signal::content_hash()` (BLAKE3 over kind + body +
   author + taint + lineage + tags).
3. Defaults `status` to `SignalStatus::Transient`, `balance` to `1.0`,
   `access_count` to `0`, `demurrage_paid` to `0.0`.

---

## 2. SignalStatus: The Four Tiers

Graduation is **monotonic** -- signals can only move forward, never backward.

```
Transient  -->  Working  -->  Consolidated  -->  Persistent
```

### Tier Semantics

| Tier | Retention | When | Pruning |
|---|---|---|---|
| `Transient` | Minutes | Default on creation | Aggressive: pruned when weight < threshold |
| `Working` | Task scope | Promoted on score threshold or Pulse graduation | Retained during active task |
| `Consolidated` | Cross-session | Promoted after gate pass | Feeds learning; survives restarts |
| `Persistent` | Permanent | Promoted after age + access criteria | Never auto-pruned |

`SignalStatus::is_durable()` returns `true` for `Consolidated` and `Persistent`.

### Transition Rules

**Transient to Working** (`promote_to_working(min_score)`):
- Precondition: `status == Transient`
- Gate: `score.effective() >= min_score`
- Error: `GraduationError::InvalidTransition` or `GraduationError::ScoreTooLow`

**Working to Consolidated** (`promote_to_consolidated()`):
- Precondition: `status == Working`
- No additional gate (caller decides, typically after a gate pass)
- Error: `GraduationError::InvalidTransition`

**Consolidated to Persistent** (`promote_to_persistent(min_age_secs, min_accesses)`):
- Precondition: `status == Consolidated`
- Gate 1: signal age >= `min_age_secs` seconds
- Gate 2: `access_count >= min_accesses`
- Errors: `InvalidTransition`, `InsufficientAge`, `InsufficientAccesses`
- Checked in order: status, then age, then accesses

### GraduationError

```rust
pub enum GraduationError {
    InvalidTransition { from: SignalStatus, to: SignalStatus },
    ScoreTooLow { required: f32, actual: f32 },
    InsufficientAge { required_secs: u64, actual_secs: u64 },
    InsufficientAccesses { required: u32, actual: u32 },
}
```

---

## 3. Demurrage and Access

### Balance

Every Signal carries a `balance: f64` in `[0.0, 1.0]`. This is the
demurrage-based economic value of the Signal. It starts at `1.0` and
decays via a configurable demurrage tick. When balance reaches zero,
the Signal is eligible for garbage collection.

### Touch

`Signal::touch()` resets balance to `1.0` and increments `access_count`.
This models "signals that are used stay alive." The access count is a
graduation precondition for `Consolidated -> Persistent`.

### Demurrage Paid

`demurrage_paid: f64` tracks the total balance lost to demurrage ticks.
It is monotonically increasing -- even if novelty gains partially offset
decay, this field only grows. Used for economic accounting and the
identity economy.

---

## 4. Decay

The `decay: Decay` field determines how a Signal's weight diminishes
over time. `Decay::apply(age_ms)` returns a multiplier in `[0.0, 1.0]`.

The effective weight at any point in time is:
```
weight = score.effective() * decay.apply(now_ms - created_at_ms)
```

### Decay Variants

| Variant | Formula | Constant shortcuts |
|---|---|---|
| `None` | `1.0` always | -- |
| `HalfLife { half_life_ms }` | `0.5^(age/hl)` | `THREAT` (2h), `OPPORTUNITY` (4h), `WISDOM` (24h), `GATE_VERDICT` (24h) |
| `Ttl { ttl_ms }` | `1.0` before, `0.0` after | -- |
| `Ebbinghaus { strength, scale_ms }` | `exp(-age/(strength*scale))` | -- |

Edge cases: negative age returns `1.0`. Zero half-life decays
immediately. Non-finite Ebbinghaus strength returns `0.0`.

---

## 5. JSONL Serialization

### Format

Signals are persisted as one JSON object per line in
`.roko/signals.jsonl` (previously `.roko/engrams.jsonl`). Each line
is a complete `serde_json::to_string(&signal)` serialization.

### File Substrate

`FileSubstrate` in `crates/roko-fs/src/file_substrate.rs` manages
the JSONL log:

```rust
pub struct FileSubstrate {
    root: PathBuf,             // directory containing signals.jsonl
    index: RwLock<HashMap<ContentHash, Signal>>,
    log: Mutex<File>,
}
```

On startup, `FileSubstrate::open(root)` replays the JSONL log into an
in-memory index. All subsequent reads are served from memory; writes
append to the JSONL file under a tokio `Mutex`.

### Migration

If `signals.jsonl` does not exist but `engrams.jsonl` does (the
previous canonical name), `FileSubstrate` replays from `engrams.jsonl`.
All subsequent writes go to `signals.jsonl`.

### Compaction

`FileSubstrate::compact()` rewrites the log from the current in-memory
index, removing deleted or pruned entries. Writes to a `.tmp` file
first and renames atomically.

### Generational GC

The `roko-fs` crate implements bounded JSONL generations. Older
generations are archived via the `ColdStore` trait before being
pruned from the hot substrate.

---

## 6. Content Hash Identity

Fields **included** in the content hash:
- `kind` (via `Kind::identity_key()`)
- `body` (via `Body::canonical_bytes()`)
- `provenance.author`
- `provenance.is_tainted()` (boolean byte)
- `lineage` (each parent hash)
- `tags` (key=value pairs in BTreeMap order)

Fields **excluded** from the content hash:
- `score` (can be recomputed by different scorers)
- `decay` (a policy, not content)
- `created_at_ms` (timing, not identity)
- `attestation` (layered on after creation)
- `emotional_tag` (affect metadata)
- `balance`, `status`, `access_count`, `demurrage_paid` (mutable state)
- `fingerprint` (derived from body)

This split ensures that the same Signal content always produces the
same hash regardless of when it was created, how it was scored, or
what attestations were attached.

---

## 7. HDC Fingerprinting

`Signal::compute_fingerprint()` sets the `fingerprint` field:

- `Body::Text(s)` -> `HdcVector::from_seed(s.as_bytes())`
- `Body::Json(v)` -> `HdcVector::from_seed(v.to_string().as_bytes())`
- `Body::Empty` -> `HdcVector::zeros()`
- `Body::Bytes(_)` -> skipped (no fingerprint)

The encoder version is always `ENCODER_VERSION_TEXT_V1 = 1`.
`ensure_fingerprint()` is idempotent -- it only computes if absent.

HDC operations on fingerprints:
- `signal.bind(other)` -> XOR binding of two fingerprints
- `Signal::bundle(signals)` -> majority-vote consensus
- `signal.at_position(n)` -> cyclic permutation for positional encoding

---

## 8. Full Lifecycle Example

```
1. Builder creates Signal       status=Transient, balance=1.0
2. Store::put() persists        appended to signals.jsonl
3. Score threshold met          promote_to_working(0.5) -> status=Working
4. Gate pass                    promote_to_consolidated() -> status=Consolidated
5. Time passes                  demurrage ticks reduce balance
6. Signal accessed              touch() resets balance, increments access_count
7. Maturity criteria met        promote_to_persistent(86400, 5) -> status=Persistent
8. Never auto-pruned            permanent archival
```

---

## 9. Source Files

| File | Contents |
|---|---|
| `crates/roko-core/src/signal.rs` | Signal, SignalBuilder, SignalStatus, GraduationError |
| `crates/roko-core/src/decay.rs` | Decay enum and apply logic |
| `crates/roko-fs/src/file_substrate.rs` | FileSubstrate JSONL persistence |
| `crates/roko-core/src/hash.rs` | ContentHash (BLAKE3) |
