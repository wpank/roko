# Carry-Forward Map — PU03 Audit Refresh

Use this when an item appears during PU03 work but belongs in a later pass.

| Item | Better Home | Keep In PU03 As | Why |
|------|-------------|-----------------|-----|
| full active-inference / EFE learning policy | `tmp/docs-parity/05` | naming note + seam check | PU03 only needs honest scorer/runtime contracts |
| complexity-policy redesign beyond one wired path | `tmp/docs-parity/05` | one-path handoff | wider routing and learning policy is not a 90-minute audit task |
| broad enrichment redesign | later composition hardening | runtime-seam note | PU03 only needs to describe the existing enrichment seam honestly |
| deeper HDC dedup or retrieval-policy redesign | later context hardening | code-anchor note | current audit centers on the shipped `ContextProvider` path |
| truthful VCG / fairness floors / mechanism design | research or `05` | theory deferral | not required to prove the composition contract |
| calibrated MVT patch modeling | `tmp/docs-parity/05` | theory deferral | routing economics, not core prompt wiring |
| distributed context engineering / agent mesh | post-parity roadmap | roadmap note | outside the single-process runtime path under audit |
| eval harnesses such as RAGAS / CLEAR / CIV / Meta-Harness | post-parity eval pass | eval handoff | depends on stable runtime behavior first |

When deferring, record:

1. the exact file, symbol, or gap id,
2. the runtime path you checked,
3. why the item is outside PU03,
4. the next owning batch or pass,
5. the smallest contract PU03 still leaves behind.
