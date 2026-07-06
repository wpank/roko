# 22 — Research API Backlog (Not Yet Spec'd)

> **Status**: Backlog — spec when Perplexity (20) and Gemini (21) are wired

These APIs complement the Perplexity and Gemini integrations. Each fills a distinct gap in the research pipeline.

## Tier 1 — Spec Next

### Semantic Scholar (free, academic papers)
- **API**: `https://api.semanticscholar.org/graph/v1/`
- **Cost**: Free (1 RPS authenticated, shared pool unauthenticated)
- **What**: 200M papers, citation graphs, SPECTER2 embeddings, recommendations
- **Why**: `RESEARCH_SYSTEM_PROMPT` asks agents to cite papers from arXiv/ACL/NeurIPS/ICML. This API verifies citations exist, finds real papers, returns structured metadata (authors, abstract, venue, year, PDF URLs, citation count). Eliminates hallucinated citations.
- **Roko use**: `roko research topic` → query papers, verify `[AUTHOR-YEAR]` citations, get recommendation for "papers similar to X"

### Exa (neural/semantic search)
- **API**: `https://api.exa.ai/`
- **Cost**: $5/1K (Instant), $7/1K (with contents), $12/1K (deep), 1K free/month
- **What**: Embeddings-based web search — finds pages by meaning, not keywords. Returns full page content + highlights. Sub-200ms latency. 1200-domain filtering.
- **Why**: Perplexity generates synthesized answers; Exa returns raw sources for the agent to synthesize itself. Better for discovering similar code patterns, crates, and documentation.
- **Roko use**: find similar implementations, discover relevant crates/libraries, semantic search over docs

### Jina Reader (URL → markdown, free)
- **API**: Prepend `https://r.jina.ai/` to any URL; search via `https://s.jina.ai/?q=`
- **Cost**: Free (20 RPM unauthenticated, 200 RPM with free key)
- **What**: Converts any URL to clean LLM-friendly markdown. Strips nav, ads, boilerplate. Structured JSON extraction via `x-json-schema` header.
- **Why**: Critical glue between search APIs and comprehension. When Perplexity/Exa/Semantic Scholar return URLs, the agent needs to *read* the content. Jina converts any page to ingestible markdown.
- **Roko use**: ingest content behind citations, read docs.rs pages, convert GitHub READMEs

## Tier 2 — Consider Later

### Brave Search API
- **API**: `https://api.search.brave.com/`
- **Cost**: $5/1K requests, 1K free/month
- **What**: Independent 35B+ page index (not a Google wrapper). `llm/context` endpoint returns pre-compacted web context. Code context extraction for technical queries.
- **Overlap**: Similar to Perplexity Search API. Worth having as fallback/diversity source.

### Firecrawl
- **API**: `https://api.firecrawl.dev/v1/`
- **Cost**: 500 free lifetime credits, then ~$0.005/page
- **What**: Web scraping/crawling → markdown/JSON. `/extract` with structured schema. `/crawl` for entire sites with depth control.
- **Overlap**: Jina covers single pages. Firecrawl better for crawling entire docs sites (e.g., all of a crate's docs.rs).

### Tavily
- **API**: `https://api.tavily.com/`
- **Cost**: $0.008/request, 1K free/month
- **What**: Search built for AI agents (LangChain/LlamaIndex ecosystem). Returns extracted content, not just URLs.
- **Overlap**: Perplexity + Exa covers this. Acquired by Nebius (Feb 2026), future unclear.

## Complete Research Stack (Target)

```
Research query
    │
    ├── Semantic Scholar ─── find papers (free, structured)
    │     └── paper IDs, authors, abstracts, citations, PDF URLs
    │
    ├── Exa ─── find similar code/docs (semantic search)
    │     └── full page content, highlights
    │
    ├── Perplexity ─── synthesized answers with citations (spec'd: doc 20)
    │     └── grounded responses, search_results
    │
    ├── Gemini grounding ─── Google Search citations (spec'd: doc 21)
    │     └── groundingChunks, groundingSupports
    │
    ├── Jina Reader ─── read any URL found above (free)
    │     └── URL → clean markdown for context injection
    │
    └── Brave/Firecrawl ─── fallback search / deep crawling
```
