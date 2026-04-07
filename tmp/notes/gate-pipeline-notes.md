# Gate Pipeline Analysis

## Current 7-rung pipeline
1. Format check (clippy)
2. Compile gate
3. Test gate
4. Diff review
5. Safety scan
6. Coverage gate
7. Integration gate

## Observations
- Rungs 1-3 are fast (<10s each)
- Rung 4 (diff review) is the bottleneck — requires LLM call
- Rungs 5-7 rarely fail after 1-4 pass
- Adaptive thresholds (EMA) reduce false positives by ~40%

## Ideas
- Run rungs 1-3 in parallel
- Cache diff review for unchanged files
- Skip rungs 5-7 for "trivial" changes (docs, comments)
