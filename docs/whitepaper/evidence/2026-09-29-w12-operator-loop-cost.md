# W12: Evidence from the development process itself
Agent W12 · 2026-09-29, revised after the author's answers:
- **The operator loop costs about 16–20× Roko's recorded spend.** Claude sessions from 09-25 to 09-29 come to about
  **$2.7–3.4k API-equivalent** at S08's prices. This is an estimate, and it includes the research programme. Roko
  recorded $172.80 over the same days.
### F2. Baselines (raw figures; estimates)

| Measure | Value | Caveat |
|---|---|---|
| Roko-labelled share of added lines since 09-01 | Rust: 13.7k of 459k (≈3%). Apps: 24.0k of 65.3k (≈37%) | Subject label only |
| Claude sessions (`cli`, `claude-desktop`), 09-25..29 | Opus 5.5: 22.9k calls, 12.2M output, 5.98B cache-read, 151M cache-write tokens. Fable 5.1: 1.2k calls. Other models: small | Includes research, paper, audits and this note |
| API-equivalent at S08 `prices-2026-09-28` | ≈ $2.7–3.4k (the range is 5-minute vs 1-hour cache-write rates) | Opus 5, Opus 4.6 and Sonnet 5 priced at Opus 5.5 rates; ambiguous Sonnet 4.6 rows excluded |
| Roko's child sessions (`sdk-cli`, Sonnet 4.6) | 3,208 calls, 2.29M output tokens | Already inside Roko's $172.80. A check on Roko's ledger, which has no cost source (B3) |
