# Invalid model config continues with ambiguous fallback

- Severity: medium

The run repeatedly warns that `agent.default_model` references missing `claude-sonnet-4-6`, while also warning that the same slug is duplicated under two model keys. Dispatch continues and selects `claude-sonnet-4-5` for several tasks.

Validation neither fails fast nor clearly records the fallback decision. Normalize aliases during load, reject duplicate slugs, and make unresolved defaults fatal before agent dispatch.

