# Structural Embeddings for Trace Tool-Call Graphs

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust AI trace registry (~235K LOC, 6 crates) that scores AI coding agent session traces for quality and novelty inside TEEs (Trusted Execution Environments) on NEAR AI Cloud, compensating contributors with NEAR blockchain credits. ~352 submissions to date. TC currently embeds traces as rendered text via BGE-large-en-v1.5 for cosine similarity against an HNSW index (VectorIndex, usearch). This discards structural information -- two traces calling the same tools in the same order with different arguments are structurally identical but may score as "novel." This document synthesizes GNN and HDC-native approaches to structural embedding for trace tool-call graphs, provides a priority-ordered integration path, and identifies one withdrawn paper whose empirical claims must not be cited.

---

## 1. The Gap: Text Embedding Discards Graph Structure

TC's current embedding pipeline treats traces as flat text. The `Embedder` trait receives rendered trace content, passes it through BGE-large-en-v1.5, and stores the resulting vector in the HNSW index. Novelty is then cosine distance from the nearest neighbor.

This loses several classes of structural information:

- **Tool-call sequences.** The order in which tools are invoked encodes agent strategy. A trace that calls `read_file` then `edit_file` then `run_tests` has a fundamentally different structure than one that calls `run_tests` first, reads failures, then edits. Text embedding may score these as similar if they contain the same tokens.
- **Dependency graphs.** The output of one tool call feeds into the input of another. These data-flow edges create a directed acyclic graph (or cyclic, in retry scenarios) that text embedding cannot represent.
- **Branching decisions.** Conditional tool calls -- where the agent chooses between alternatives based on prior output -- create branching nodes in the trace graph. Text embedding collapses branches into a linear narrative.
- **Loop structures.** Retry loops, error-recovery cycles, and iterative refinement create back-edges in the graph. These are structurally meaningful (a trace with 5 retry loops has a different structure than one that succeeds on the first attempt) but invisible to text embedding.
- **Parallel invocations.** Agents that invoke multiple tools concurrently create parallel subgraphs. Text rendering serializes these into an arbitrary order.

The consequence: structurally distinct traces may score as "near-duplicate" (same tools, different graph), and structurally identical traces may score as "novel" (same graph, different argument text). The embedding-based novelty signal is necessary but not sufficient.

---

## 2. Five Verified Papers (One Withdrawn)

### 2.1 GraphTracer (arXiv:2510.10581) -- WITHDRAWN

GraphTracer introduced Information Dependency Graphs (IDGs) for representing agent traces as directed graphs where nodes are LLM operations and edges are information dependencies. Version 2 of the paper was **withdrawn in December 2025** due to a "fundamental error in methodology."

**Status**: The IDG concept -- representing agent traces as dependency graphs rather than linear sequences -- survives as a useful abstraction. The empirical claims (18.18% improvement on attribution, specific benchmark numbers) are **invalidated** and must not be cited. The withdrawal underscores the importance of verifying methodology in trace-graph research, a field where evaluation protocols are not yet standardized.

**TC relevance**: The IDG abstraction aligns with TC's need to represent tool-call dependencies as graphs. The concept is valid; the specific implementation and evaluation are not.

### 2.2 GRADE (arXiv:2606.22741)

Graph Representation of LLM Agent Dependency and Execution. GRADE distinguishes three complementary graph projections from agent execution logs:

- **Execution-layer projections**: ReAct chains, tool-call trees, and step-by-step execution traces as directed graphs.
- **Dependency/PROV layers**: Data dependencies between steps -- which output feeds which input -- following W3C PROV-DM conventions.
- **Epistemic grading of dependency edges**: Not all dependencies are equal. Some are causal (output A directly determines input B), some are informational (A provides context for B), and some are incidental (A and B share a variable but the dependency is not functional). GRADE assigns epistemic grades to edges.

GRADE provides a better substrate than flat conformance checking for high-variability traces. Agent traces vary significantly across runs even for the same task; a flat conformance model flags legitimate variation as anomalous. Graph projections with epistemic grading distinguish structural deviation (different tools invoked) from surface deviation (same tools, different arguments).

**TC relevance**: GRADE provides the graph representation TC needs. Its execution-layer projection maps directly onto TC's tool-call event sequences, and the PROV layer captures data-flow dependencies that text embedding discards. The epistemic grading enables weighted structural comparison -- two traces that differ only in incidental dependencies are structurally more similar than two that differ in causal dependencies.

### 2.3 AgentGraph (AAAI 2026)

Published in AAAI 2026 proceedings (ojs.aaai.org/index.php/AAAI/article/view/42393). AgentGraph is a trace-to-graph platform that performs:

- **Causal attribution on logged traces**: Given a trace and an outcome, identify which tool calls were causally responsible for the outcome. This goes beyond correlation (which calls co-occurred with success) to intervention-based attribution (which calls, if removed, would change the outcome).
- **Perturbation robustness testing**: Systematically perturb the trace graph (remove edges, swap tool order, substitute tool calls) and evaluate whether the outcome changes. This quantifies how robust the agent's strategy is to structural changes.
- **No re-execution required**: Both attribution and robustness testing operate on the logged trace without re-running the agent. This is critical for TC, where traces are submitted post-hoc and re-execution is neither feasible nor desirable.

**TC relevance**: AgentGraph's perturbation robustness is directly applicable to structural novelty scoring. A trace whose outcome is robust to many perturbations has a more redundant (less novel) structure than one that is fragile to perturbation. This provides a structural novelty signal orthogonal to text embedding distance.

### 2.4 MCPShield (arXiv:2605.11053)

Content-aware tool-call attack detection via GNN on logged MCP (Model Context Protocol) traffic. MCPShield trains a graph neural network on tool-call interaction graphs to detect malicious patterns.

Key finding: **content features dominate structure for attack detection**. The GNN's structural features alone provide modest detection rates. However, for **novelty detection** (distinguishing previously unseen tool-call patterns from known ones), structural features add **2-10 percentage points AUC** on top of content-only baselines.

This result is important for TC's design: structure is a complement, not a replacement, for text embedding. The structural embedding layer adds marginal discriminative power for novelty detection -- exactly TC's use case -- but does not obsolete the existing text embedding pipeline.

**TC relevance**: MCPShield's 2-10pp improvement on novelty detection quantifies the expected gain from adding structural features to TC's scoring pipeline. This is the empirical basis for prioritizing structural embedding as a complement (Layer 4 in the multi-layer pipeline) rather than a replacement.

### 2.5 Agent-OSI (arXiv:2602.13795)

Layer-5 provenance interface for agent systems. Agent-OSI defines a standardized layered model (inspired by the networking OSI model) for agent provenance data. The L5 interface provides the standardized layer that structural embeddings consume -- a common representation for provenance events, tool calls, and data flows across heterogeneous agent frameworks.

**TC relevance**: Agent-OSI's L5 interface standardizes the provenance data format that feeds into structural embedding. If TC adopts Agent-OSI's L5 provenance layer for trace ingest (complementary to the OTel ingest path described in doc 03), structural embeddings operate on a standardized substrate rather than TC's bespoke trace envelope format. This reduces integration friction for traces from non-IronClaw agent frameworks.

---

## 3. VS-Graph: HDC-Native Graph Embedding (arXiv:2512.03394)

**TC's natural first step.** VS-Graph is a hyperdimensional computing (HDC) approach to graph embedding that represents graph structure as high-dimensional binary vectors.

Properties that make VS-Graph the right starting point for TC:

- **Pure Rust bit operations.** No GPU required. No Python dependency. No ONNX runtime. The core operations are XOR, rotation, and majority vote on bit vectors -- all implementable in a few hundred lines of Rust with no external dependencies.
- **450x faster than GNNs.** VS-Graph reports 450x speedup over GNN-based graph classification on standard benchmarks. For TC's TEE-hosted scoring pipeline, where inference latency directly impacts throughput, this is a material advantage.
- **Combinable with TC's existing HDC fingerprints.** TC already computes HDC fingerprints per episode (stored in the `hdc_fingerprint` field of episode records). VS-Graph's graph embeddings are HDC vectors of the same type -- they compose via XOR with existing fingerprints. No new embedding framework is needed.
- **Hamming distance comparison.** Structural similarity is Hamming distance between bit vectors. This is exact (no approximation), fast (single POPCNT instruction per 64-bit word), and compatible with existing nearest-neighbor infrastructure.

### 3.1 How It Works

1. **Extract tool-call graph** from the trace. Nodes are individual tool invocations; edges represent data flow (output of tool A used as input to tool B) or sequential dependency (tool B was invoked after tool A).

2. **Encode each node** as an HDC vector. The node vector is computed by binding (XOR) together component vectors for: tool name, argument types, outcome status (success/failure/error), and position in the invocation sequence.

3. **Encode graph structure** via VS-Graph's bind/bundle operations. Edge encoding binds the source node vector with a rotated version of the target node vector. The full graph vector is the bundle (majority vote) of all edge vectors. This preserves both local connectivity (which nodes are adjacent) and global topology (overall graph shape).

4. **Result**: A single HDC vector representing the trace's structural fingerprint. This vector captures tool-call patterns, dependency structure, and execution topology in a fixed-size representation.

5. **Compare** via Hamming distance. Two traces with similar tool-call graphs will have low Hamming distance between their structural fingerprints, regardless of differences in argument text or output content.

### 3.1.1 Accuracy Caveat: MUTAG/DD vs. TC's Trajectory Graphs

VS-Graph's 450x speedup is verified (arXiv:2512.03394). However, the benchmarks it was evaluated on -- MUTAG and DD -- are molecular graph datasets. Molecular graphs have different structural properties than TC's trajectory graphs: they are small (dozens of nodes), undirected, without back-edges, and without labeled sequential semantics on nodes.

TC's trajectory graphs are structurally different: tool-call sequences with branching (conditional tool selection), loops (retry/error-recovery back-edges), parallel subgraphs (concurrent tool invocations), and labeled node semantics (tool name, outcome status, argument types). Whether VS-Graph's HDC encoding captures these graph characteristics with the same discriminative power it shows on molecular graphs is an open question.

**Note**: VS-Graph's accuracy on TC's trajectory graphs (vs MUTAG/DD molecular graphs) needs validation before committing. The speedup advantage is structural (HDC vs GNN compute) and is architecture-independent, but classification accuracy on TC's specific graph topology is an open question that must be answered in Priority 3 evaluation.

### 3.2 Composition with Existing HDC Fingerprints

TC's existing HDC fingerprints encode episode content (text-level features). VS-Graph's structural fingerprint encodes graph topology. These are orthogonal signals that compose naturally:

```
content_fingerprint  = HDC vector from episode content (existing)
structure_fingerprint = HDC vector from tool-call graph (VS-Graph)
combined_fingerprint  = content_fingerprint XOR structure_fingerprint
```

The combined fingerprint encodes both content and structure. Hamming distance on the combined fingerprint incorporates both signals. Alternatively, the two distances can be computed separately and combined via weighted sum -- this allows tuning the relative importance of content vs. structure for novelty scoring.

### 3.3 Brute-Force HDC Scan: Two-Birds Solution

At TC's current scale (~352 traces), the correct implementation choice for structural fingerprint lookup is **brute-force exact Hamming distance scan over HDC hypervectors**, not HNSW. This resolves two independent problems simultaneously.

### 3.3.1 Determinism for Attestation (docs 12, 13)

TC's attestation model requires that scoring be deterministic: if two enclave runs receive the same input, they must produce bit-identical output or the attestation quote is meaningless. HNSW is nondeterministic by construction:

- **Randomized layer assignment**: HNSW assigns nodes to layers using a random geometric distribution. The layer a node lands in affects which edges are created during insertion, which affects the graph structure, which affects query results.
- **Parallel insert order**: When multiple vectors are inserted concurrently (which HNSW supports for throughput), the resulting graph structure depends on the interleaving order of concurrent inserts.

Seeding the HNSW RNG and pinning thread count reduces nondeterminism, but does not eliminate it in the general case (reduction order in parallel operations remains implementation-dependent). Brute-force scan has no random state and no data-dependent branching in its core loop: iterate over all stored vectors, compute Hamming distance via POPCNT, return the minimum. This is fully deterministic.

### 3.3.2 Side-Channel Resistance (docs 12, 13)

HNSW's graph traversal produces data-dependent memory access patterns: which nodes are visited, in what order, and which edges are followed all depend on the query vector and the current graph state. Inside a TEE enclave, these access patterns are observable via cache timing, memory bus monitoring, and other microarchitectural side channels. An adversary who can observe the access pattern of an HNSW query learns information about which region of the embedding space the query falls in -- leaking structural information about the trace being scored.

Brute-force scan has no data-dependent access patterns: every stored vector is accessed in sequence, regardless of the query. This is side-channel-free by construction.

From research4, section B9: "at 352 traces, deterministic brute-force scan over HDC vectors sidesteps HNSW side-channels AND provides determinism for attestation -- a rare two-birds win."

### 3.3.3 Feasibility at TC's Scale

D=128-bit HDC vectors reduce the per-comparison cost to exactly 2 POPCNT instructions (two 64-bit words). At 352 traces:

- 352 comparisons × 2 POPCNT = 704 instructions per query
- On any modern CPU, this executes in well under 1 microsecond

Even at 10,000 traces, brute-force Hamming scan over D=128 vectors takes roughly 20,000 POPCNT instructions -- still sub-millisecond and within the Layer 4 latency budget (< 5ms).

The crossover point where HNSW's sub-linear traversal becomes necessary is approximately 100,000+ traces, at which point the O(n) brute-force cost begins to dominate the O(log n) HNSW traversal. TC should not use HNSW for structural fingerprint lookup until the corpus reaches that scale.

**Implementation recommendation**: Use brute-force HDC Hamming scan for structural fingerprint lookup at TC's current and near-term scale. HNSW is deferred until corpus growth requires it. The brute-force path is simpler to implement, simpler to audit, deterministic, and side-channel-free.

---

## 4. Tool-Call Graph Extraction

Building the graph from TC's existing trace envelope requires extending the current chunker, not replacing it.

### 4.1 From Trace Envelope to Directed Graph

1. **Parse events** via `parse_envelope_rendered_events` (already exists in the chunker). This produces a sequence of typed events from the trace envelope's rendered content.

2. **Extract `(event_type, tool_name)` pairs** (already parsed). Each tool-call event becomes a candidate graph node. Non-tool events (text output, thinking, metadata) are annotation on adjacent nodes, not nodes themselves.

3. **Build directed graph.** Each tool-call event becomes a node. Edges are added based on:
   - **Sequential dependency**: Tool B was invoked after tool A completed. This is the default edge type, inferred from event ordering.
   - **Data flow**: Output of tool A appears in the input of tool B (detected via substring matching or argument reference tracking). These edges are stronger signals of structural dependency.

4. **Annotate edges** with data-flow metadata. An edge labeled "sequential" means ordering only; an edge labeled "data-flow" means the output of the source was consumed by the target. This distinction maps to GRADE's epistemic grading (section 2.2).

5. **Handle branching.** Conditional tool calls -- where the agent evaluates a condition and chooses between alternative tools -- create branching nodes. Detected by: multiple tool calls following a single observation/thinking event, or explicit conditional language in agent reasoning.

6. **Handle loops.** Retry and error-recovery patterns create back-edges (edges pointing to earlier nodes in the sequence). Detected by: repeated invocation of the same tool with similar arguments after a failure event. Loop detection is important because retry count is a structural feature -- a trace with 5 retries has a different graph structure (with back-edges) than a trace that succeeds on the first attempt.

### 4.2 Example Graph

For a trace with the sequence: `read_file -> edit_file -> run_tests -> [fail] -> edit_file -> run_tests -> [pass]`:

```
read_file ---[data-flow]--> edit_file(1) ---[seq]--> run_tests(1)
                                  ^                      |
                                  |                      v
                                  +---[retry]--- edit_file(2) ---[seq]--> run_tests(2)
```

This graph has 5 nodes, 4 forward edges, and 1 back-edge (the retry loop). Text embedding sees this as a flat sequence; the graph representation captures the retry structure explicitly.

---

## 5. GNN Path (For Later)

For trained GNN inference without a Python dependency, use ONNX Runtime via the `ort` crate (Rust bindings to ONNX Runtime). This avoids introducing Python into TC's TEE-hosted scoring pipeline.

### 5.1 Architecture

1. **Pre-train a small GNN** (e.g., 3-layer Graph Convolutional Network) on a corpus of tool-call graphs labeled for novelty. Training happens offline, outside the TEE, using PyTorch Geometric or DGL.

2. **Export to ONNX format.** The trained GNN is exported as a static ONNX model. The graph's adjacency matrix and node feature matrix are the model inputs; the output is a fixed-size embedding vector.

3. **Load via `ort` crate** in TC's scoring pipeline. The `ort` crate provides Rust bindings to ONNX Runtime with CPU-only inference. No GPU required. Model loading and inference are deterministic given the same input, which is important for TEE reproducibility.

4. **GNN embedding as an additional novelty signal** alongside text embedding (Layer 3) and HDC structural fingerprint (Layer 4). The GNN embedding captures learned structural features that hand-crafted HDC encoding may miss.

### 5.2 Expected Improvement

Per MCPShield's results (section 2.4), structural features add 2-10 percentage points AUC improvement on novelty detection beyond content-only baselines. The GNN path is the upper end of this range -- it learns task-specific structural features rather than relying on generic graph encoding.

### 5.3 ONNX Determinism in TEE Contexts

The description in section 5.1 -- "model loading and inference are deterministic given the same input" -- requires a significant qualification from research4 (section A9).

**ONNX Runtime does not expose deterministic primitive selection or reduction order.** FP non-associativity combined with dynamic-batch reduction ordering means that bit-identical inference across runs is not guaranteed by the ONNX Runtime API, even with the same model and the same input. (Source needed: the previously cited arXiv:2501.05867 is a **WRONG CITATION** — that paper is "Neural network verification challenges as programming-language challenges," unrelated to ONNX non-determinism. ⚠️ SOURCE UNVERIFIED — needs re-sourcing.)

Achieving bit-identical inference requires batch-invariant kernels, which carry a throughput penalty of approximately **34-61%** (Thinking Machines/SGLang result on Qwen3-8B). The lower bound (34%) requires CUDA graph fusion; without it the cost is ~61%. (Source needed: the previously cited arXiv:2606.03019 is a **WRONG CITATION** — that paper is "Reproducibility is the New Copyleft: Defining AGI-oriented Reproducible Builds," unrelated to TEE inference or batch-invariant kernel costs. ⚠️ SOURCE UNVERIFIED — needs re-sourcing.)

For TC's TEE-hosted scoring pipeline, this means:

- If the GNN path is pursued and attestation requires deterministic scoring, bit-identical inference must be explicitly enforced via batch-invariant kernels, not assumed from the ONNX model format.
- The throughput penalty (34-61%) is a real operational cost that was not accounted for in the original estimate of the GNN path's complexity.
- This makes the GNN path more expensive in TEE contexts than section 5.1 assumed.

The brute-force HDC scan approach (section 3.3) sidesteps this entirely: Hamming distance over integer bit vectors is trivially deterministic and incurs no throughput penalty.

### 5.4 When to Pursue

The GNN path is worth the added complexity only after VS-Graph evaluation (section 7, priority 3) demonstrates that structural signal is valuable for TC's novelty scoring. If VS-Graph's HDC encoding already captures the structural signal (low Hamming distance correlates with human-judged structural similarity), the GNN adds marginal value at significant complexity cost (ONNX model management, version pinning, training pipeline, and the 34-61% determinism throughput penalty in TEE contexts).

---

## 6. Integration with Multi-Layer Pipeline (Doc 02)

Structural embedding slots in as Layer 4 of the multi-layer novelty pipeline defined in doc 02:

```
Layer 1: MinHash / LSH dedup           (< 1ms)    -- doc 02, A.2
Layer 2: NCD via zstd                  (< 10ms)   -- doc 02, A.4
Layer 3: Embedding distance            (existing)  -- BGE-large-en-v1.5 + HNSW
Layer 4: Structural / graph embedding  (NEW)       -- this document
Layer 5: LLM perplexity               (existing)  -- Qwen 3.6 35B-A3B-FP8
```

### 6.1 Layer 4 Specification

**Input**: Trace envelope (same input as Layers 1-3).

**Processing**:
1. Extract tool-call graph (section 4).
2. Compute VS-Graph HDC structural fingerprint (section 3).
3. Brute-force Hamming distance scan over all stored structural fingerprints (section 3.3). Do not use HNSW at current scale.
4. Return structural novelty score: normalized Hamming distance to nearest neighbor in [0, 1].

**Output**: Structural novelty score. High score = structurally dissimilar from all previously seen traces. Low score = structurally similar to at least one existing trace.

**Short-circuit**: If Layer 1 or Layer 2 has already flagged the trace as near-duplicate, Layer 4 can be skipped (structural comparison of near-duplicates is redundant).

**Latency budget**: < 5ms. VS-Graph's HDC operations are sub-millisecond; brute-force Hamming scan over 352 D=128 vectors is well under 1 microsecond. Total Layer 4 cost is dominated by graph extraction, not lookup. This fits comfortably within the inter-layer latency budget.

### 6.2 Score Aggregation

Layer 4's structural novelty score combines with existing layer scores in the gate evaluation. Two approaches:

**Weighted sum** (simple): `novelty = w1*minhash + w2*ncd + w3*embedding + w4*structural + w5*perplexity`. Layer 4 adds one more term. Weights tuned on the calibration set (doc 09).

**Hierarchical gating** (recommended): Each layer is a sequential filter. A trace must pass all layers to be scored as novel. Layer 4 acts as a structural filter: traces that are textually novel (passed Layers 1-3) but structurally derivative (same tool-call graph as an existing trace) are flagged. This catches the specific failure mode described in the introduction -- same tools, same order, different arguments scoring as "novel."

---

## 7. Priority Order

| Priority | What | Effort | Rationale |
|---|---|---|---|
| **1** | VS-Graph HDC structural embedding + brute-force scan (no HNSW) | Days | Pure Rust, no dependencies, composes with existing HDC fingerprints via XOR. Brute-force Hamming scan (not HNSW) provides determinism for attestation and side-channel-free lookup at TC's scale. See section 3.3. |
| **2** | Tool-call graph extraction | Days | Extends existing chunker (`parse_envelope_rendered_events`). No new parsing infrastructure needed. |
| **3** | Evaluate structural vs. text novelty | Days | Does structural Hamming distance add discriminative power beyond text embedding cosine distance? Run on ~352 existing submissions. Also validates whether VS-Graph's accuracy transfers from MUTAG/DD molecular graphs to TC's trajectory graphs. Answer determines whether priorities 4+ are worthwhile. |
| **4** | GNN via `ort` crate | Weeks | Only if priority 3 shows structural signal is valuable AND HDC encoding is insufficient. Adds learned structural features beyond generic HDC encoding. 2-10pp AUC improvement (MCPShield). Note: requires batch-invariant kernels for bit-identical inference in TEE contexts (~34-61% throughput cost — ⚠️ SOURCE UNVERIFIED, previously cited arXiv:2606.03019 is a WRONG CITATION for this claim). HNSW lookup deferred until corpus exceeds ~100K traces. |

**Critical dependency**: Priority 3 is the decision gate. If structural distance does not add power beyond text embedding distance on TC's actual corpus, the GNN path (priority 4) is not worth pursuing. VS-Graph (priority 1) and graph extraction (priority 2) are low-cost enough to build regardless -- they compose with existing infrastructure and the HDC fingerprints have value beyond novelty scoring (e.g., structural clustering, trace retrieval by topology). HNSW for structural fingerprint lookup is explicitly deferred; brute-force HDC scan is the correct initial implementation.

---

## 8. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| **Structural signal is weak** | VS-Graph adds no discriminative power beyond text embedding | Evaluate on existing corpus (priority 3) before investing in GNN path. VS-Graph is low-cost regardless. |
| **Graph extraction is noisy** | Tool-call sequences are ambiguous (conditional branches, parallel calls) | Start with sequential edges only. Add data-flow and branching detection incrementally. |
| **HDC dimensionality mismatch** | VS-Graph vectors may need different dimensionality than existing HDC fingerprints | Use same dimensionality (e.g., 10,000 bits). VS-Graph is flexible on vector size. |
| **ONNX model versioning** | GNN model updates require reindexing the structural fingerprint index | Version-pin the model. Reindex on model update (same as text embedding model update). |
| **ONNX non-determinism in TEE** | ONNX Runtime does not guarantee bit-identical inference (⚠️ SOURCE UNVERIFIED — previously cited arXiv:2501.05867 is a WRONG CITATION; that paper is about NN verification as PL challenges); required batch-invariant kernels carry 34-61% throughput cost (⚠️ SOURCE UNVERIFIED — previously cited arXiv:2606.03019 is a WRONG CITATION; that paper is about reproducible builds). Both technical claims are likely correct but need re-sourcing. | Use brute-force HDC scan (section 3.3) instead of GNN inference where attestation requires determinism. If GNN is pursued, explicitly enforce batch-invariant kernels; do not assume ONNX determinism. |
| **Withdrawn paper confusion** | GraphTracer's invalidated claims cited by downstream work | Explicitly note withdrawal in all references. Do not cite empirical claims. |

---

## 9. Deep Research Queries

### Q-SE1: HDC for Graph Classification

```
"hyperdimensional computing" OR "HDC" graph classification embedding structure 2025 2026
```
**Looking for:** HDC-based graph classification methods beyond VS-Graph. Alternative encoding schemes for directed graphs. Benchmark comparisons with GNN-based graph embeddings.

### Q-SE2: Tool-Call Graph Extraction from Agent Traces

```
"tool-call graph" OR "action graph" agent trace extraction dependency 2026
```
**Looking for:** Methods for extracting structured graphs from agent execution logs. Data-flow edge detection between tool calls. Handling of parallel and conditional invocations.

### Q-SE3: Structural Novelty in Software Traces

```
"structural novelty" OR "graph novelty" software trace detection embedding 2026
```
**Looking for:** Systems that detect structural novelty (as opposed to content novelty) in software execution traces. Applicable methods for distinguishing "same tools, different order" from "same tools, same order."

### Q-SE4: ONNX Runtime in TEEs

```
"ONNX Runtime" OR "ort" trusted execution environment TEE inference deterministic 2026
```
**Looking for:** Experience reports on running ONNX Runtime inside TEEs. Determinism guarantees for model inference in encrypted enclaves. Compatibility with Intel TDX.

---

## 10. Verification Ledger

All papers cited in this document have been verified against arXiv or conference proceedings.

| Paper | ID / Venue | Status | Notes |
|---|---|---|---|
| GraphTracer | arXiv:2510.10581 | **WITHDRAWN** (Dec 2025) | v2 withdrawn for methodology error. IDG concept valid; empirical claims invalidated. |
| GRADE | arXiv:2606.22741 | **Verified** | Graph Representation of LLM Agent Dependency and Execution. |
| AgentGraph | AAAI 2026 (ojs.aaai.org/index.php/AAAI/article/view/42393) | **Verified** | Causal attribution + perturbation robustness on logged traces. |
| MCPShield | arXiv:2605.11053 | **Verified** | Content-aware tool-call attack detection via GNN. Structural features add 2-10pp for novelty. |
| Agent-OSI | arXiv:2602.13795 | **Verified** | L5 provenance interface for agent systems. |
| VS-Graph | arXiv:2512.03394 | **Verified** | HDC-native graph embedding. 450x faster than GNNs on MUTAG/DD molecular datasets. Accuracy on TC's trajectory graphs requires validation (section 3.1.1). |
| ONNX Runtime non-determinism | arXiv:2501.05867 | **WRONG CITATION** | arXiv:2501.05867 is "Neural network verification challenges as programming-language challenges" — unrelated to ONNX non-determinism. The technical claim (ONNX Runtime does not expose deterministic primitive selection or reduction order) is likely correct but the citation is wrong. Needs re-sourcing. |
| Batch-invariant deterministic inference | arXiv:2606.03019 | **WRONG CITATION** | arXiv:2606.03019 is "Reproducibility is the New Copyleft: Defining AGI-oriented Reproducible Builds" — unrelated to deterministic TEE inference or batch-invariant kernel costs. The technical claim (~34-61% throughput cost, Thinking Machines/SGLang result on Qwen3-8B) may be real but the citation is wrong. Needs re-sourcing. |

*5 verified papers + 1 withdrawn + 2 wrong citations corrected. Last updated August 2026 (v6, post-v6 citation audit).*
