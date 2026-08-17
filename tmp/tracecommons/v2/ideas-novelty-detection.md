# Ideas for Fixing Novelty Detection in TraceCommons

Brainstorming notes from studying the gate-enclave codebase and related
literature. Nothing here is a spec or a recommendation -- these are ideas that
might help, ordered roughly by how much existing infrastructure they can reuse.

---

## 1. The Core Problem

The A2.6 model bake-off that selected the production perplexity scorer was
confounded. PR #216 found that six trivial no-model baselines -- paragraph
count, line count, word count, byte count, distinct word count, and mean word
length -- ALL beat the winning model on the evaluation corpus. Paragraph count
achieved AUC 1.000 because every duplicate in the test corpus had exactly 1
paragraph while novel files had 7-163 paragraphs. The corpus builder entangled
the class label with source format: "duplicate" meant "a file that looks
structurally different from novel files regardless of its content."

This matters because the gate's novelty signal -- the thing that decides
whether an agent trace is original enough to earn credit -- is calibrated
against results from a corpus where the scoring model was never actually
tested against real novelty discrimination. The floor thresholds, the
embedding model choice, the perplexity scorer selection -- all downstream
decisions rest on a leaky evaluation.

The gate pipeline itself (`EnclaveGateOrchestrator::evaluate` in
`trace-commons-gate-enclave/src/orchestrator.rs`) is well-structured: chunk,
score perplexity, embed, compare to vector index, gate on both signals. The
traits (`PerplexityScorer`, `Embedder`, `VectorIndex`, `TokenRarityScorer`)
are clean plugin points. The problem is not the architecture -- it is the
measurement methodology that was used to select what goes behind those traits,
and some gaps in the scoring pipeline itself.


## 2. Idea: Multi-Layer Novelty Pipeline

Instead of running one embedding distance check against the vector index (the
current `1 - max(cosine_similarity)` path in `evaluate`), one approach worth
exploring is layering cheap, fast filters before the expensive ones. Each
layer could be a separate trait implementation, with early short-circuiting
when a duplicate is detected at a cheaper tier.

A possible layering:

**Layer 1: MinHash / LSH dedup (< 1ms).** The Rensa crate (Rust
MinHash, reportedly 608x faster than Python datasketch) could generate
fixed-size fingerprints for each trace's rendered text. Two traces whose
MinHash Jaccard estimate exceeds, say, 0.9 are near-duplicates regardless of
what the embedding model thinks. This catches the verbatim and
near-verbatim copies that are the easiest duplicates to miss when your
embedding model is weak. The fingerprint is tiny (a few hundred bytes) and
could be stored alongside the vector entry. This layer would be purely
additive -- it does not replace the embedding check, it just short-circuits
it for the easy cases.

**Layer 2: NCD via zstd (< 10ms).** Normalized Compression Distance is
parameter-free and compression-based: compress A, compress B, compress AB
concatenated, compute `NCD(x,y) = (C(xy) - min(C(x), C(y))) / max(C(x),
C(y))`. It approximates Kolmogorov complexity and catches structural
similarity that token-level hashing misses. See Section 5 below for more on
this.

**Layer 3: Embedding distance (the current approach, potentially improved).**
The existing `Embedder` trait path, but possibly with a better embedding model
(Section 9).

**Layer 4: Structural analysis.** For agent traces specifically, the sequence
of tool calls and message types is itself a signal. Two traces that follow
the same tool-call DAG are structurally similar even if the natural language
content differs. See Section 6 on process mining.

**Layer 5: LLM perplexity (the current scorer path, unchanged).**

The key property of this layering is that each layer is independently
implementable behind a trait. The orchestrator's `evaluate` method currently
runs perplexity and embedding sequentially; extending it to run additional
pre-filter layers before the embedding step would be a local change to
`EnclaveGateOrchestrator`. Each layer either short-circuits ("this is a
duplicate, stop") or lets the trace through to the next layer. The final gate
decision remains the AND of all layer verdicts, consistent with the existing
fail-closed design.

One thing to be careful about: each new layer is a new dimension of
calibration. Adding MinHash without a corpus that tests MinHash specifically
would repeat the original bake-off mistake. But the advantage is that MinHash
and NCD are well-understood algorithms with known properties -- their
false-positive rates are analytically derivable, unlike a model whose
behavior depends on its training distribution.


## 3. Idea: Fix the Bake-Off Corpus First

PR #216 is the right fix for the decision rule (use the no-model baselines to
sanity-check future bake-off runs), but the corpus itself needs fixing before
any model comparison is meaningful.

The central issue is that the corpus conflates format with novelty class. Some
ideas for building a better one:

**Need paraphrase pairs.** The current corpus lacks traces that are
semantically identical but syntactically different. But there is a known
problem with the synthetic paraphrase approach: 299 out of 300 synthetic
paraphrases generated so far are shorter than their originals, with a median
length ratio of 0.282. This means the paraphraser is summarizing, not
paraphrasing -- a length-based classifier would detect these trivially,
repeating the original confound. Any paraphrase generation approach probably
needs a length-matching constraint (e.g., reject outputs where
`|len_out/len_in - 1| > 0.2`).

**Need a held-out evaluation set with human annotations.** This is the
uncomfortable truth: without human judgment on "is this trace novel?", there
is no ground truth to calibrate against. Process mining literature suggests
that annotating 200+ traces with 3+ independent reviewers and measuring
inter-annotator agreement (Krippendorff's Alpha) is the minimum for a usable
evaluation corpus. The cost is real (probably 40-80 person-hours for 200
traces) but without it, any automated metric is validated against other
automated metrics -- which is what got the bake-off into trouble in the first
place.

**Ranking-based annotation might be easier than absolute scoring.** Asking
reviewers "is Trace A more novel than Trace B?" produces more reliable
judgments than asking "rate this trace's novelty from 1-5." Pairwise
rankings can be converted to total orderings via Bradley-Terry or Elo, and
inter-annotator agreement on rankings tends to be higher than on absolute
scales. This approach could piggyback on the corpus map + trace triage
infrastructure from PR #173's Phase 2.

**Stratify by source, length, and structural features.** The new corpus
should explicitly control for the confounds PR #216 found. Every class
(novel, duplicate, paraphrase, near-duplicate) should have examples at every
length quintile and every paragraph-count range. A corpus where "novel" is
uniformly long and "duplicate" is uniformly short is broken by construction;
this must be a design-time invariant, not a post-hoc discovery.


## 4. Idea: Wire TokenRarityScorer into the Live Gate

This one is interesting because the code already exists. The
`TokenRarityScorer` trait is defined in `trace-commons-gate-api`, the
`per_token_rarity_micros` function in `perplexity_local.rs` computes
`exp(-mean(K rarest logprobs))`, the mock scorer (`MockTokenRarityScorer`)
is implemented, the bake-off calibration tooling already evaluates it
alongside perplexity, and `global_rarity_micros_across_chunks` in
`chunk_aggregate.rs` already handles the cross-chunk aggregation.

But the live gate path -- `EnclaveGateOrchestrator::evaluate` -- does not
call it. The `evaluate` method scores perplexity per-chunk and novelty via
embedding distance, but token rarity is computed only in the bake-off
tooling.

One idea: wire `TokenRarityScorer` as an optional fourth component of
`EnclaveGateOrchestrator`. Since the production `LocalPerplexityScorer`
already does a forward pass that produces per-token logprobs, and the rarity
metric is derived from the same logprob vector, this would cost zero
additional inference -- just a sort + mean over the logprobs that are already
computed. The rarity score could serve as a secondary quality signal: a trace
with high aggregate perplexity but low token rarity might be "surprising" only
because it is incoherent (random noise has high perplexity but low rarity).
The combination might be a better proxy for "genuinely novel content" than
either signal alone.

This is probably the lowest-effort change with the most information gain,
because it is already built and tested. The only new code would be the plumbing
in `evaluate` to call `global_rarity_micros_across_chunks` and include the
result in `OrchestrationDecision`.


## 5. Idea: NCD as a Cheap Pre-Filter

Normalized Compression Distance, introduced by Li et al. (2004), uses a
real-world compressor as a proxy for Kolmogorov complexity. The formula:

```
NCD(x, y) = (C(xy) - min(C(x), C(y))) / max(C(x), C(y))
```

where `C(z)` is the compressed size of `z`. NCD ranges from 0 (identical) to
approximately 1 (maximally dissimilar). It is parameter-free, O(n) per
comparison, and trivially TEE-compatible (zstd is a pure function of its
input, no model weights or state).

One approach: for each incoming trace, compute `NCD(trace, sample_i)` against
100 randomly sampled corpus entries from the tenant's history. The minimum NCD
across samples is a cheap novelty signal -- a trace that compresses well
against any existing trace is probably a near-duplicate. This could serve as
a fast pre-filter before the embedding path: traces with `min_NCD < 0.3`
(say) are flagged as likely duplicates and skip the expensive embedding +
vector lookup.

The "Less is More" paper from ACL 2023 showed that NCD with gzip
outperformed fine-tuned BERT on several text classification tasks. The result
was controversial (see the rebuttals about implementation artifacts), but the
core insight -- that compression captures structural similarity without a
model -- holds. For TC's use case, the relevant question is not whether NCD
beats embeddings in general, but whether it catches a class of duplicates
that the embedding path misses (specifically: structurally similar traces
whose embedding distance is above the floor because the embedding model
lacks domain-specific training).

Implementation note: zstd is a better compressor choice than gzip for this
use case. Its dictionary-based compression mode can be pre-trained on a
sample of the tenant's corpus, which would make NCD comparisons more
discriminative without adding model parameters. The `zstd` Rust crate is
mature and already used in many infrastructure projects.

A rough implementation might look like:

```
trait CompressionNoveltyFilter: Send + Sync {
    fn is_likely_duplicate(&self, trace: &[u8], tenant: &str) -> Result<bool>;
    fn ncd_score(&self, trace: &[u8], reference: &[u8]) -> Result<f64>;
}
```

This would slot in before the `Embedder` call in the orchestrator pipeline.


## 6. Idea: Process Mining for Trace Sequences

Agent traces are not arbitrary text -- they are structured records of
processes. Each trace is a sequence of events with types (`user_message`,
`assistant_message`, `tool_call`), tool names, and content. The chunker in
`chunker.rs` already parses this structure via `parse_envelope_rendered_events`,
and `render_event_text` formats it as `event_type (tool_name): content`.

This means there is an implicit process model in the data that the current
scoring pipeline throws away. The embedding path embeds the rendered text,
which captures the content but not the control flow. Two traces that call the
same tools in the same order with different arguments are structurally
identical but might score as "novel" under embedding distance.

Process mining is the established field for exactly this kind of analysis.
The idea: build a process model (e.g., a directly-follows graph, or a Petri
net via alpha-miner) from historical traces in a tenant's corpus. Score new
traces by conformance deviation -- how different is this trace's tool-call
sequence from the historical model? High deviation means the agent tried a
novel approach. Low deviation means it followed a well-trodden path.

Recent work that might be relevant:

- "Detecting Anomalous Patterns in Process Executions" (2025) describes
  algorithms for scoring trace conformance against a learned process model,
  with both frequency-based and structural novelty.
- "Control-flow Anomaly Detection by Process Mining" (2025) applies
  process-mining conformance checking to detect anomalous executions,
  which is conceptually similar to detecting novel agent behavior.

Nobody seems to be applying process mining specifically to AI agent traces,
which makes this a potentially novel contribution for TC. The tool-call
sequence in an agent trace IS a process; the events even come with types
and names that map directly to activities in process mining notation.

A concrete approach might extract just the `(event_type, tool_name)` pairs
from each trace (ignoring content), build a frequency-weighted
directly-follows graph per tenant, and score new traces by the fraction of
their transitions that are unseen in the graph. This would be cheap (just
counting bigrams of event types) and complementary to the content-based
embedding novelty.


## 7. Idea: Novelty = harmonic_mean(Originality, Quality)

Padmakumar et al. (2025), in their study of what makes creative AI output
"novel," propose decomposing novelty into two orthogonal dimensions:

- **Originality**: how different is this from what has been seen before?
- **Quality**: how good is this at achieving its stated purpose?

Their key insight: novelty should be the harmonic mean of originality and
quality, not just originality alone. This prevents gaming -- you cannot be
"novel" by producing garbage. Random noise is maximally original but zero
quality; a template-following high-quality trace is high quality but zero
originality. Only traces that are both original and effective score high on
the harmonic mean.

For TC, this might map to:

- **Originality**: the existing novelty signal (embedding distance, possibly
  enhanced with NCD/MinHash/process mining from the ideas above). Measures
  "fraction of trace content/structure unseen in the tenant's history."
- **Quality**: did the agent achieve its goal? This could be derived from
  trace-internal signals (did the conversation end with a successful tool
  call? was there an explicit success/failure marker?) or from external
  annotation.

The current gate formula already has a structure that could accommodate this.
The credit quality formula `q = f * g * a` (perplexity term x novelty term x
anomaly penalty) is a product of three factors. Replacing the novelty term
with `harmonic_mean(originality, quality)` would penalize traces that are
original-but-useless or useful-but-derivative. The challenge is defining
"quality" in a way that is automatable and not gameable -- which circles back
to the human annotation question in Section 3.


## 8. Idea: NovAScore Decomposition

Ai et al. (COLING 2025) propose a method for evaluating the novelty of
generated text by decomposing it into Atomic Content Units (ACUs) -- the
smallest meaningful claims or actions in the text. Each ACU is scored for
novelty against a historical bank of ACUs, and the overall novelty score is
a salience-weighted aggregate.

For TC, this could work as follows:

1. **Decompose**: extract ACUs from the agent trace. For agent traces, an
   ACU might be: a tool call with specific arguments, a decision to use one
   approach over another, a novel combination of tools, a specific code
   pattern produced. The chunker already segments traces into events; each
   event could be treated as an ACU, or events could be further decomposed.

2. **Score each ACU**: compare each ACU against a bank of historical ACUs
   (stored per tenant). An ACU that has never been seen before contributes
   maximally to novelty; one that appears in every trace contributes nothing.
   The comparison could use embedding distance (the existing `Embedder` trait)
   or exact/fuzzy matching on the structured event representation.

3. **Weight by salience**: not all ACUs matter equally. A novel tool call
   that was critical to the outcome matters more than a novel but irrelevant
   log message. Salience could be approximated by position in the trace
   (events near the resolution tend to matter more) or by the event type
   (tool_call events are more salient than user_message events for judging
   agent capability).

4. **Aggregate**: the weighted sum of ACU novelty scores is the trace's
   overall novelty.

This is probably the most principled approach for TC's specific problem,
because it directly addresses the question "what specifically is novel about
this trace?" rather than treating the trace as a bag of text. But it is also
the most complex to implement -- it requires an ACU extraction step, a
historical ACU bank, and a salience model, none of which exist today.

One advantage: ACU-level novelty is intrinsically explainable. When a trace
scores low on novelty, you can point to the specific ACUs that matched
historical ones. When it scores high, you can identify the specific novel
actions. This is useful for the human review pipeline (Section 10) and for
debugging the scoring model.


## 9. Idea: Better Embeddings

The current production embedding path uses BGE-large-en-v1.5, a
general-purpose sentence embedding model. The reference embedder
(`ReferenceEmbedder` in `trace-commons-gate-api/src/reference.rs`) is a
bag-of-tokens hash: it splits on whitespace, lowercases, hashes each token
into one of 256 buckets, and L2-normalizes. The reference embedder explicitly
cannot detect paraphrases -- its docstring says "paraphrases with no shared
tokens look maximally novel, which is precisely the weakness a production
embedder fixes."

Some ideas for embedding improvements:

**Code-aware embeddings.** Agent traces are not pure natural language -- they
contain code snippets, shell commands, API calls, error messages, and
structured data. A general-purpose sentence embedder was not trained on this
distribution. GraphCodeBERT or CodeBERT embeddings might capture the
code-structure similarity that BGE misses. This matters because two traces
that generate functionally equivalent code but with different variable names
should score as duplicates, and a pure text embedder might not see that.

**Contrastive fine-tuning on TC data.** If a labeled corpus of
novel/duplicate trace pairs can be assembled (Section 3), contrastive
learning (e.g., SimCSE or CoSENT) can fine-tune the embedding model to
maximize distance between novel pairs and minimize distance between duplicate
pairs, specifically on TC's data distribution. This is the standard
approach for domain adaptation of embeddings, and it is probably the single
highest-leverage improvement to the embedding path -- but it requires a
labeled dataset that does not yet exist.

**Multi-view embedding.** Agent traces have multiple views: the natural
language content, the code content, the tool-call structure, and the
temporal ordering. A single embedding conflates all of these. One approach is
to embed each view separately and compare per-view, with the final novelty
score being the minimum across views (a trace must be novel in every view to
score as novel). This is more expensive but more discriminative -- it catches
traces that are novel in text but derivative in structure (or vice versa).

**Matryoshka embeddings.** Some recent embedding models (e.g., nomic-embed)
support truncated embedding dimensions -- the first 64 dimensions capture
coarse similarity, the full 768 dimensions capture fine-grained similarity.
This could speed up the multi-layer pipeline: use the first 64 dimensions for
a fast coarse filter, then the full dimensions for the precise novelty
calculation.


## 10. Idea: Human-in-the-Loop Calibration

PR #173's Phase 2 (corpus map + trace triage) is the measurement
infrastructure that the scoring pipeline is missing. Without human annotation,
every automated novelty metric is validated against other automated metrics --
the definition of circular reasoning. The bake-off demonstrated this: the
winning model was selected by comparing its AUC against a corpus whose labels
were themselves constructed by an automated process that had a format confound.

Building the annotation tool is probably more important than improving any
individual scorer, because it creates the feedback loop that lets you tell
whether improvements to scorers actually work. Here is what that might look
like:

1. **Sample 200+ traces** from production, stratified by length, event count,
   tenant, and current novelty score (to ensure coverage of the score
   distribution, not just the tails).

2. **Annotation UI**: a simple trace viewer that shows the rendered events
   (the output of `render_event_text`) and asks 3+ reviewers per trace:
   "Have you seen a trace substantially similar to this one?" with options
   of "yes, link to the similar trace", "no, this is novel", "unclear, skip."
   The pairwise ranking approach from Section 3 could be used alongside or
   instead of binary labels.

3. **Agreement metric**: compute Krippendorff's Alpha on the annotations.
   If alpha < 0.67, the annotation task is too ambiguous and the labeling
   guidelines need revision before proceeding. If alpha > 0.8, the labels
   are reliable enough to use as ground truth.

4. **Calibrate**: run every candidate scorer (perplexity, token rarity,
   embedding distance, NCD, MinHash, process-mining conformance) on the
   labeled corpus. Compute AUC, precision-recall, and calibration curves.
   The scorer with the best AUC on human-labeled data is the right choice --
   not the one with the best AUC on an automatically constructed corpus.

This is a substantial time investment, but it is also the only way to know
whether any of the ideas in this document actually work. Without it,
improvements are measured against a broken ruler.


## 11. What I'd Try First (Personal Take)

If I were working on this, here is the order I would explore these ideas.
This is an opinionated prioritization based on effort-to-information-gain
ratio, not a recommendation:

1. **Wire TokenRarityScorer into the live gate (Section 4).** Already built,
   already tested, zero new inference cost (reuses the same logprob vector).
   The information gain is: does rarity separate novel from duplicate traces
   better than aggregate perplexity? The bake-off tooling already computes
   both; comparing their AUC on a corrected corpus would answer this quickly.
   Even without a corrected corpus, having the rarity signal in production
   audit rows means it can be retrospectively analyzed once a labeled corpus
   exists.

2. **Add a MinHash dedup layer (Section 2, Layer 1).** The Rensa crate makes
   this fast to prototype. MinHash catches verbatim and near-verbatim
   duplicates with analytically known false-positive rates, which means it
   can be evaluated without a labeled corpus -- you just need to verify that
   known duplicates produce high Jaccard estimates and known non-duplicates
   produce low ones. This is a cheap, well-understood first layer that
   reduces the load on the more expensive embedding path.

3. **Fix the bake-off corpus (Section 3).** This is foundational but
   time-consuming. The key insight from PR #216 is that the corpus must
   control for format features (length, paragraph count, structure) across
   all classes. Building a corpus that satisfies this invariant is probably
   a week of work, but every subsequent model comparison depends on it.

4. **Build human annotation infrastructure (Section 10, PR #173 Phase 2).**
   200+ traces, 3+ reviewers, Krippendorff's Alpha. This is the feedback
   loop that makes everything else measurable. Without it, we are flying
   blind.

5. **Try NCD as a cheap novelty signal (Section 5).** Straightforward to
   implement, parameter-free, and it captures a different kind of similarity
   (structural/compressible) than the embedding path (semantic). Worth
   benchmarking against the embedding path on a corrected corpus to see if
   it adds discriminative power or is redundant.

6. **Fine-tune embeddings on TC data (Section 9).** This is high-leverage but
   requires a labeled corpus (from steps 3-4). Once the corpus exists,
   contrastive fine-tuning is a standard recipe and likely to produce the
   biggest improvement in the embedding path's discrimination.

The ideas I would defer:

- **Process mining (Section 6)**: interesting and potentially unique to TC,
  but the implementation is more involved and the payoff is uncertain without
  first knowing whether the simpler approaches work.
- **NovAScore decomposition (Section 8)**: the most principled approach, but
  also the most complex. Worth revisiting after the simpler layers are in
  place and the annotation infrastructure exists to measure them.
- **Harmonic mean of originality and quality (Section 7)**: conceptually
  clean, but "quality" is hard to define and measure for agent traces. This
  is more of a long-term research direction than a near-term fix.


## References

- Li, M., Chen, X., Li, X., Ma, B., and Vitanyi, P.M.B. (2004). "The
  Similarity Metric." IEEE Transactions on Information Theory, 50(12).
  The original NCD paper.
  https://doi.org/10.1109/TIT.2004.838101

- Jiang, Z., Yang, M., Tsvetkov, M., He, P., and Gao, J. (2023). "Low-
  Resource Text Classification: A Parameter-Free Classification Method with
  Compressors" (commonly known as "Less is More"). ACL 2023.
  https://aclanthology.org/2023.findings-acl.426/

- Padmakumar, V., He, H., and Daume III, H. (2025). "Does Writing with
  Language Models Reduce Content Diversity?" ICLR 2025.
  https://arxiv.org/abs/2309.05196

- Ai, M., et al. (2025). "NovAScore: A New Automated Metric for Evaluating
  Document-Level Novelty." COLING 2025.
  https://aclanthology.org/2025.coling-main.53/

- van der Aalst, W.M.P. (2016). "Process Mining: Data Science in Action."
  2nd edition, Springer. The canonical reference on process mining.
  https://doi.org/10.1007/978-3-662-49851-4

- Nolle, T., Seeliger, A., and Muhlhauser, M. (2018). "BINet: Multivariate
  Business Process Anomaly Detection Using Deep Learning." International
  Conference on Business Process Management.
  https://doi.org/10.1007/978-3-319-98648-7_16

- Bezerra, F. and Wainer, J. (2013). "Algorithms for Anomaly Detection of
  Traces in Logs of Process Aware Information Systems." Information Systems,
  38(1).
  https://doi.org/10.1016/j.is.2012.04.004

- Gao, T., Yao, X., and Chen, D. (2021). "SimCSE: Simple Contrastive
  Learning of Sentence Embeddings." EMNLP 2021.
  https://aclanthology.org/2021.emnlp-main.552/

- Rensa: Rust MinHash library.
  https://github.com/beowolx/rensa

- Nussbaumer, D., Waldis, A., and Lehmann, M. (2025). "Detecting Anomalous
  Patterns in Process Executions with LLMs." Process Mining Workshops, ICPM
  2024.
  https://doi.org/10.1007/978-3-031-78091-4_1

- Ko, J. and Comuzzi, M. (2025). "Control-flow Anomaly Detection by Process
  Mining." Intelligent Data Analysis, 29(1).
  https://doi.org/10.1177/1748006X241310058
