# Secret scrubber corrupts ordinary identifiers

- Severity: medium
- Area: observability / redaction
- Reproduced: 2026-07-11, headless gate output

Headless output rendered `task-verify:SH01-T02:test` as `ta[REDACTED:API_KEY]:SH01-T02:test`. The built-in expression `sk-[A-Za-z0-9-]+` matches the internal `sk-verify` substring in the ordinary identifier `task-verify`; it has neither a token boundary nor a credible minimum secret length. `RedactingFormat` applies it to the fully formatted tracing line, so prose, paths, and stable event names can all be corrupted.

Durable JSONL retained the original identifier, making terminal and durable views disagree. Tighten the secret pattern around real key shape and boundaries, replace the test that requires `sk-short` to be scrubbed, and add negative cases for `task-verify`, `risk-aware`, and similar prose.
