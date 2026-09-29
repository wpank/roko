+++
id = "find-1f0fbb"
kind = "finding"
title = "[provider F079] HDC episode similarity search is O(n) brute force per dispatch"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/hdc_fingerprint"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F079"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F079"
anchors = ["crates/roko-learn/src/hdc_fingerprint.rs", "crates/roko-learn/src/episode_logger.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`query_similar_episodes()` reads all episodes from the JSONL file, decodes every HDC fingerprint (10,240-bit base64), and computes pairwise cosine similarity. No index structure is used. Search time grows linearly with episode log size.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F079`
- `tmp/archive/provider-audit/17-learning-loop.md`

How to verify: Confirm in crates/roko-learn/src/hdc_fingerprint.rs, crates/roko-learn/src/episode_logger.rs whether still true: HDC episode similarity search is O(n) brute force per dispatch
