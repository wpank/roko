# Domain Catalog: Concrete Arenas

Each domain below maps to an `Arena` implementation: a task source, gate configuration,
scoring function, and prompt enrichment strategy. The learning infrastructure
(CascadeRouter, playbooks, experiments, episodes) is shared across all of them.

---

## 1. Code Benchmarks

### SWE-bench (GitHub issue resolution)

- **Task source**: HuggingFace `princeton-nlp/SWE-bench_Lite` (300 tasks) or full (2294)
- **Instance**: repo + base_commit + issue description + gold patch (for oracle)
- **Gates**: `git apply --check` (fast proxy), instance test command (accurate)
- **Scoring**: % resolved (patch applies AND tests pass)
- **Enrichment**: oracle file contents, repo structure context
- **Pacing**: batch (50-100 per iteration)
- **Learning signal**: which models produce valid diffs for which frameworks

### MBPP / HumanEval (function synthesis)

- **Task source**: HuggingFace `google-research-datasets/mbpp` (974 tasks), `openai/openai_humaneval` (164)
- **Instance**: function docstring + test cases
- **Gates**: execute generated function against test cases
- **Scoring**: pass@k (k attempts, any pass counts)
- **Enrichment**: function signature, type hints
- **Pacing**: batch, fast turnaround (~seconds per instance)
- **Learning signal**: which prompt structures produce syntactically correct code

### CodeContests (competitive programming)

- **Task source**: HuggingFace `deepmind/code_contests`
- **Instance**: problem statement + input/output examples + hidden tests
- **Gates**: execute against all test cases (including hidden)
- **Scoring**: % solved
- **Enrichment**: example I/O pairs, constraints
- **Pacing**: batch
- **Learning signal**: which models handle algorithmic reasoning, long context

### Aider Polyglot (multi-language editing)

- **Task source**: Aider's polyglot benchmark (various languages)
- **Instance**: file + edit instruction + expected output
- **Gates**: diff comparison against expected
- **Scoring**: % correct edits
- **Learning signal**: per-language model preference (Rust vs Python vs Go)

---

## 2. Blockchain & DeFi

### Chain Monitor (real-time event processing)

- **Task source**: Ethereum/L2 block stream (subscription via WebSocket RPC)
- **Instance**: new block → transactions → events → decode
- **Gates**: prediction verification (did the predicted outcome occur?)
- **Scoring**: prediction accuracy, latency, profit (if trading)
- **Enrichment**: recent block history, mempool state, DEX reserves
- **Pacing**: real-time (per-block, ~12s on mainnet)
- **Learning signal**: which patterns predict MEV, liquidations, large swaps

Existing infrastructure: `roko-chain` has backend-agnostic `ChainClient` trait,
event watching, MEV detection, tx simulation via Revm.

### DeFi Strategy (trading decisions)

- **Task source**: market conditions (price feeds, liquidity depth, volatility)
- **Instance**: "given this market state, what trade should be executed?"
- **Gates**: Revm fork simulation (pre-flight), balance verification (post-execution)
- **Scoring**: PnL, Sharpe ratio, max drawdown
- **Enrichment**: Nelson-Siegel yield curves, order flow, historical patterns
- **Pacing**: real-time to hourly depending on strategy
- **Learning signal**: which models make profitable decisions under which conditions

Existing infrastructure: trading tools (uniswap_get_quote, execute_swap, etc.),
3-mode custody, session keys with caveat enforcers.

### MEV Detection & Protection

- **Task source**: mempool monitoring (pending transactions)
- **Instance**: pending tx → classify as potential MEV target
- **Gates**: was the prediction correct? (check next block for sandwich/frontrun)
- **Scoring**: true positive rate, false positive rate
- **Enrichment**: gas price distribution, DEX reserve state
- **Pacing**: real-time (~seconds)
- **Learning signal**: MEV pattern recognition across different DEX protocols

### On-Chain Fact Verification (ISFR)

- **Task source**: claims submitted to Intersubjective Fact Registry
- **Instance**: claim + evidence → verify or refute
- **Gates**: clearing cycle (submission → reveal → settlement)
- **Scoring**: accuracy vs eventual ground truth
- **Learning signal**: which verification strategies work for which claim types

---

## 3. Security

### Vulnerability Detection (CVE dataset)

- **Task source**: HuggingFace datasets of known CVEs, or curated vulnerable code snippets
- **Instance**: code snippet + known vulnerability → detect and describe
- **Gates**: does the report match the known CVE? does the fix compile?
- **Scoring**: true positive rate, false positive rate, fix quality
- **Enrichment**: OWASP taxonomy, CWE database, recent CVE patterns
- **Pacing**: batch
- **Learning signal**: which models detect which vulnerability classes

### Penetration Testing (CTF challenges)

- **Task source**: CTF challenge archives (picoCTF, HackTheBox, etc.)
- **Instance**: challenge description + target → find the flag
- **Gates**: flag matches expected value
- **Scoring**: % solved, time to solve
- **Enrichment**: tool documentation, common exploit patterns
- **Pacing**: batch (minutes to hours per challenge)
- **Learning signal**: which reasoning strategies work for which challenge types

### Code Audit (smart contract security)

- **Task source**: known-vulnerable smart contracts (SWC registry, rekt database)
- **Instance**: contract source → identify vulnerabilities
- **Gates**: compare against known vulnerabilities, verify suggested fix compiles
- **Scoring**: recall (found / total known), precision, fix quality
- **Enrichment**: Solidity patterns, reentrancy taxonomy, ERC standards
- **Pacing**: batch
- **Learning signal**: which models understand Solidity semantics

---

## 4. Research & Knowledge

### Literature Synthesis

- **Task source**: research questions paired with relevant paper sets
- **Instance**: question + N papers → produce synthesis with citations
- **Gates**: citation accuracy (do citations exist?), fact verification, coherence
- **Scoring**: citation precision, recall, synthesis quality (LLM judge)
- **Enrichment**: paper abstracts, related work sections
- **Pacing**: batch (minutes per synthesis)
- **Learning signal**: which models produce accurate citations, avoid hallucination

### Question Answering (domain-specific)

- **Task source**: HuggingFace QA datasets (SQuAD, TriviaQA, Natural Questions, domain-specific)
- **Instance**: context + question → extract or generate answer
- **Gates**: exact match or F1 against gold answer
- **Scoring**: EM, F1
- **Enrichment**: retrieval-augmented context
- **Pacing**: batch, fast
- **Learning signal**: which models handle which question types

### Fact Checking

- **Task source**: claim datasets (FEVER, ClaimBuster, etc.)
- **Instance**: claim → verify/refute with evidence
- **Gates**: label matches gold (supported/refuted/not enough info)
- **Scoring**: accuracy, evidence quality
- **Learning signal**: which reasoning chains produce accurate judgments

---

## 5. Operations & Infrastructure

### Incident Response

- **Task source**: historical incident reports (PagerDuty archives, postmortems)
- **Instance**: alert + system state → diagnose root cause + propose fix
- **Gates**: does diagnosis match known root cause? does proposed fix address it?
- **Scoring**: diagnostic accuracy, time to root cause
- **Enrichment**: system topology, recent metrics, runbook library
- **Pacing**: real-time (simulated) or batch (historical)
- **Learning signal**: which diagnostic strategies work for which failure modes

### Infrastructure as Code

- **Task source**: desired-state descriptions → generate Terraform/Pulumi/etc.
- **Instance**: "deploy a Redis cluster with 3 replicas in us-east-1"
- **Gates**: `terraform validate`, `terraform plan` (no errors), cost estimate within budget
- **Scoring**: validates, plans cleanly, cost-optimal
- **Enrichment**: cloud provider docs, existing infrastructure context
- **Pacing**: batch
- **Learning signal**: which models produce valid IaC for which cloud providers

### Log Analysis

- **Task source**: log streams with known anomalies
- **Instance**: N minutes of logs → identify anomaly and root cause
- **Gates**: anomaly detection matches known label
- **Scoring**: precision, recall, mean time to detection
- **Enrichment**: service topology, baseline metrics
- **Pacing**: real-time (streaming) or batch
- **Learning signal**: which log patterns indicate which failure modes

---

## 6. Data & Analytics

### SQL Generation (Text-to-SQL)

- **Task source**: HuggingFace `spider` dataset (10,181 questions, 200 databases)
- **Instance**: natural language question + database schema → SQL query
- **Gates**: execute query against database, compare result to gold answer
- **Scoring**: execution accuracy, exact match
- **Enrichment**: schema documentation, sample rows
- **Pacing**: batch, fast
- **Learning signal**: which models handle complex joins, subqueries, aggregations

### Data Pipeline Debugging

- **Task source**: broken data pipelines with known failures
- **Instance**: pipeline definition + error logs → identify and fix the issue
- **Gates**: pipeline runs successfully after fix
- **Scoring**: fix rate, time to fix
- **Learning signal**: which pipeline failure modes are solvable

---

## 7. Multi-Modal

### Document Understanding

- **Task source**: HuggingFace document QA datasets (DocVQA, etc.)
- **Instance**: document image + question → answer
- **Gates**: answer matches gold
- **Scoring**: ANLS, exact match
- **Enrichment**: OCR context, document structure
- **Learning signal**: which vision models handle which document types

### Image Generation Evaluation

- **Task source**: prompt + desired output criteria
- **Instance**: text prompt → generate image → evaluate
- **Gates**: CLIP score against prompt, aesthetic score, safety classifier
- **Scoring**: aggregate quality metrics
- **Learning signal**: which prompt structures produce better images with which models

---

## 8. Self-Hosting (Meta-Arena)

### Roko Developing Roko

- **Task source**: PRDs, implementation plans, GitHub issues
- **Instance**: task description → write code → pass gates
- **Gates**: compile + test + clippy + diff review (the existing 11-gate pipeline)
- **Scoring**: gate pass rate, cost per task, time to completion
- **Enrichment**: codebase context (roko-index), prior episodes, playbooks
- **Pacing**: continuous
- **Learning signal**: which models write better Rust, which prompt strategies produce
  code that passes clippy on first try

This is the meta-arena — improvements to the arena system are themselves tasks in this
arena. The fixed-point where the system improves its own improvement mechanism.

---

## Cross-Domain Synergies

| Arena A | Arena B | Transfer |
|---------|---------|----------|
| SWE-bench | Self-hosting | Patch formatting playbooks |
| Chain monitor | Security audit | Time-pressure model routing |
| Research synthesis | PRD drafting | Citation and structure playbooks |
| SQL generation | Data pipeline | Schema understanding |
| Vulnerability detection | Smart contract audit | Security reasoning patterns |
| Incident response | Log analysis | Diagnostic strategies |
| Code benchmarks (all) | Self-hosting | Per-language model preferences |

The neuro store (HDC-indexed) enables retrieval across arenas. An insight learned
in SWE-bench ("Django ORM migrations need both forward and reverse") can be retrieved
when a self-hosting task involves database schema changes.

---

## Implementation Priority

### Tier 1: Immediate (enables the grinder)
1. **SWE-bench** — native Rust, replaces Python scripts, closes learning loop
2. **Self-hosting** — already works via `plan run`

### Tier 2: Near-term (broadens signal diversity)
3. **MBPP / HumanEval** — simple to add, fast feedback, diverse language signal
4. **Chain monitor** — infrastructure largely exists in roko-chain
5. **Vulnerability detection** — high value, reuses existing gate infrastructure

### Tier 3: Medium-term (compound effects)
6. **SQL generation** — popular benchmark, clean evaluation
7. **Research synthesis** — already partially works via `roko research`
8. **Incident response** — high operational value

### Tier 4: Long-term (full multi-domain network)
9. **DeFi strategy** — requires chain deployment (Phase 2)
10. **Infrastructure as Code** — requires cloud provider integration
11. **Document understanding** — requires vision model routing

Each tier adds arenas that generate diverse training signal, making all previous arenas
perform better through cross-domain transfer.
