# Prompt: 21-references

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/21-references/`. This topic is the **master citation index** — a consolidated reference list of every academic paper, specification, and research citation used anywhere in the Roko PRD documentation. Group by domain. Use academic format.

This topic runs LAST, ideally after all other topics exist (so you can scan them), but it can also run in parallel if the other topics aren't yet generated. Be defensive: generate the citation list from the source material directly rather than depending on other topics existing.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd (scan ALL for citations)

Use Grep to find citation patterns in every refactoring-prd file:

```bash
# Example searches:
grep -rn 'et al\.' /Users/will/dev/nunchi/roko/refactoring-prd/
grep -rnE '\(19|20[0-9]{2}\)' /Users/will/dev/nunchi/roko/refactoring-prd/
grep -rnE 'arXiv:[0-9]{4}\.[0-9]+' /Users/will/dev/nunchi/roko/refactoring-prd/
```

Then Read each refactoring-prd file (all 12) to catch citations the grep might miss.

## Step 3 — SOURCE-INDEX entry `## references.md`

Key legacy citation sources:
- `bardo-backup/prd/shared/citations.md`
- `bardo-backup/prd/shared/research.md`
- `bardo-backup/prd/02-mortality/14-research-foundations.md` — 130+ papers (preserve ALL, reframe for lifecycle not mortality)
- `bardo-backup/prd/02-mortality/15-references.md` — 162 citations (preserve ALL)
- `bardo-backup/prd/04-memory/10-research.md`
- `bardo-backup/prd/shared/hdc-vsa.md`
- `bardo-backup/tmp/hyperliquid/new/shared/citations.md`
- `bardo-backup/tmp/agent-chain/08-references.md`
- `bardo-backup/tmp/agent-chain/14-academic-foundations.md`
- `bardo-backup/tmp/mori-agents/12-references.md`

## Step 4 — implementation-plans

- `modelrouting/11-research-context.md` — 23 sections: RouteLLM, MixLLM, FrugalGPT, GVU, GEPA, SAGE, ABC, and many more
- `12a-cognitive-layer.md` §Source Documents table — 14 core references with full citations

## Step 5 — Scan all output topics (if they exist)

If output directories for other topics exist:

```bash
grep -rnE '\b(arXiv:|ICLR|NeurIPS|ACM|IEEE|Proc\.|et al\.)\b' /Users/will/dev/nunchi/roko/roko/docs/ 2>/dev/null
```

Extract citations the other agents included. Add any that aren't already in your list.

## Step 6 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/21-references
```

Write **24 sub-docs** (one per domain) plus `INDEX.md`:

| # | Filename | Domain |
|---|---|---|
| 00 | `00-lifecycle-and-finite-agency.md` | Reframed from "mortality." Ray 1991 Tierra, Lenski LTEE, Ebbinghaus 1885 forgetting curve, Hayflick 1961 (historical reference only), plus ~130 other mortality research citations REFRAMED for knowledge lifecycle. Note: the original source was "02-mortality/14-research-foundations.md" — keep ALL citations but reframe each brief annotation. |
| 01 | `01-memory-consolidation.md` | McClelland 1995 Complementary Learning Systems, Wilson & McNaughton 1994 (hippocampal replay), Mattar & Daw 2018 (prioritized memory access), Ha & Schmidhuber 2018 World Models, Hafner 2025 DreamerV3, Lin et al. 2025 sleep-time compute, WSCL 2024. |
| 02 | `02-affective-computing.md` | Mehrabian 1996 PAD (Current Psychology 14(4)), Damasio 1994 Descartes' Error, Bechara et al., Bower 1981 (mood congruence), Blaney 1986, Plutchik 1980, Russell-Mehrabian 1977, Scherer 2001 (appraisal), Walker & van der Helm 2009 (emotional depotentiation), OCC (Ortony Clore Collins 1988), Zhang et al. SIGDIAL. |
| 03 | `03-dreams-and-offline-learning.md` | Lacaux et al. 2021 Science Advances (83% N1 hidden rule discovery), Haar Horowitz et al. 2020/2023 MIT Dormio (43% creativity boost), Park 2023 Generative Agents (arXiv:2304.03442), Boden creativity modes, Pearl 2009 Causality SCM, Walker & van der Helm 2009, Derrida 1993 hauntology (Specters of Marx). |
| 04 | `04-coordination-and-multi-agent.md` | Grassé 1959 Insectes Sociaux 6(1), Theraulaz 1999 Artificial Life 5(2), Parunak et al. 2002 (digital pheromones), Dorigo 1997 IEEE Trans. Evol. Comp. 1(1) Ant Colony, Reed's Law, Metcalfe's Law, Woolley et al. 2010 Science 330(6004) C-Factor, Beer 1972 Brain of the Firm VSM, Conant & Ashby 1970 Good Regulator, Heylighen, Holland 1992. |
| 05 | `05-biological-analogues.md` | Odling-Smee et al. 2003 niche construction, Charnov 1976 MVT, Pirolli & Card 1999 information foraging, Kauffman autocatalytic sets, Yerkes-Dodson 1908. |
| 06 | `06-self-learning-systems.md` | Reflexion (Shinn et al. 2023, arXiv:2303.11366), ExpeL (Zhao et al.), DSPy (Khattab et al.), Voyager (Wang et al. 2023, arXiv:2305.16291), Meta-Harness (Lee et al. 2026, arXiv:2603.28052), EvoSkills 2026, ADAS (Hu et al., ICLR 2025), AgentPRM, Song et al. ICLR 2025 generation-verification gap, Lightman et al. "Let's Verify Step by Step," Sumers et al. 2023 CoALA (arXiv:2309.02427), Park 2023. |
| 07 | `07-context-engineering.md` | Liu et al. 2023 Lost in the Middle TACL (arXiv:2307.03172), Karpathy 2025 context engineering blog, ACE framework (Zhang et al. 2025), CSO Samsung, ACON Kang et al., Lewis et al. 2020 RAG (NeurIPS 2020, arXiv:2005.11401), RAGAS, ARES, Meta-Harness Lee et al. 2026. |
| 08 | `08-security-and-provenance.md` | CaMeL (Debenedetti et al.), OWASP Top 10, Constitutional AI (Anthropic), Cohen undecidability theorems, C2PA content credentials, W3C DIDs, ERC-8004, EU AI Act Art. 14 + FRIA, SEC/CFTC, HIPAA, SOX, GDPR, MiFID II. |
| 09 | `09-hdc-vsa.md` | **14+ HDC/VSA papers.** Kanerva 2009 (Cognitive Computation 1(2)), Plate 1994, Frady 2021 (resonator networks), Kleyko 2022 (ACM Computing Surveys 55(6)), Neubert 2022 (Proc. IEEE VSA survey), Johnson-Lindenstrauss 1984, Rahimi & Recht random features, and more. |
| 10 | `10-market-microstructure.md` | Kalman filter, Mallat wavelets, MIDAS anomaly detection, Ousterhout et al. 2013 power-of-two-choices, spectral methods for liquidity. |
| 11 | `11-streaming-algorithms.md` | HyperLogLog, Count-Min Sketch, Bloom filters. |
| 12 | `12-signal-processing.md` | Kalman, Mallat wavelets, spectral decomposition. |
| 13 | `13-philosophy.md` | Heidegger (Being and Time), Jonas (The Imperative of Responsibility), Camus (The Myth of Sisyphus), Derrida 1993 (Specters of Marx). |
| 14 | `14-agent-harnesses-and-tool-use.md` | Meta-Harness (Lee et al. 2026, arXiv:2603.28052), FrugalGPT (Chen et al. 2023, arXiv:2305.05176), SWE-bench, RouteLLM, MixLLM, GVU, GEPA, SAGE, ABC. |
| 15 | `15-cybernetics-and-vsm.md` | Ashby's Law of Requisite Variety, Ashby 1956 Introduction to Cybernetics, Beer 1972 Brain of the Firm, Beer Viable System Model, Conant & Ashby 1970 Good Regulator Theorem, Wiener 1948 Cybernetics, Boyd OODA. |
| 16 | `16-active-inference.md` | Friston Free Energy Principle, Parr et al. 2024 (arXiv:2402.14460), Koudahl et al. 2024 (arXiv:2412.10425), VERSES AI Genius, pymdp library. |
| 17 | `17-process-reward-models.md` | Lightman et al. "Let's Verify Step by Step" (OpenAI), AgentPRM, Song et al. ICLR 2025 generation-verification gap. |
| 18 | `18-collective-intelligence.md` | Woolley et al. 2010 Science 330(6004) C-Factor, Metcalfe's Law, Reed's Law. |
| 19 | `19-regulatory-compliance.md` | EU AI Act Art. 14 (human oversight) + FRIA, SEC/CFTC trading reconstruction, HIPAA audit trails, SOX financial controls, GDPR purpose limitation, C2PA, MiFID II. |
| 20 | `20-cognitive-architectures.md` | CoALA (Sumers 2023 arXiv:2309.02427), ACT-R (Anderson), SOAR (Laird), CLARION dual-level (Sun et al.), Kahneman System 1/2 (Thinking Fast and Slow 2011). |
| 21 | `21-mechanism-design.md` | Vickrey 1961, Clarke 1971, Groves 1973 VCG, FPSB, Dutch auction, Vickrey auction, Glicko-2 rating. |
| 22 | `22-protocol-standards.md` | ERC-8004 (agent identity), ERC-721 (NFT), ERC-3009 (signed transfers), ERC-4337 (account abstraction), x402 (Coinbase/Linux Foundation), ERC-20. |
| 23 | `23-generational-and-evolutionary.md` | Ray 1991 Tierra (evolutionary CS), Lenski Long-Term Evolution Experiment, Holland genetic algorithms. Reframed for Roko: evolutionary computing inspires skill evolution (EvoSkills), not biological mortality. |

Plus `INDEX.md` that lists all 24 sub-docs, gives a brief description of each domain, and provides a master citation count and methodology note.

## Step 7 — Writing rules (critical for references doc)

- Each sub-doc must list **every** citation in that domain, in standard academic format.
- For each citation, include: authors, year, title, venue (journal/conference/arXiv ID if available), and a 1-2 sentence annotation explaining which Roko subsystem it grounds.
- Use consistent format:
  ```
  - Author1, Author2, & Author3 (YYYY). Title. _Venue_, volume(issue), pages. arXiv:ID. [DOI](url)
    *Grounds: <which Roko subsystem> — <what concept the paper contributes>*
  ```
- No duplicates across sub-docs. If a paper spans domains, put it in the primary domain and cross-reference.
- Preserve the original author ordering from sources.
- For papers you know about from memory (e.g., CoALA arXiv ID), use that information. For papers you don't have precise bibliographic data on, use whatever partial information the sources provide.

## Step 8 — INDEX.md

Master citation index. Lists all 24 domain sub-docs. Gives a citation count per domain. Provides methodology note (how citations were sourced, how duplicates were handled). Cross-references to topics that use each domain's citations most heavily.

## Step 9 — Self-check

- [ ] 24 domain sub-docs + INDEX.md
- [ ] Each sub-doc has at least the citations listed in Step 6 above (plus any additional ones you find)
- [ ] Every citation in standard academic format with annotation
- [ ] No duplicates across sub-docs
- [ ] Each mortality-era citation is reframed to be about knowledge lifecycle, not agent mortality
- [ ] INDEX.md gives counts per domain and methodology note

## CRITICAL REMINDERS

- **Preserve ALL 200+ citations from all sources.** This is the canonical reference list.
- Use standard academic format for every citation.
- Reframe mortality-era citations for lifecycle — they're still valid, just differently contextualized.
- 14+ HDC/VSA citations minimum.
- Don't ask questions. If a citation has incomplete data, include what you have.
- Use Write tool. Absolute paths.
