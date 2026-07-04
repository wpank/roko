# Implementation Direction

Based on discussion 2026-04-21.

## Decisions

1. **Chain target**: Korai (with Mirage as local stand-in) is the primary coordination
   substrate. P2P (Iroh) secondary. General EVM is out of scope for now.

2. **Priority**: Wire existing + build new in parallel. Don't gate new capabilities
   on finishing the wiring — do both.

3. **Privacy**: Essential. PP-HDC hash-encoding is a core requirement, not an add-on.
   Multiple instances must share learning without exposing task details.

## Parallel Workstreams

### Stream A: Wire Existing HDC (foundation)

Connect the built-but-disconnected HDC code:

1. **Episode clustering on schedule** — Call `k_medoids` from orchestrate.rs every N
   episodes (N configurable, default 50). Persist clusters to
   `.roko/learn/episode-clusters.json`. The code exists in
   `roko-learn/src/hdc_clustering.rs`.

2. **Cross-domain resonance at dispatch** — Before composing the system prompt,
   query the pattern store for resonant patterns matching the current task fingerprint.
   `detect_cross_domain_resonance` exists in `roko-primitives/src/codebook.rs`.
   Inject matching patterns as context in the prompt.

3. **Neuro → prompt injection** — At compose time, `KnowledgeHdcEncoder::encode_query()`
   in `roko-neuro/src/hdc.rs` can find relevant knowledge entries. Feed results into
   `SystemPromptBuilder` as an enrichment section.

4. **Somatic marker population** — After each episode, feed (task_fingerprint, outcome)
   into daimon's somatic k-d tree. At dispatch time, query for sub-ms "gut feeling"
   before analytical model selection.

5. **CascadeRouter cluster features** — Add episode cluster ID to the 18-dim context
   vector. "This task's fingerprint is in cluster 7, where model X has 85% pass rate."

### Stream B: New Capabilities (expansion)

Build the novel integrations:

1. **HD-CB for CascadeRouter** — Implement HDC-based contextual bandit alongside
   existing LinUCB. HD-CB (IEEE Jan 2025) replaces ridge regression with parallel
   vector operations. Benefits: faster convergence, noise-resilient, naturally
   composable across instances.

2. **PP-HDC hash-encoding** — Implement distance-preserving non-invertible projection
   for knowledge vectors. This is the privacy layer — vectors can be shared without
   exposing original episodes. Based on PP-HDC (IEEE 2024), <1% accuracy loss.

3. **Korai on-chain commitments** — Merkle tree of knowledge cluster medoids, root
   hash committed on Korai chain. HDC bind/similarity as Mirage precompile (~600 gas).
   Other instances verify inclusion and query by similarity.

4. **HuggingFace integration** (`roko-hf` crate):
   - Inference Providers as LLM backend (dedicated `HuggingFaceApi` provider)
   - Dataset Viewer for benchmark loading (SWE-bench, MBPP, etc.)
   - Hub API for model discovery (CascadeRouter dynamic arm addition)
   - AutoTrain for fine-tuning loop (successful episodes → training data → model)
   - Hub publishing for learned artifacts (playbooks, models, episodes)

5. **Arena framework** (`roko bench`) — Native Rust benchmark harness that converts
   any HF dataset into roko tasks, runs through the full orchestrator, and feeds
   outcomes into all learning loops.

6. **Distributed knowledge exchange** — P2P layer (Iroh) for raw vector storage,
   Korai chain for Merkle commitments. PP-HDC-encoded vectors only (privacy-first).

### Stream C: Fine-Tuning Loop (the exponential)

The goal: roko generates training data as a byproduct of working, fine-tunes
specialized models, and deploys them back into production.

1. Successful episodes (from any arena) → extract (prompt, completion) pairs
2. Upload as HF dataset → AutoTrain fine-tune on base model
3. Push fine-tuned model to Hub
4. CascadeRouter (HD-CB variant) adds as new arm
5. Bandit explores → if it wins → more traffic → more training data → repeat

This loop requires Streams A + B to be partially wired first (episodes need
fingerprints and clustering for quality selection of training data).

## Architecture: Where Everything Connects

```
Arena (any domain)
  │
  ├── sample() → tasks
  │
  ├── [HDC] fingerprint task → somatic check (sub-ms)
  │     └── if similar to past failure → escalate model tier
  │
  ├── [HDC] query neuro store by fingerprint → inject knowledge into prompt
  │
  ├── [HDC] query pattern store for resonant patterns → inject as context
  │
  ├── [HD-CB] CascadeRouter picks model using HDC context features
  │
  ├── dispatch agent → run gates → record episode
  │
  ├── [HDC] fingerprint episode → update clusters (every N episodes)
  │     ├── cross-domain resonance detection
  │     ├── Lotka-Volterra pattern evolution
  │     └── somatic marker update
  │
  ├── [PP-HDC] hash-encode cluster medoids → share via Korai/P2P
  │
  ├── [HF] publish successful episodes as training dataset
  │     └── AutoTrain fine-tune → push model → CascadeRouter adds arm
  │
  └── score() → log to .roko/bench/scores.jsonl
        └── next batch (all learning carries over)
```

## Key Properties

- **Per-turn latency**: Somatic marker check adds <1ms. HDC fingerprinting adds <1ms.
  No overhead visible to the user.

- **Cross-domain transfer**: Resonance detection finds structural analogies between
  arenas. Playbooks transfer via HDC similarity, not semantic matching.

- **Privacy-first sharing**: PP-HDC hash-encoding means vectors can be shared without
  exposing what tasks were being worked on. Essential for multi-org deployment.

- **Compositional reasoning**: HDC's algebraic structure (bind/bundle/permute) enables
  analogical transfer that neural embeddings can't do. "borrow check error in Rust"
  can be algebraically transformed to "reference error in Python."

- **Communication-efficient**: 1,280 bytes per medoid vector. Federated HDC achieves
  66x communication reduction vs neural federated learning.

- **On-chain feasibility**: XOR bind ≈ 600 gas for 10,240-bit vectors. Similarity
  check (XOR + popcount) ≈ 800 gas. Merkle root storage ≈ 20K gas. All within
  practical gas budgets on Korai/Mirage.
