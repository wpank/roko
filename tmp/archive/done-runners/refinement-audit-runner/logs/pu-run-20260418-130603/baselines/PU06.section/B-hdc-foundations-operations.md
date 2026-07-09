# B — HDC Foundations + Operations (Docs 04, 05, 06, 09)

Parity analysis of `docs/06-neuro/04-hdc-vsa-foundations.md`,
`docs/06-neuro/05-hdc-operations.md`, `docs/06-neuro/06-hdc-knowledge-encoding.md`,
and `docs/06-neuro/09-false-positive-math.md` vs the actual codebase.

---

## B.01 — `HdcVector` struct shape and dimension

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §"The Rust Implementation" — `HdcVector` is a 10,240-bit binary sparse distributed vector stored as `[u64; 160]` (1,280 bytes). Derives `Clone, Copy, Debug, Eq, PartialEq`. Doc 04 §"Dimensionality Analysis" cites `HDC_BITS = 10,240`, `HDC_WORDS = 160`, `HDC_BYTES = 1,280` and says 160 u64 words = 5 × 32-word AVX-512 passes.
**Reality**: `crates/roko-primitives/src/hdc.rs:19-26` defines `pub struct HdcVector { bits: [u64; 160] }` with exactly `#[derive(Clone, Copy, Debug, Eq, PartialEq)]` at `:19` plus optional rkyv archive derives gated at `:20-23` via `#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]`. The `160` constant appears 11 times (`:25, :81, :98, :108, :123, :151, :154, :158, :179, :203, :222`). `to_bytes()` at `:168` produces `[u8; 1280]`; `from_bytes()` at `:178` consumes the same. The 10_240.0 divisor appears at `:217` (scalar `similarity`) and `:233` (rkyv `similarity_archived`). Doc's claim of `[u64; 160]` = 10,240 bits = 1,280 bytes matches code exactly. No `[u32; 320]` representation — the code uses 64-bit slots, not 32-bit. Tests `hdc_bytes_roundtrip` at `:286-291` and `hdc_serde_roundtrip_json` at `:308-313` exercise the 1,280-byte round-trip.

---

## B.02 — `HdcVector::zeros()` / `random()` / `from_seed()` creation APIs

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §"The Rust Implementation" — three constructors: `zeros()` for all-zero vector, `random()` for pseudo-random, `from_seed(bytes)` for deterministic seeded generation using FNV-1a hashing followed by splitmix64 expansion. Identical seeds produce identical vectors.
**Reality**: `crates/roko-primitives/src/hdc.rs:80-82` defines `pub const fn zeros() -> Self { Self { bits: [0; 160] } }`. `random()` at `:86-103` derives two u64 halves from a fresh `Uuid::new_v4().as_u128()`, XORs them into splitmix64 state (with `0xA5A5_A5A5_5A5A_5A5A` fallback for zero seed at `:94-96`), then fills 160 words via `splitmix64(&mut state)`. `from_seed()` at `:193-208` implements FNV-1a with offset basis `0xcbf2_9ce4_8422_2325` at `:194` and prime `0x0100_0000_01b3` at `:197`, exactly matching the doc's "FNV-1a offset basis" claim in 06 §"Step 1". Zero-hash fallback is `0xA5A5_A5A5_5A5A_5A5A` at `:199-201`. The `splitmix64` mixer itself is the shared const fn at `:6-12` with Weyl increment `0x9E37_79B9_7F4A_7C15`. Determinism asserted by tests `hdc_from_seed_deterministic` (`:294-298`) and `hdc_from_seed_distinct` (`:301-305`, checks `similarity < 0.6`).

---

## B.03 — `bind()` (XOR) operation + involution property

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 §"Bind (XOR)" — `bind(A, B) = A ⊕ B` componentwise XOR. Self-inverse: `bind(bind(A, B), B) = A`. Commutative. Associative. Distributes over bundle. Rust code block shows 160-word XOR loop.
**Reality**: `crates/roko-primitives/src/hdc.rs:107-113` implements `pub fn bind(&self, other: &Self) -> Self` (marked `#[must_use]` at `:106`) with a 160-word XOR loop that mirrors the doc block byte-for-byte: `*slot = left ^ right` (line :110) inside a `bits.iter_mut().zip(self.bits.iter().zip(other.bits.iter()))` iteration (line :109). Involution verified by test `hdc_bind_involution` at `:262-267`: `a.bind(&b).bind(&b)` is checked for `(recovered.similarity(&a) - 1.0).abs() < 1e-6` (similarity ≈ 1.0). Distributivity and commutativity are algebraic consequences of XOR and are exercised implicitly by the knowledge encoding tests in `roko-neuro/src/hdc.rs:303-333` (directional causal encoding + query matching).

---

## B.04 — `bundle()` (majority vote) operation + tie-breaking rule

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 §"Bundle (Majority Vote)" — `bundle(A, B, C)[i] = majority(A[i], B[i], C[i])`. Ties break to 0 for determinism. Threshold rule `ones * 2 > len`. Empty bundle returns zeros.
**Reality**: `crates/roko-primitives/src/hdc.rs:117-138` implements `pub fn bundle(vectors: &[&Self]) -> Self` (`#[must_use]` at `:116`, doc at `:115`). Empty-input short-circuit at `:118-120` returns `Self::zeros()`. Per-bit majority threshold `if ones * 2 > len` at `:131` matches doc exactly — this is a strict inequality so ties (`ones * 2 == len`) leave the bit at 0. The outer word loop runs 160 iterations (`:124`); the inner bit loop runs 64 times per word (`:126`). Tie-break-to-zero verified by test `hdc_bundle_tie_rule` at `:276-283`: one vector with `a.bits[0] = 1` and one with `b.bits[0] = 0` yields `bundled.bits[0] == 0`.
**Notes**: The `roko-index/src/hdc.rs:51-72` has a parallel non-generic `bundle(vectors: &[[u64; WORDS]]) -> [u64; WORDS]` using `let threshold = len / 2;` at `:56` and `if ones > threshold` at `:65`. Mathematically equivalent to `ones * 2 > len` for integer inputs (both break ties to zero). That duplicate lives alongside its own local `bind`, `hamming_distance`, `splitmix64`, `fnv1a`, and `vector_from_seed`, so `roko-index` does not depend on `roko_primitives::HdcVector`.

---

## B.05 — `permute()` (cyclic shift) operation

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 §"Permute (Cyclic Shift)" — `permute(A, k) = cyclic_left_shift(A, k)`. Group operation: `permute(permute(A, j), k) = permute(A, j+k)`. Preserves similarity structure. Code block shows word-shift + bit-shift scheme with 64-bit word cross-over using `(index + 160 - word_shift) % 160`.
**Reality**: `crates/roko-primitives/src/hdc.rs:142-164` implements `pub fn permute(&self, n: usize) -> Self` (doc at `:140`, `#[must_use]` at `:141`). Modulo wrap via `let bits_len = self.bits.len() * 64;` and `let n = n % bits_len;` at `:143-144` (so `bits_len = 160 * 64 = 10,240`). Identity early-return for `n == 0` at `:145-147`. Word-split at `:149-150` (`word_shift = n / 64`, `bit_shift = n % 64`). Word-shift index `(index + 160 - word_shift) % 160` at `:154`; bit-shift zero fast path `self.bits[src0]` at `:155-156`; cross-word bit-shift `(self.bits[src0] << bit_shift) | (self.bits[src1] >> (64 - bit_shift))` at `:159` matches the doc code block exactly. No dedicated permute unit test in `roko-primitives`, but heavy usage in `roko-neuro/src/hdc.rs` for directional causal encoding: query probes (`:31-32`), cause/effect bindings in `encode_causal_link` (`:69-78`), plus `role_hv("cause").permute(CAUSE_SHIFT)` and `role_hv("effect").permute(EFFECT_SHIFT)` with constants `CAUSE_SHIFT = 1` / `EFFECT_SHIFT = 2` (`:5-6`). The test `directional_causal_encoding_distinguishes_reversal` at `roko-neuro/src/hdc.rs:303-317` relies on permute to make "cause -> effect" quasi-orthogonal to "effect -> cause".

---

## B.06 — `similarity()` (normalized Hamming) operation

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 §"Similarity (Hamming Distance)" — `sim(A, B) = 1 - hamming_distance(A, B) / D`. Returns f32 in [0, 1]. XOR + POPCNT inner loop. Self-similarity = 1.0. Random pairs ≈ 0.5.
**Reality**: `crates/roko-primitives/src/hdc.rs:211-218` implements `pub fn similarity(&self, other: &Self) -> f32` (doc comment at `:210`). Loop at `:212-215` initialises `let mut differing_bits = 0u32;` then accumulates `differing_bits += (left ^ right).count_ones();` across 160 word pairs. The `u16::try_from(differing_bits).unwrap_or(u16::MAX)` safety clamp at `:216` handles the theoretical overflow case documented at doc 05 §"Similarity (Hamming Distance)" "Note" paragraph. Final `1.0_f32 - (f32::from(differing_bits) / 10_240.0_f32)` at `:217` normalizes to [0, 1]. Self-similarity ≈ 1.0 verified by test `hdc_similarity_self` at `:270-273` (`(vector.similarity(&vector) - 1.0).abs() < 1e-6`). Distinct seeds < 0.6 verified by `hdc_from_seed_distinct` at `:301-305` (`a.similarity(&b) < 0.6`).
**Notes**: `roko-index/src/hdc.rs:114-117` provides `HdcFingerprint::similarity(&self, other: &Self) -> f64` — same algorithm but returns `f64` instead of `f32`, normalising by `TOTAL_BITS as f64` (`:116`). `roko-index/src/hdc.rs:123-125` also exposes a free `pub fn similarity(a: &HdcFingerprint, b: &HdcFingerprint) -> f64` wrapper.

---

## B.07 — `similarity_archived()` zero-copy rkyv variant

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 §"Zero-Copy Similarity (rkyv)" — feature-gated variant `similarity_archived(&self, archived: &ArchivedHdcVector) -> f32` reads directly from the mmap'd buffer on little-endian platforms.
**Reality**: `crates/roko-primitives/src/hdc.rs:226-234` implements exactly that signature behind `#[cfg(feature = "rkyv")]` at `:225`. Loop structure at `:227-231` mirrors scalar `similarity()`, iterating over `self.bits.iter().zip(archived.bits.iter())`. The conversion `let right_u64: u64 = (*right).into();` at `:229` unpacks the archived `u64` to native endianness. Final normalisation at `:233` matches the scalar path (`1.0_f32 - (f32::from(differing_bits) / 10_240.0_f32)`). The rkyv derive is gated at `:20-23` (`#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]`). The `ArchivedHdcVector` type is generated by `rkyv::Archive` — no manual declaration in this file. Doc claim of "reads directly from the mmap'd buffer on little-endian platforms" matches the `:222-224` doc comment verbatim.

---

## B.08 — `BundleAccumulator` incremental vote-tracking bundle

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 04 §"BundleAccumulator with vote tracking" and Doc 05 §"BundleAccumulator (Designed, Not Yet Implemented)" — specify a `BundleAccumulator { votes: Vec<i32>, count: usize }` with `add()`, `add_weighted()`, `finish()`, and `decay()` methods. 40 KB memory (10,240 × 4 bytes). Bipolar +1/-1 per-bit voting; `finish()` thresholds at 0 with ties → 0. `decay(factor)` multiplies all votes by a scalar for controlled forgetting.
**Reality**: `rg 'BundleAccumulator' crates/` returns **zero matches**. No per-bit vote-count accumulator exists anywhere in `roko-primitives`, `roko-neuro`, `roko-index`, `roko-learn`, or anywhere else. Incremental bundling is not available — every caller must materialize a `Vec<&HdcVector>` and call `HdcVector::bundle(&refs)` in one shot. The `roko-neuro/src/hdc.rs:225-228` helper is exactly this pattern: `fn bundle(vectors: Vec<HdcVector>) -> HdcVector { let refs = vectors.iter().collect::<Vec<_>>(); HdcVector::bundle(&refs) }`. Doc 04 §"Implementation Details: BundleAccumulator with vote tracking" (docs/06-neuro/04-hdc-vsa-foundations.md:295-427) gives a ~130-line reference implementation that does not match any compiled code. Doc 05 §"BundleAccumulator (Designed, Not Yet Implemented)" (05-hdc-operations.md:159-194) already acknowledges the gap.
**Fix sketch**: Doc 05 already labels the accumulator "Designed, Not Yet Implemented". Doc 04 §"BundleAccumulator with vote tracking" should adopt the same "Design" framing. If demanded by a learning use case, implement `BundleAccumulator` in `roko-primitives` behind a non-default feature flag first.

---

## B.09 — `ItemMemory` codebook for named concept vectors

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 04 §"ItemMemory codebook" and Doc 06 §"Role vector registry" — `ItemMemory { entries: HashMap<String, HdcVector>, sorted_keys: Vec<String>, dirty: bool }` with `insert()`, `insert_seeded()`, `get()`, `top_k()`, `nearest()`, `len()`, `is_empty()`. Used to map role names to vectors and to do nearest-neighbor cleanup for decomposition.
**Reality**: `rg 'ItemMemory' crates/` returns **zero matches**. No codebook type exists anywhere. Role vectors are generated on-demand inside `roko-neuro/src/hdc.rs:217-219` via `fn role_hv(role: &str) -> HdcVector { HdcVector::from_seed(format!("role:{role}").as_bytes()) }` — no registry, no caching, no reverse lookup. `roko-index/src/hdc.rs:130-143` does the same pattern for `SymbolKind` using a hard-coded `match` statement (seeds `b"roko:role:function"`, `b"roko:role:struct"`, etc.). Doc 04 §"ItemMemory codebook" (04-hdc-vsa-foundations.md:429-530) gives a ~100-line spec with `HashMap<String, HdcVector>` storage, `insert`, `insert_seeded`, `get`, `top_k`, `nearest`, `len`, `is_empty`, and a `build_role_codebook` helper — none of that exists in code. Doc 06 §"Role vector registry" (06-hdc-knowledge-encoding.md:482-506) additionally claims the encoder holds `role_registry: ItemMemory, domain_codebook: ItemMemory, kind_codebook: ItemMemory` with 12 standard role seeds pre-populated; see B.12 — actual struct is a zero-sized `KnowledgeHdcEncoder` unit type with no fields.
**Fix sketch**: Mark doc 04 §"ItemMemory codebook" as design only. The on-demand `role_hv` pattern in `roko-neuro` is sufficient for encoding (because `from_seed` is deterministic), but any nearest-neighbor cleanup for resonator networks or structured-query decomposition would require this type.

---

## B.10 — `ResonatorNetwork` factor decomposition

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 04 §"ResonatorNetwork (Frady et al. 2020)" and Doc 05 §"ResonatorNetwork (Designed, Not Yet Implemented)" — `ResonatorNetwork { config: ResonatorConfig }` with `decompose(composite, codebooks) -> ResonatorResult`. Iterated projection algorithm for recovering factors of `z = bind(x1, ..., xF)`. `ResonatorConfig { max_iterations: 50, convergence_threshold: 0.001, early_termination_sim: 0.9 }`.
**Reality**: `rg 'ResonatorNetwork|ResonatorConfig|ResonatorResult' crates/` returns **zero matches**. No factor-decomposition mechanism exists. The structured-query unbinding technique in doc 06 §"Unbinding for Decomposition" (06-hdc-knowledge-encoding.md:286-298) is not implemented either (see B.13). Doc 04 §"ResonatorNetwork (Frady et al. 2020)" (04-hdc-vsa-foundations.md:532-716) and Doc 05 §"Implementation Details: ResonatorNetwork Factor Decomposition" (05-hdc-operations.md:542-646) give ~200 lines of reference pseudocode including a `decompose` signature that depends on `&[&ItemMemory]` — so resonator work is gated on B.09 as well.
**Fix sketch**: Keep the doc label "Designed, Not Yet Implemented". Implementation requires `ItemMemory` (B.09) first. The use case — decomposing a bundled knowledge entry into its role-filler pairs — is nice-to-have, not blocking any current feature.

---

## B.11 — Fractional binding / `DecayingBundleAccumulator` / `OnlineBundler`

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 05 §"Fractional Binding (FHRR Extension)" shows `fractional_bind(&self, other: &Self, alpha: f64, seed: u64) -> Self`. Doc 05 §"Weighted Bundling via Vote Accumulator" specifies `OnlineBundler { bundle: HdcVector, total_weight: f64 }` with stochastic-rounding weighted bundling. Doc 04 §"DecayingBundleAccumulator" specifies a `Vec<f32>` vote accumulator with `decay_factor` applied on every `add()`.
**Reality**: `rg 'FractionalBinding|fractional_bind|DecayingBundleAccumulator|OnlineBundler|stochastic_decay|word_trigram_fingerprint' crates/` returns **zero matches** across the whole workspace. Doc 05 §"Advanced Operations: Beyond the Core Four (2024-2025)" (05-hdc-operations.md:650-867) gives reference code for `fractional_bind`, `OnlineBundler::add`, `word_trigram_fingerprint`, and `stochastic_decay`, and ends with a "Test criteria" block (05-hdc-operations.md:858-866) covering all four — none of these functions exist in the codebase. Doc 04 §"DecayingBundleAccumulator with temporal weighting" (04-hdc-vsa-foundations.md:718-821) similarly provides a full `DecayingBundleAccumulator` implementation that is absent from the workspace.
**Fix sketch**: Doc 04 and Doc 05 should label the "Advanced Operations" section explicitly as "Design roadmap". These are extension proposals, not shipped APIs. Tests in the doc are aspirational.

---

## B.12 — `KnowledgeHdcEncoder` automatic encoding pipeline

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 06 §"Implementation Details: Automatic HDC Encoding Pipeline" specifies a `pub struct KnowledgeHdcEncoder { role_registry: ItemMemory, domain_codebook: ItemMemory, kind_codebook: ItemMemory }` with `new() -> Self` pre-populating 12 role names and 6 kind names, and `encode(&mut self, entry: &KnowledgeEntry) -> HdcVector` producing a bundle of kind/content/tag/domain bindings. Wired into `NeuroStore::ingest()` to auto-fill `hdc_vector`.
**Reality**: `crates/roko-neuro/src/hdc.rs:8-9` defines `#[derive(Debug, Default, Clone, Copy)] pub(crate) struct KnowledgeHdcEncoder;` — a **zero-sized unit struct** with no `role_registry`, no `domain_codebook`, no `kind_codebook` fields. The `impl` (`:20-111`) has no `new()` constructor, no `&mut self` methods, and no `ItemMemory` fields to populate. `encode_entry(self, entry: &KnowledgeEntry) -> HdcVector` at `:21-27` dispatches to `encode_causal_link` at `:61-110` when `entry.kind == KnowledgeKind::CausalLink`, else to `encode_generic_entry` at `:36-59`. `encode_query(self, topic: &str) -> HdcVector` at `:29-34` bundles `text_hv(topic)` plus permuted cause/effect probes — used for search queries so cause-only/effect-only queries can hit causal-encoded entries. The generic encoder at `:36-59` bundles `text_hv(content)` + `role_hv("kind").bind(&text_hv(kind.as_str()))` + optional tag bundle + optional `role_hv("source").bind(&text_hv(trimmed))`. The causal encoder at `:61-110` bundles content + kind-binding + `role_hv("cause").permute(CAUSE_SHIFT).bind(&text_hv(cause))` + `role_hv("effect").permute(EFFECT_SHIFT).bind(&text_hv(effect))` + `role_hv("causal_edge").bind(&cause.permute(1).bind(&effect.permute(2)))` + `role_hv("strength").bind(&strength_hv(strength))` + optional domain binding + optional conditions bundle + optional non-structural tag bundle. Constants `CAUSE_SHIFT = 1` and `EFFECT_SHIFT = 2` at `:5-6`. `CausalLinkParts` helper at `:11-18` parses cause/effect from structured tags (`cause:X`, `effect:Y`, `domain:Z`, `strength:w`, `condition:...`) or by pattern-matching on content (arrows `->`, `=>`, `→`, plus English connectives "causes", "leads to", "results in", "triggers", "drives" via `parse_causal_content` at `:140-170`). Strength bins are discretised via `strength_hv` at `:212-215` (`format!("strength:{bin}")` where `bin = (s.clamp(0.0, 1.0) * 5.0).round() as u8`). Role seeds use `format!("role:{role}")` at `:217-219` (`role_hv`), matching doc's `b"role:domain"` naming convention. `text_hv` at `:221-223` normalises text via `normalize_text` at `:230-243` (ASCII-lowercase, alnum+whitespace only, split/join). Bundling helper `bundle(Vec<HdcVector>)` at `:225-228` is a thin wrapper around `HdcVector::bundle(&refs)`. Wired into `MemoryIndex::search` at `roko-neuro/src/knowledge_store.rs:683` via `KnowledgeHdcEncoder.encode_query(query)` and into `fingerprint_entry` at `knowledge_store.rs:712-719` via `KnowledgeHdcEncoder.encode_entry(entry)` (with a short-circuit at `:713-717` for pre-computed `hdc_vector` bytes). Auto-fill happens in `ensure_hdc_vector` at `knowledge_store.rs:738-747` (hdc feature-gated at `:711, :721, :737`). Tests `directional_causal_encoding_distinguishes_reversal` (`:303-317`) and `causal_query_encoding_matches_both_cause_and_effect` (`:320-333`) validate the causal + query paths.
**Fix sketch**: Update doc 06 §"Implementation Details: Automatic HDC Encoding Pipeline" to reflect the actual unit-struct shape: no `ItemMemory` fields, no stateful codebook, no `&mut self`, no `new()`. Document (a) the causal-vs-generic dispatch, (b) the `encode_query()` method with cause/effect probes, (c) text normalisation, (d) `CausalLinkParts` tag-and-content parsing, and (e) the actual role names used (`kind`, `source`, `cause`, `effect`, `causal_edge`, `strength`, `domain`, `condition`) rather than the 12-name registry the doc currently claims. Reference `MemoryIndex` (the real type), not `KnowledgeMemoryIndex`.

---

## B.13 — `query_by_role` / `unbind_role` / structured query API

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 06 §"Structured query API" specifies `NeuroStore::query_by_role(role_name, filler_name) -> Vec<(usize, f32)>` and `NeuroStore::unbind_role(entry_hv, role_name) -> HdcVector`, and a `query_multi(role_fillers: &[(&str, &str)]) -> Vec<(usize, f32)>` variant. Filters at similarity > 0.526.
**Reality**: `rg 'query_by_role|unbind_role|query_multi' crates/roko-neuro/` returns **zero matches**. The only `query_by_role` hit anywhere in the workspace is `crates/roko-learn/src/costs_db.rs:945` — a completely unrelated test function `fn costs_db_query_by_role()` for a cost-lookup SQL-style API. No structured query decomposition exists on `MemoryIndex` or `NeuroStore`. Similarity search runs via `MemoryIndex::search` (`roko-neuro/src/knowledge_store.rs:678-696`, feature-gated on `hdc` via `:642`): it encodes the query topic once with `KnowledgeHdcEncoder.encode_query(query)` at `:683`, maps every indexed entry into a `MemoryHit { entry, similarity: query_fingerprint.similarity(&indexed.fingerprint) as f64 }` at `:684-691`, sorts via `compare_hits` (defined at `:788`), and truncates to `limit` — no role filtering, no unbinding, no multi-role bundling, no 0.526 threshold. The `encode_query` method does bundle `text_hv(topic)` with permuted cause/effect probes (`roko-neuro/src/hdc.rs:29-34`), which is a weaker form of structured query (it matches causal-encoded entries whose `cause` or `effect` contains the topic) but there is no API to query by an arbitrary `(role, filler)` pair.
**Fix sketch**: Mark doc 06 §"Structured query API" (06-hdc-knowledge-encoding.md:508-582) as design-only. The existing `encode_query` + brute-force rank satisfies the top-k search claim but does not support the role-specific filtering the doc describes. Future work: add `MemoryIndex::query_by_role`, `query_multi`, and `unbind_role` using the existing `role_hv` helper and the 0.526 threshold constant from B.15.

---

## B.14 — Three-tier search strategy (Bloom + approximate + exact)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 06 §"Three-Tier Search Strategy" describes a Bloom + reduced-precision + full-comparison pipeline, with Bloom rejection at 90-95%, Tier 2 pruning at 80-90% more. Doc 04 §"Three-tier search algorithm" specifies `ThreeTierIndex { bloom: BloomFilter, vectors: Vec<HdcVector>, config: ThreeTierConfig }` with `num_hash_functions=8, bloom_bits=1M, approx_words=32, approx_threshold=0.51`.
**Reality**: `rg 'ThreeTierIndex|ThreeTierConfig|BloomFilter' crates/` returns **zero matches**. Search is pure brute-force across `self.entries.iter()` at `crates/roko-neuro/src/knowledge_store.rs:684-691`: a flat `map(|indexed| MemoryHit { ..., similarity: query_fingerprint.similarity(&indexed.fingerprint) as f64 })` over every indexed entry, followed by a full sort + truncate. No LSH, no Bloom filter, no approximate-similarity first-pass, no hyperplane hashing, no partial-word comparison. At N < 100K this is explicitly documented as acceptable (~1.3 ms) by doc 06 §"Three-Tier Search Strategy" lead paragraph (06-hdc-knowledge-encoding.md:211-213). Doc 04 §"Three-tier search algorithm" (04-hdc-vsa-foundations.md:891-987) gives a ~100-line design with `ThreeTierIndex { bloom, vectors, config }`, `ThreeTierConfig { num_hash_functions: 8, bloom_bits: 1 << 20, approx_words: 32, approx_threshold: 0.51 }`, and `BloomFilter { bits, num_hashes, hyperplanes }`. None of that is in code.
**Fix sketch**: Keep the design in docs. Implement only if a real knowledge base exceeds 100K entries and search latency becomes a bottleneck. A first step would be adding `approx_similarity(n_words: usize)` to `HdcVector` in `roko-primitives`.

---

## B.15 — 0.526 Bonferroni threshold for 100K-vocabulary false positives

**Status**: NOT DONE
**Severity**: HIGH
**Doc claim**: Doc 09 §"Threshold Selection by Vocabulary Size" — 0.526 is the recommended per-comparison threshold for ≤100K entries, guaranteeing <1% overall FPR after Bonferroni correction. Doc 09 §"For Neuro Query API" shows `pub struct NeuroQuery { ..., pub min_similarity: f64 }` with default 0.526 for cross-domain, 0.51 for within-domain. Doc 06 §"Structured query API" and Doc 05 §"Similarity" Interpretation both cite 0.526 as the threshold for meaningful similarity.
**Reality**: `rg '0\.526' crates/` returns **zero matches**. No named constant, no literal, no `SIMILARITY_THRESHOLD`, no `MIN_SIMILARITY`, no `BONFERRONI_` anywhere in the Rust codebase. All `0.526` mentions live in the markdown docs (`docs/06-neuro/05, 06, 08, 09, 11, 12, 16`, `docs/00-architecture/*`, `docs/08-chain/*`). Current search in `MemoryIndex::search` at `roko-neuro/src/knowledge_store.rs:693-694` sorts all entries by similarity and truncates to `limit` — no threshold filter applied. `compare_hits` at `:788` is a `fn(&MemoryHit, &MemoryHit) -> std::cmp::Ordering` f64 ordering helper with no statistical significance cutoff. Doc 09 §"Threshold Table" (09-false-positive-math.md:57-70) and §"Threshold Selection by Vocabulary Size" (`:87-96`) spell out the derivation (Z = 5.26, per-comparison FP = 7.3e-8, Bonferroni 1% over 100K). Doc 09 §"For Neuro Query API" (`:156-168`) shows a `pub struct NeuroQuery { ..., pub min_similarity: f64 }` with defaults 0.526 for cross-domain and 0.51 for within-domain — `rg 'NeuroQuery' crates/roko-neuro/` also returns **zero matches**, so that struct doesn't exist either.
**Fix sketch**: Add `pub const CROSS_DOMAIN_SIMILARITY_THRESHOLD: f32 = 0.526;` and `pub const WITHIN_DOMAIN_SIMILARITY_THRESHOLD: f32 = 0.51;` to `roko-primitives/src/hdc.rs`. Wire an optional `min_similarity: Option<f64>` parameter into `MemoryIndex::search`. Introduce `pub struct NeuroQuery { topic, limit, min_similarity }` as doc 09 specifies. Doc 09 §"Current Status and Gaps" (`:189-193`) already admits "Missing: Configurable similarity threshold in query API" — this is the matching code-side gap.

---

## B.16 — `fingerprint()` / `text_fingerprint()` convenience helpers

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §"Convenience Functions" and Doc 06 §"Step 1: Concept Vector Generation" — two free functions: `fingerprint(value: &impl serde::Serialize) -> HdcVector` using `serde_json::to_vec` then `from_seed`, and `text_fingerprint(text: &str) -> HdcVector` wrapping `HdcVector::from_seed(text.as_bytes())`.
**Reality**: `crates/roko-primitives/src/hdc.rs:242-245` defines `pub fn fingerprint(value: &impl serde::Serialize) -> HdcVector` (doc at `:237-240`, `#[must_use]` at `:241`) using exactly `let seed = serde_json::to_vec(value).unwrap_or_default(); HdcVector::from_seed(&seed)`. `crates/roko-primitives/src/hdc.rs:253-255` defines `pub fn text_fingerprint(text: &str) -> HdcVector { HdcVector::from_seed(text.as_bytes()) }` (doc at `:247-251`, `#[must_use]` at `:252`). Determinism verified by tests `hdc_fingerprint_is_deterministic` at `:333-337` (`fingerprint(&json!({"a": 1, "b": [2, 3]}))` is stable across calls) and `hdc_text_fingerprint_is_deterministic` at `:340-344`. Call sites:
- `fingerprint(&signal.body)` used in `roko-fs/src/file_substrate.rs:280` and `roko-serve/src/routes/webhooks.rs:183` (both hash an incoming signal body into a deterministic HDC vector for indexing)
- `text_fingerprint(&text)` imported in `roko-neuro/src/context.rs:17`, `roko-learn/src/episode_logger.rs:37`, `roko-dreams/src/imagination.rs:13`, `roko-dreams/src/hypnagogia.rs:12`, and `roko-dreams/src/cycle.rs:31`
- `fingerprint` also re-imported in the test section of `roko-serve/src/routes/webhooks.rs:486`
The `roko-learn/src/pattern_discovery.rs` crate uses `HdcVector` directly (via `use roko_primitives::HdcVector;` at `:41`) but does not call the free `fingerprint` or `text_fingerprint` helpers.

---

## B.17 — Code-symbol fingerprinting (`fingerprint_symbol`, `fingerprint_file`)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 06 §"Code Symbol Fingerprinting (roko-index)" — `role_vector_for_kind(SymbolKind) -> HdcVector` with per-kind seeds, `encode_name(&str) -> HdcVector` using character trigrams and position-permutation, and `fingerprint_symbol(kind, name) -> HdcVector` bundling kind + name vectors.
**Reality**: `crates/roko-index/src/hdc.rs:130-143` defines `fn role_vector(kind: &SymbolKind) -> [u64; WORDS]` (private, not `role_vector_for_kind` as doc names it) with seeds `b"roko:role:function"`, `b"roko:role:struct"`, `b"roko:role:enum"`, `b"roko:role:trait"`, `b"roko:role:const"`, `b"roko:role:type"`, `b"roko:role:module"`, `b"roko:role:impl"`, and catch-all `b"roko:role:unknown"` for any other `SymbolKind` variant. `encode_name(name: &str) -> [u64; WORDS]` at `:148-163` has a short-name short-circuit (`chars.len() < 3` seeds the raw bytes at `:150-152`), then takes character trigrams via `chars.windows(3).map(|w| { let trigram: String = w.iter().collect(); vector_from_seed(trigram.as_bytes()) })` at `:154-160` and bundles — **no position permutation** (doc 06 §"Trigram-Based Name Encoding" at 06-hdc-knowledge-encoding.md:179-193 shows `.permute(pos)` per trigram; code omits it and uses raw trigram bundling). `pub fn fingerprint_symbol(symbol: &Symbol, context: &[u8]) -> HdcFingerprint` at `:173-181` does `bind(role_vec, bundle(&[name_vec, ctx_vec]))` — two-way bundle (name + context) XOR-bound with the role vector. Doc 06 shows `HdcVector::bundle(&[&kind_hv, &name_hv])` directly — actual code (a) adds context bytes as a third ingredient via `vector_from_seed(context)`, and (b) binds rather than bundles the role. `pub fn fingerprint_file(source: &SourceFile) -> HdcFingerprint` at `:187-206` bundles per-symbol fingerprints (re-passing `source.content.as_bytes()` as context to each), with `source.symbols.is_empty()` fallback to a content-seeded vector at `:188-192`. The `HdcFingerprint` wrapper at `:99-118` exposes `words()` and `similarity() -> f64`.
**Notes**: Two drifts from doc 06 §"Code Symbol Fingerprinting" to note: (1) seed prefix is `roko:role:*` in code but unqualified `symbol:*` in docs/06-neuro/06-hdc-knowledge-encoding.md:160-169; (2) `fingerprint_symbol` uses `bind(role, bundle(name, context))` not `bundle(kind, name)` — the doc's bundle-of-two is simpler than the real bind-of-bundle-of-three. Tests at `roko-index/src/hdc.rs:223-354` cover: identical symbols (`:223-233`), similar names (`:236-247`), different kinds (`:250-262`), completely different symbols (`:265-275`), file fingerprints (`:278-306`), `similarity` function consistency (`:309-315`), `<1 µs` perf budget (`:318-335`), self-similarity (`:338-347`), and short names (`:350-354`). 10 tests total in this module, not 11.

---

## B.18 — HDC split across crates (primitives, neuro, index, learn)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 "Key sources" cites `crates/bardo-primitives/src/hdc.rs` for `HdcVector`, `crates/roko-index/src/hdc.rs` for code fingerprints. Doc 05 and Doc 06 refer to `bardo-primitives/src/hdc.rs`. The rename to `roko-primitives` is noted in doc 04 line 208: "current implementation in `bardo-primitives` (to be renamed `roko-primitives`)".
**Reality**: Four HDC source locations are distinct:
- `crates/roko-primitives/src/hdc.rs` (345 LOC, 10 tests) — core `HdcVector` type, 4 operations (`bind`, `bundle`, `permute`, `similarity`), `similarity_archived` under rkyv, serde, `from_seed`, `to_bytes`/`from_bytes`, `random`, `zeros`, plus free `fingerprint()` and `text_fingerprint()` helpers. Doc 04 is the authoritative spec.
- `crates/roko-neuro/src/hdc.rs` (334 LOC, 4 tests) — `KnowledgeHdcEncoder` unit struct with generic + causal-link encoders + `encode_query`, role/text helpers, causal-content parsing (arrows + English connectives), strength discretisation, text normalisation. Implements the Doc 06 pipeline (partial, see B.12).
- `crates/roko-index/src/hdc.rs` (355 LOC, 10 tests) — parallel HDC implementation inlined for code-symbol fingerprinting: its own `splitmix64`, `fnv1a`, `bundle`, `bind`, `hamming_distance`, `vector_from_seed`. **Does not depend on `roko_primitives`** — inlines its own `[u64; WORDS]` (WORDS=160) storage and helpers. Exports `HdcFingerprint`, `fingerprint_symbol`, `fingerprint_file`, and a free `similarity` wrapper.
- `crates/roko-learn/src/hdc_clustering.rs` (498 LOC, 10 tests) — consumer crate using `roko_primitives::HdcVector` (`:34`) for k-medoids PAM clustering with farthest-first seeding. Uses `d(a, b) = 1.0 - vectors[i].similarity(&vectors[j])` as distance metric (`:164`).

`rg 'use roko_primitives::(HdcVector|hdc)' crates/` returns 10 Rust files (plus `roko-primitives/README.md`): `roko-neuro/src/knowledge_store.rs`, `roko-neuro/src/context.rs`, `roko-neuro/src/hdc.rs`, `roko-learn/src/episode_logger.rs`, `roko-learn/src/pattern_discovery.rs`, `roko-learn/src/hdc_clustering.rs`, `roko-dreams/src/imagination.rs`, `roko-dreams/src/hypnagogia.rs`, `roko-dreams/src/cycle.rs`, `roko-serve/src/routes/webhooks.rs`. Additionally `roko-fs/src/file_substrate.rs:280` and `roko-serve/src/routes/webhooks.rs:183` reference `roko_primitives::hdc::fingerprint` via a qualified path rather than a `use` statement. A grep for `roko_primitives` (any usage) returns 13 files total. The rename `bardo_primitives` → `roko_primitives` has landed everywhere in code, but the doc key-sources blocks in docs/06-neuro/04-, 05-, and 06- still point at `bardo-primitives/src/hdc.rs`.
**Notes**: The `roko-index` duplicate HDC implementation is a parity oddity — it could be replaced with `roko_primitives::HdcVector` operations. The `HdcFingerprint` wrapper returns `f64` similarity instead of `f32`, which is the only user-visible divergence from `HdcVector::similarity`. The `roko-primitives/src/lib.rs` module at `:17-21` exposes `pub mod hdc;` and `pub use hdc::HdcVector;` alongside `tier::{InferenceTier, T2_VITALITY_THRESHOLD, TierError, TierRouter}` — HDC shares this crate with tier routing types.

---

## B.19 — Explicit SIMD intrinsics (AVX-512 / AVX2) for bind and similarity

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 04 §"SIMD intrinsics strategy" (04-hdc-vsa-foundations.md:823-889) specifies a runtime-dispatched three-tier path — AVX-512 (`_mm512_xor_si512`, `_mm512_popcnt_epi64`), AVX2, scalar fallback — with `std::is_x86_feature_detected!()` selection cached in an `AtomicU8`. Reference `bind_avx512` and `hamming_avx512` unsafe functions and a `#[target_feature]`-gated module `mod simd`. Expected speedups: `bind` 5→1 ns, `similarity` 13→2 ns on AVX-512.
**Reality**: `rg 'target_feature|_mm512|_mm256|is_x86_feature_detected|avx512|avx2' crates/roko-primitives/` returns **zero matches**. There is no `simd` module, no `unsafe` SIMD code, no runtime feature detection, no dispatch table. The current `bind` at `crates/roko-primitives/src/hdc.rs:107-113` and `similarity` at `:211-218` are scalar u64 loops that LLVM may auto-vectorise. The crate attribute `#![deny(unsafe_code)]` declared in `roko-primitives/src/lib.rs:14` actively blocks any explicit SIMD intrinsic path (they all require `unsafe`).
**Fix sketch**: Keep the doc labelled as design. If added, the implementation would need to relax `#![deny(unsafe_code)]` on `roko-primitives` to `#![forbid(unsafe_code)]` only on non-SIMD modules, or move SIMD into a separate feature-gated module. Benchmarks first; only promote past scalar when profiling shows a hot path.

---

## B.20 — Formal knowledge ontology schema (`KnowledgeOntology`, `TypeSchema`, `ProvenanceChain`)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 06 §"Knowledge Ontology: Formal Schema" (06-hdc-knowledge-encoding.md:660-724) specifies a `KnowledgeOntology { type_schemas: HashMap<KnowledgeKind, TypeSchema> }` with `TypeSchema { required_slots, optional_slots, type_relationships }`, `OntologySlot { role, filler_type, description }`, `FillerType::{Text, Enum, Reference, Numeric}`, and `TypeRelation::{PromotesTo, Refutes, ComposedFrom}`. Doc 06 §"Knowledge Provenance Chain" (`:776-882`) further specifies `ProvenanceChain { entry_id, origin, transformations, sources, original_hash, current_hash }` with a five-variant `ProvenanceOrigin` enum (`Distilled`, `Promoted`, `Imported`, `UserCreated`, `DreamsSynthesized`). Also `encode_causal_link(cause, effect, domain, strength)` in Doc 06 §"CausalLink Encoding Schema" (`:725-772`) declares a four-argument free function.
**Reality**: `rg 'KnowledgeOntology|OntologySlot|TypeSchema|TypeRelation|FillerType|ProvenanceChain|ProvenanceOrigin' crates/` returns **zero matches**. None of these types exist in any workspace crate. The actual causal encoding lives inside `KnowledgeHdcEncoder::encode_causal_link` at `crates/roko-neuro/src/hdc.rs:61-110` (private, no free function, no explicit slot schema — see B.12). Provenance-like fields on `KnowledgeEntry` (`source`, `source_episodes`, `source_model`, `refuted_insight_id`, `refutation_evidence`) are flat strings, not a structured chain.
**Fix sketch**: Mark the Knowledge Ontology and Provenance Chain sections as design-only in doc 06. Any schema introduction should start by deriving `TypeSchema` from the existing `KnowledgeKind` enum in `roko-neuro`, then adding structured provenance as a new durable field on `KnowledgeEntry`.

---

## B.21 — Episode compression via bundling (`compress_episode`, `BundleDiversity`)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 06 §"Episode compression via bundling" (06-hdc-knowledge-encoding.md:584-645) specifies `pub fn compress_episode(entries: &[&KnowledgeEntry]) -> Option<HdcVector>` that bundles pre-encoded entry vectors (short-circuit on empty input), plus a `fn check_bundle_diversity(entries: &[HdcVector]) -> BundleDiversity` quality gate with thresholds 0.45 / 0.52 and a `BundleDiversity::{TooFew, TooSimilar, Borderline, Good}` enum.
**Reality**: `rg 'compress_episode|BundleDiversity|check_bundle_diversity' crates/` returns **zero matches**. No episode-bundling helper, no diversity gate, no summary-vector API anywhere. `KnowledgeHdcEncoder` encodes individual entries only. Episode-level aggregation would need to (a) load entries via `NeuroStore::read_all`, (b) deserialise each `hdc_vector` back into `HdcVector` via `HdcVector::from_bytes`, (c) bundle — none of this pipeline is wired.
**Fix sketch**: Mark this section as design-only. If implemented, `compress_episode` fits naturally as a free function in `roko-neuro/src/hdc.rs` alongside `KnowledgeHdcEncoder`.

---

## B.22 — k-medoids HDC clustering (roko-learn)

**Status**: DONE
**Severity**: —
**Doc claim**: Not in docs 04/05/06/09 directly, but doc 04 "Key sources" implicitly endorses HDC as a clustering substrate and doc 06 §"Current Status and Gaps" lists clustering as part of the knowledge layer. The algorithm (PAM with farthest-first seeding, distance = 1.0 − similarity) is a natural corollary of the `similarity` operation in B.06.
**Reality**: `crates/roko-learn/src/hdc_clustering.rs:81-152` implements `pub fn k_medoids(vectors: &[HdcVector], config: &KMedoidsConfig) -> ClusterResult` with full PAM iteration (assign + update + converged flag). `KMedoidsConfig { k: usize, max_iterations: usize }` at `:36-52` (Default `k=3, max_iterations=100`). `HdcCluster { medoid_index, medoid, members }` at `:56-63`. `ClusterResult { clusters, iterations, converged }` at `:65-75`. Distance precompute at `:159-170` uses `1.0 - vectors[i].similarity(&vectors[j])` via `roko_primitives::HdcVector` (`:34`). Greedy farthest-first seeding at `:177-220`. Assign at `:223-237`. Best-medoid update at `:241-260`. Consumed by `roko-learn/src/pattern_discovery.rs:45, 360` for pattern clustering. 10 tests covering empty input, k=0, single-vector, k > n, identical vectors, three-cluster recovery, threshold crossing, convergence, determinism, and complete-assignment round-trip.

---

## B.23 — BSC capacity / SNR formula and dimension bounds (doc-only constants)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §"Signal-to-Noise Ratio and Capacity Bounds" (04-hdc-vsa-foundations.md:145-166) gives `SNR = √(D/K)` with a table at D=10,240 (K=5 → SNR 45.3, K=100 → SNR 10.1, K=500 → SNR 4.5). Doc 04 §"Johnson-Lindenstrauss Bound" (`:119-141`) gives `D ≥ (8 ln N) / ε²`. Doc 09 §"Threshold Table" (09-false-positive-math.md:57-70) gives Z-score → threshold tables (0.505/0.510/0.512/0.515/0.520/0.526/0.530/0.540/0.550). These are mathematical facts, not code requirements.
**Reality**: The code uses D = 10,240 (via `[u64; 160]` at `crates/roko-primitives/src/hdc.rs:25`) consistent with the doc's dimension choice, and the `similarity` normalisation `/ 10_240.0_f32` at `:217, :233` matches the D used in the SNR formula. No Rust constant or function exposes `SNR(D, K)` or the JL bound; they live in markdown. This is appropriate — these are analysis tools, not runtime values. The only code-side consequence is the `160`/`10_240` dimension, which is already locked in.
**Notes**: If a `snr()` or `max_bundle_size()` helper is ever useful for code-side validation (e.g. refusing to bundle > 100 items without warning), it could live in `roko-primitives::hdc`. Today none exists; doc and code are statistically consistent but not formally linked.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 12 (B.01-B.07, B.16, B.17, B.18, B.22, B.23) |
| PARTIAL | 1 (B.12 `KnowledgeHdcEncoder` is real but much thinner than spec) |
| NOT DONE | 10 (B.08 BundleAccumulator, B.09 ItemMemory, B.10 ResonatorNetwork, B.11 fractional/decaying/online, B.13 query_by_role, B.14 three-tier search, B.15 0.526 threshold constant, B.19 SIMD, B.20 ontology schema, B.21 episode compression) |
| SCAFFOLD | 0 |

The core 10,240-bit `HdcVector` algebra — bind, bundle, permute, similarity,
from_seed, to_bytes/from_bytes, fingerprint helpers — is production-quality in
`roko-primitives` with 10 tests and rkyv zero-copy support. The knowledge
encoder (`roko-neuro/src/hdc.rs`) is real but is a simple unit struct with
causal-vs-generic dispatch plus a cause/effect-probe `encode_query`, not the
stateful `ItemMemory`-backed pipeline the doc implies. k-medoids clustering on
`HdcVector` (`roko-learn/src/hdc_clustering.rs`) is fully implemented and
consumed by `pattern_discovery`. Everything above the core operations —
`BundleAccumulator`, `ItemMemory`, `ResonatorNetwork`, three-tier search,
structured query API, fractional/decaying/online binding variants, SIMD
intrinsics, formal ontology schema, and `compress_episode` — is designed in
the docs but not implemented. The 0.526 Bonferroni threshold is a documented
recommendation that exists only in markdown; no `const`, no literal, no
similarity filter uses it in any Rust file. Doc 04 line 208's note that
`bardo-primitives` is to be renamed to `roko-primitives` has landed in code —
all consumer files import `roko_primitives::HdcVector` — but docs/06-neuro/04-,
05-, and 06- "Key sources" blocks still point at the old `bardo-primitives`
path.

## Agent Execution Notes

### B.15 — Query Threshold Contract First

The highest-value execution target in this section is not `ResonatorNetwork` or SIMD work.

It is making the live HDC similarity/query contract less implicit:

1. decide whether a threshold or `min_similarity` belongs in runtime,
2. keep that decision aligned with the actual `KnowledgeStore` query path,
3. avoid adding research-heavy HDC machinery just because the docs mention it.

### B.08-B.14 / B.19-B.21 — Triage, Don’t Sprawl

If an agent touches the advanced HDC items:

- prefer a very small enabler such as `BundleAccumulator`, `ItemMemory`, or a structured query helper only if a real caller needs it,
- explicitly defer `ResonatorNetwork`, SIMD kernels, three-tier search, ontology schema, and compression helpers if no production path depends on them.

Acceptance criteria for this section:

- later agents know which HDC helpers are runtime-relevant now,
- the threshold/query story is clearer,
- research-only HDC items are marked as deferred instead of implied.
