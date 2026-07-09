# B — Identity and On-Chain Trust (Docs 03, 04, 05, 06)

Parity of the four trust-layer chapters in topic 08: HDC precompile,
Korai Passport (ERC-721 soulbound), Ventriloquist defense, and the
ERC-8004 three-registry design. The only shipping surface touched by
this section is the local HDC primitive at `roko-primitives/src/hdc.rs`
— everything else is Phase 2+ Tier-6 design.

The interesting thread here is **HDC primitive ownership**: the canonical
10,240-bit vector lives in `roko-primitives`, is re-used by `mirage-rs/src/
chain/hdc_index.rs` and `mirage-rs/src/chain/hnsw.rs` (which wrap but do
not duplicate it), and is the concrete input to an EVM precompile that
does not yet exist at address `0xA01`.

Generated 2026-04-16.

---

## B.01 — 10,240-bit BSC vector primitive ships (Doc 03 §"HDC Vector Format", §"Binary Spatter Code")

**Status**: DONE
**Severity**: —
**Doc claim**: BSC encoding: 10,240 bits (1,280 bytes), binary elements, normalized Hamming similarity, three core operations — `BIND` (XOR), `BUNDLE` (majority vote), `PERMUTE` (cyclic shift).
**Reality**: `crates/roko-primitives/src/hdc.rs` ships the exact primitive. `HdcVector { bits: [u64; 160] }` at `hdc.rs:24-26` (160 × 64 bits = 10,240 bits). Operations: `bind(&self, other)` at `:107-113` is the XOR involution; `bundle(vectors: &[&Self])` at `:117-138` is the per-dimension majority-vote with tie-break to 0; `permute(&self, n)` at `:142-164` is the cyclic-shift by `n` positions. `similarity(&self, other)` at `:211-218` returns `1.0 - differing_bits / 10_240.0`. Serialize/deserialize via `to_bytes()` / `from_bytes()` at `:168-186` — produces exactly 1,280 bytes. Deterministic seeding via `from_seed(seed)` at `:193-208` (FNV-1a → splitmix64 fill). Eight tests at `:261-344` include `hdc_bind_involution`, `hdc_similarity_self`, `hdc_bundle_tie_rule`, `hdc_bytes_roundtrip`, `hdc_from_seed_deterministic`, `hdc_from_seed_distinct`, `hdc_serde_roundtrip_json`, `hdc_fingerprint_is_deterministic`, `hdc_text_fingerprint_is_deterministic`.

---

## B.02 — Two HDC index wrappers exist; both reuse the canonical primitive (Doc 03 §"Three-Tier Search Architecture")

**Status**: DONE (with drift in ownership documentation)
**Severity**: LOW
**Doc claim**: "The encoding is the same whether computed by `roko-primitives` locally or by the precompile on-chain." Doc 03 §"Three-Tier Search Architecture" describes a three-tier pipeline (Bloom → approximate → exact); Doc 18 §"Emulated Precompiles" lists `HDC similarity | 0xA01 | Local roko-primitives HDC operations`.
**Reality**: Two wrappers sit above the canonical primitive and neither duplicates it:
- `apps/mirage-rs/src/chain/hdc_index.rs:1-237` — `HdcIndex`, `IndexedVector { id, vector: HdcVector, weight }`, `Hit { id, similarity, weight, score }`. File comment at `:1-16` explicitly states: "We do not ship SIMD-specific code paths in this POC (we use `HdcVector::similarity`, which compiles to scalar XOR+popcnt on x86-64 and ARM without any intrinsics)." Brute-force top-K Hamming similarity with `similarity × weight` combined scoring.
- `apps/mirage-rs/src/chain/hnsw.rs:1-483` — `HnswBinaryIndex`, `HnswConfig`. Approximate nearest-neighbour on the same 10,240-bit vectors.

Both wrappers re-export `HdcVector` from `roko-primitives`; neither re-implements the bit-level primitive. There is no third implementation — the "three HDC implementations" concern in the parity plan is a **wrapper, not a duplication**. The canonical owner is unambiguously `crates/roko-primitives/src/hdc.rs`; the two mirage files are users. What the docs don't say is that `HdcIndex` covers the "exact" tier and `HnswBinaryIndex` covers the "approximate" tier, with no "Bloom" tier shipping.
**Fix sketch**: Update Doc 03 §"Three-Tier Search Architecture" to map the three tiers explicitly: Bloom tier = `Design — not yet implemented`, approximate tier = `HnswBinaryIndex` (`apps/mirage-rs/src/chain/hnsw.rs`), exact tier = `HdcIndex` (`apps/mirage-rs/src/chain/hdc_index.rs`). Note that all three tiers share the same underlying `roko_primitives::HdcVector`.

---

## B.03 — HDC precompile at `0xA01` does NOT exist (Doc 03 §"Precompile Interface", Doc 18 §"Emulated Precompiles")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 03 §"Precompile Interface" specifies four operations at reserved address `0xA01` — `hdc_similarity(a, b) -> uint256`, `hdc_topk(query, k) -> (uint256[], bytes32[])`, `hdc_bind(a, b) -> bytes1280`, `hdc_bundle(vectors) -> bytes1280`. Doc 18 §"Emulated Precompiles" claims mirage-rs emulates the precompile at `0xA01`.
**Reality**: `Grep '0xA01\b|0xa01\b|hdc_precompile|HdcPrecompile' crates/ apps/` returns zero matches on the `.rs` side (one doc-only hit in roko-demo manifest). `apps/mirage-rs/src/chain/mod.rs` does not register a precompile — the chain feature exposes `HdcIndex` and `HnswBinaryIndex` as **library code**, not as an EVM-callable precompile. No revm precompile registration, no bytecode export, no `hdc_*` opcode handler. The agent-callable path is Rust → `HdcVector::similarity`, not Solidity → precompile.
**Fix sketch**: Mark Doc 03 as `Design — Phase 2+`. Update Doc 18 §"Emulated Precompiles" table: `HDC similarity | 0xA01 | Not yet emulated (Phase 2+)` and add a "local library path" column pointing at `roko_primitives::HdcVector::similarity`.

---

## B.04 — Gas cost model (400 gas for top-K=20) is pure calibration design (Doc 03 §"Gas Cost Model")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: ~50 gas for `hdc_similarity`, ~400 gas for `hdc_topk` K=20, ~80 gas for `hdc_bind`, ~150 gas for `hdc_bundle`; model calibrated to ECRECOVER anchor (1 gas ≈ 10 ns); includes a full breakdown with input-read / popcount / heap-selection components. The 2026-04-13 enhancement pass added Stylus WASM / ZK-proven / optimistic fraud proof / TEE / Binius binary-field STARK variants.
**Reality**: No gas model exists because no precompile exists (B.03). The ~2 ms per query at 10K entries benchmark mentioned in `hdc_index.rs:1-12` is the ONLY real calibration data in the code — and it is for `HdcIndex` (library), not a gas-metered precompile. The Stylus / ZK / fraud-proof / TEE / Binius sections in Doc 03 are all design essays.
**Fix sketch**: Apply a single `Design — Phase 2+` banner to Doc 03 from §"Gas Cost Model" onward. Keep the FPR / threshold / 100K vocabulary statistical section (B.01) unbannered — it is real.

---

## B.05 — Korai Passport soulbound ERC-721 does NOT exist (Doc 04 §"Passport Struct", Doc 24 §"1. Agent Registry")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Full `AgentPassport` Rust struct at Doc 04 `:28-65` with `passport_id`, `owner`, `capability_list` (u64 bitmask), `domain_stakes: BTreeMap<String, U256>`, `reputation_tracks: BTreeMap<String, ReputationScore>`, `tee_attestation: Option<(Hash, u64)>`, `system_prompt_hash: [u8; 32]`, `tier: PassportTier`, `slash_history: Vec<SlashRecord>`. ERC-721 soulbound at on-chain address `0xA100`.
**Reality**: `Grep 'AgentPassport|PassportTier|SlashRecord|ReputationScore' crates/ apps/` returns zero matches in `.rs` files. The only Passport-like code is `crates/roko-agent-server/src/registration.rs` which the CLAUDE.md row calls out as "Imports ChainClient; passport id integration" — but it operates at the agent-server sidecar level (registering to a mock `ChainClient`), not as an on-chain ERC-721 soulbound NFT. The shipping Solidity `contracts/src/AgentRegistry.sol:9-73` is a plain `mapping(address => Agent)` without `tokenId` / `ownerOf` / non-transferability enforcement (see A.11). The full `AgentPassport` struct is design-only.
**Fix sketch**: Mark Doc 04 `Design — Phase 2+`. When the real ERC-721 implementation begins, cross-link to the existing `contracts/src/AgentRegistry.sol` and decide whether to extend it or write fresh.

---

## B.06 — Four Passport tiers and capability bitmask are unused (Doc 04 §"Tier System", §"Capability Bitmask")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Four tiers (Protocol / Sovereign / Worker / Edge). Capability bits: `inference`, `data-transform`, `fine-tune`, `RAG`, `multi-agent`, `trading`, `security`, `analytics`, `knowledge`, `strategy` — 10 bits in a `u64`.
**Reality**: `Grep 'PassportTier|Sovereign|Edge|capability_list|CAPABILITY_INFERENCE' crates/ apps/` returns zero matches. No tier enum, no capability constants, no bitmask helpers. The shipping `contracts/src/AgentRegistry.sol` uses `string capabilities` — a free-form string field, not a bitmask.
**Fix sketch**: Leave Doc 04 as Phase 2+. Record that the demo contract's `string capabilities` is incompatible with the bitmask design — either the bitmask needs to be adopted or the `capabilities` field needs to be renamed (`capabilities_mask`) when the Korai v1 contract set is introduced.

---

## B.07 — Registration flow / KORAI mint on register is absent (Doc 04 §"Registration Process")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: `korai_registerPassport(owner, capabilities, promptHash, teeAttestation) -> passport_id`. Registration mints a small KORAI bootstrap to the Passport address (tier-dependent — see A.07).
**Reality**: The shipping `contracts/src/AgentRegistry.sol:31-42` `register(capabilities, passportHash)` is a no-token registration: no KORAI transfer, no `teeAttestation` argument, no tier assignment. Returns nothing (emits `AgentRegistered(agent, passportHash, capabilities)` event). The full registration ritual (capability check + token mint + tier classification + initial domain stake) is Phase 2+.

---

## B.08 — Ventriloquist defense on-chain surface is absent (Doc 05 §"System Prompt Hash Commitment", §"24h Timelock")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: `system_prompt_hash: [u8; 32]` committed on-chain at registration. `updatePromptHash(passportId, newHash)` requires a 24-hour timelock. Pre-job TEE verification + rate limiting.
**Reality**: `Grep 'system_prompt_hash|updatePromptHash|Ventriloquist|prompt_timelock' crates/ apps/ contracts/` returns only one contract-side hit: `contracts/src/AgentRegistry.sol:12` stores `bytes32 passportHash` as an immutable field — it cannot be updated at all. No timelock, no rate limit. On the Rust side, `crates/roko-core/src/attestation.rs` and `crates/roko-chain/src/witness.rs` commit attestation hashes on-chain but for different purposes (signal witnessing; see F.08), not prompt-hash commitment.
**Fix sketch**: Doc 05 is entirely `Design — Phase 2+`. The nearest shipping ancestor is the attestation / witness flow at `crates/roko-chain/src/witness.rs`, which could be extended with a `prompt_hash` variant once Passport registration lands.

---

## B.09 — TEE attestation integration is absent (Doc 05 §"Pre-Job TEE Verification")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Korai Passport carries `tee_attestation: Option<(Hash, u64)>` refreshed via attestation endpoint; pre-job verification by clients.
**Reality**: `Grep 'TEE|SGX|TDX|enclave|DCAP|sgx_quote' crates/` returns 9 hits across 3 files — `crates/roko-compose/src/context_provider.rs` (2), `crates/roko-compose/src/prompt.rs` (6), `crates/roko-cli/src/tui/config_meta.rs` (1). All uses are **string tokens in prompt templates and UI labels**, not a TEE attestation integration. No Intel SGX / TDX quote verification, no DCAP attestation parsing, no MRENCLAVE comparison.
**Fix sketch**: Mark Doc 05 §"Pre-Job TEE Verification" + §"Emergency Revocation" as `Design — Phase 2+`. Point to the compose-layer prompt tokens as hints that the agent prompts already mention TEE but the runtime has no TEE surface.

---

## B.10 — ERC-8004 three-registry separation is absent as Solidity (Doc 06 §"Three Registries")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: ERC-8004 mandates three on-chain registries: **Identity** (passport), **Reputation** (feedback auth + EMA), **Validation** (work proofs). Separation of concerns. Cross-registry flows (e.g. Marketplace queries Reputation, submits WorkProof to Validation).
**Reality**: Only the "Identity" shape exists as Solidity — and only as the demo `contracts/src/AgentRegistry.sol` (not the soulbound ERC-721 spec; see B.05). No Reputation Registry (see A.12). No Validation Registry (`Grep 'VerifiedWork|submitWorkProof|Validation.*Registry' contracts/` returns zero matches). `contracts/src/ConsortiumValidator.sol` is a consortium voting contract, not ERC-8004 Validation.
**Fix sketch**: Mark Doc 06 `Design — Phase 2+`. Add a cross-link to `contracts/src/AgentRegistry.sol` as a non-ERC-8004 precursor that may need to be rewritten or extended.

---

## B.11 — Reputation EMA feedback loop is only a specification (Doc 06 §"Reputation Registry Operations")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Reputation is stored per (passportId, domain) as `(score, jobCount, lastUpdate)`. `submitFeedback` writes, `applyDecayTick` applies half-life decay, `slash` records violations.
**Reality**: `Grep 'apply_decay_tick|submit_feedback|reputation_store' crates/ apps/` returns zero matches on the chain side. Cross-link: `roko-learn` implements adaptive thresholds and efficiency events for agent-level scoring (CLAUDE.md status table) but none of this writes into a reputation registry. The EMA formula and decay schedule are design.

---

## B.12 — Cross-registry flows (Marketplace ↔ Reputation ↔ Validation) are absent (Doc 06 §"Cross-Registry Flows")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Multi-registry write flows: Marketplace accepts bid → queries Reputation → binds to Escrow → writes WorkProof to Validation → Validation triggers Reputation update.
**Reality**: No Marketplace + Escrow + Validation triad exists (see A.13). The `contracts/src/BountyMarket.sol` + `contracts/src/ConsortiumValidator.sol` pair does a simpler bounty-post + committee-validate flow without the ERC-8004 cross-registry signatures. Design-only.

---

## B.13 — 0xA100 / 0xA200 / 0xA300 addresses are not wired anywhere (Doc 06 §"Predeployed Addresses")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Identity Registry at `0xA100`, Reputation Registry at `0xA200`, Validation Registry at `0xA300` — all predeployed at Korai genesis.
**Reality**: Same as A.16 — `Grep '0xA100|0xA200|0xA300|0xa100|0xa200|0xa300' crates/ apps/ contracts/` returns zero matches. No shared address table, no genesis config, no mirage-rs registration of these addresses.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 2 (B.01 10240-bit HDC primitive, B.02 HdcIndex + HnswBinaryIndex wrappers) |
| PARTIAL | 0 |
| NOT DONE | 11 (B.03 HDC precompile, B.04 gas model, B.05 Passport struct, B.06 tiers + bitmask, B.07 registration flow, B.08 Ventriloquist, B.09 TEE attestation, B.10 three registries as Solidity, B.11 reputation EMA feedback, B.12 cross-registry flows, B.13 canonical addresses) |

Section B is the clearest "Phase 2+" part of the chain layer. Only
**two** items ship: the canonical 10,240-bit `HdcVector` primitive
(B.01) and the two wrappers around it (B.02). Everything related to
on-chain trust — the EVM precompile at `0xA01`, the ERC-721 soulbound
Passport, the Ventriloquist system-prompt-hash commitment, TEE
attestation, and all three ERC-8004 registries — is design work.

The concern raised in the parity plan that "HDC primitive lives in
`roko-primitives` AND in `mirage-rs/src/chain/hdc_index.rs` AND in
`mirage-rs/src/chain/hnsw.rs`" turns out to be **wrapper layering, not
duplicate implementation** (B.02). The canonical primitive lives in
`roko-primitives` and the two mirage files depend on it via
`use roko_primitives::HdcVector`. There is no third implementation to
reconcile.

## Agent Execution Notes

### B.01 / B.02 — Reaffirm canonical owner (docs only)

Best use of this section in batch `08`:

1. Doc 03 §"Three-Tier Search Architecture" should map the three tiers to concrete files (Bloom = not implemented, HNSW = `hnsw.rs`, exact = `hdc_index.rs`) and cite `roko_primitives::HdcVector` as the single canonical owner.
2. No code changes needed.

### B.03-B.13 — Frontier Banner Pass

Apply a `Design — Phase 2+` banner to each of Doc 03 (from §"Precompile
Interface" onward), Doc 04 (entire), Doc 05 (entire), Doc 06 (entire).
Do not expand into any code work.

Acceptance criteria for this section:

- HDC canonical-owner question has exactly one answer (`roko-primitives`) and that answer is in Doc 03,
- later agents can tell from Docs 04 / 05 / 06 that nothing on-chain in those chapters has been written,
- the `0xA01` / `0xA100` / `0xA200` / `0xA300` address story is defensible if anyone asks "where are these constants?".
