# HDC Fingerprint — Encoder Versioning

> How the system manages encoder upgrades without invalidating existing Engram identities.

**Status**: Shipping  
**Crate**: `bardo-primitives`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

The `encoder_version: u32` field on `HdcFingerprint` tracks which version of the HDC
encoder produced the vector. When the encoder is upgraded (e.g., improved tokenization),
the version number increments. Old fingerprints are not invalidated — they remain valid
under their version. New Engrams get fingerprints under the new version. The Substrate
re-encodes old Engrams lazily (on access) or eagerly (via a background migration job).
Because `fingerprint` is excluded from the `ContentHash`, re-encoding never changes an
Engram's identity.

---

## Why Versioning Is Necessary

If the token projection function changes (e.g., different seed hash, different vocabulary),
the same body would produce a different `HdcVector`. Comparing a v1 fingerprint to a v2
fingerprint would yield a meaningless Hamming distance. The version number prevents this
by making cross-version comparison an error.

---

## Version Lifecycle

| Phase | Action |
|---|---|
| **Active** | Current production version; used for all new Engrams |
| **Legacy** | Older version; existing Engrams have valid fingerprints but no new ones are produced |
| **Deprecated** | Legacy version scheduled for re-encoding; background job queued |
| **Retired** | All Engrams re-encoded; version no longer present in Substrate |

---

## Invariant: Version Immutability Per Vector

A stored `HdcFingerprint` always reports the version that produced it. Updating the
`encoder_version` field without recomputing the vector is a corruption — the struct
does not expose a setter for `encoder_version` separately from `vector`.

---

## Re-encoding Protocol

```rust
<!-- source: crates/bardo-primitives/src/encoder.rs -->

/// Re-encode an Engram's fingerprint to the current encoder version.
/// Returns the new fingerprint, or None if the body is not encodable.
/// Does NOT modify the Engram's ContentHash.
pub fn reencode(engram: &Engram, encoder: &HdcEncoder) -> Option<HdcFingerprint> {
    encoder.encode(&engram.body)
}
```

The Substrate's re-encoding job:

```rust
<!-- source: crates/roko-fs/src/substrate.rs -->

/// Re-encode all fingerprints below the current encoder version.
pub fn migrate_fingerprints(
    &self,
    encoder: &HdcEncoder,
) -> FingerprintMigrationReport {
    let mut updated = 0;
    let mut skipped = 0;
    for id in self.warm_store.scan_all() {
        if let Ok(mut engram) = self.get_mut(&id) {
            let needs_update = engram.fingerprint
                .as_ref()
                .map(|f| f.encoder_version < encoder.version)
                .unwrap_or(false);
            if needs_update {
                engram.fingerprint = reencode(&engram, encoder);
                self.put(engram).ok();
                updated += 1;
            } else {
                skipped += 1;
            }
        }
    }
    FingerprintMigrationReport { updated, skipped }
}
```

---

## Current Version

```rust
<!-- source: crates/bardo-primitives/src/encoder.rs -->

pub const CURRENT_ENCODER_VERSION: u32 = 1;
```

---

## Invariants

1. `encoder_version` in a stored `HdcFingerprint` equals the version that produced the vector.
2. Changing `fingerprint` on an Engram does not change its `ContentHash`.
3. Cross-version comparison is rejected by `similarity_checked()`.
4. Re-encoding uses the Engram's original `Body` — no lossy intermediate step.
5. The current version constant in `bardo-primitives` is the source of truth for new encodings.

---

## Open Questions

- Should there be a compatibility matrix (v2 can compare to v1 via an adapter) for minor
  encoder changes? Not planned; full re-encoding is simpler.
- Should encoder version be included in the `canonical_encode()` call? No — fingerprints
  are excluded from the hash entirely, not partially.

## See Also

- [`00-overview.md`](00-overview.md) — why fingerprint is excluded from ContentHash
- [`02-encoding-pipeline.md`](02-encoding-pipeline.md) — the encoding function
- [`03-similarity-distance.md`](03-similarity-distance.md) — why cross-version comparison is invalid
