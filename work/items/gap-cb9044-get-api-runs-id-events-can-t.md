+++
id = "gap-cb9044"
kind = "gap"
title = "GET /api/runs/{id}/events can't skip a run's live words: 96% of a real run's index is agent_output reasoning and text deltas, so a cold read of a phase's tool steps and events takes about 700 requests and 27 s"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-plugin-wow#item-8"
discovered_from = "agent:plugin-wow (round v7, 2026-10-19)"
anchors = ["crates/roko-serve/src/routes/runs.rs::EventQuery", "crates/roko-serve/src/routes/runs.rs::get_run_events", "crates/roko-serve/src/routes/runs.rs::STREAM_EVENT_TYPES"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-e529e3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_run_events_page_can_skip_live_words' crates/roko-serve/ && cargo test -p roko-serve --lib -- a_run_events_page_can_skip_live_words"
+++

## Problem

`GET /api/runs/{id}/events` can't leave out a run's live words. With trusted live output every reasoning and text
delta is one `agent_output` record, so a real run's index is almost all words that no page shows.

The pilot's phase 0 run of round v7 (`~/dev/undertow/.roko/events-by-run/71240a0c…`, 3 tasks, 56 min) has 34,919
records in 15.3 MB:

| Records | Rows | Bytes |
|---|---|---|
| `agent_output`, envelope `reasoning` | 31,752 | 13.9 MB |
| `agent_output`, envelope `text` with `live: true` | 1,669 | 0.73 MB |
| `agent_heartbeat` | 607 | 0.13 MB |
| `agent_output`, envelope `tool_start` / `tool_result` | 414 / 209 | 0.50 MB |
| `gate_output_line` | 124 | 35 KB |
| everything else (attempts, gates, routes, lessons, plan events) | about 150 | about 50 KB |

The route pages forward, at most 200 records a request, and serve allows 30 requests a second. plugin-wow measured
a cold read of that run at 175 requests and 6.0 s, and a stand-in for a whole phase (the index four times) at about
700 requests and 27 s (`$V7/findings/plugin-wow.md`, item 8).

`?types=` doesn't help. It is an allowlist of record types, and the tool steps a page does show are `agent_output`
records too, next to the words.

## Why it matters

A page that opens a finished or long run reads its history cold: the Hermes plugin's Activity and stream, the web
dashboard and the MCP tools. For most of those 27 s, the page says "reading". A multi-phase build like Undertow
makes it worse with every phase.

The summary route already skips these records unparsed (bug-e529e3, `STREAM_EVENT_TYPES`). The events route has no
equivalent.

## Where

- `crates/roko-serve/src/routes/runs.rs`:
  - `EventQuery`, `get_run_events` and `run_events_sse` (the SSE replay takes the same query);
  - `read_for_run` / `read_index_page_scan`, whose `skip` line predicate drops a record before it is parsed;
  - `STREAM_EVENT_TYPES` / `STREAM_TYPE_FIELDS`.

## Plan

1. Add `skip=` to `EventQuery`. It takes a comma list from a fixed vocabulary, and anything else is a 400:
   - `live_text`: `agent_output` records whose `roko.stream.v1` envelope is `reasoning`, or `text` with `live: true`;
   - `heartbeats`: `agent_heartbeat` and `plan_check_heartbeat`.
2. Skipped records are dropped by a line test, unparsed, as the summary does. A page of 200 kept records then scans
   up to `MAX_PAGE_SCAN_BYTES` (4 MiB). That reads the run above in about 4 requests and a phase in about 15.
3. Keep everything else reachable:
   - tool steps (`tool_start`, `tool_result`);
   - the final, non-live text;
   - `gate_output_line` (the build glass reads it);
   - every structural event.

   The line test must not drop a tool step whose output quotes the envelope's words. The envelope sits escaped inside
   `content`, so match its escaped form, and test it with such a tool result.
4. Cursors stay byte offsets, so a client can page with and without `skip` over the same index. The page says what
   it skipped (`"skipped": ["live_text"]`).

## Done when

A page with `?skip=live_text,heartbeats` holds the run's tool steps, gate lines and structural events, and none of
its live words. Paging with `next_cursor` crosses runs of skipped records, and a tool result that quotes the
envelope's words is kept. Checked by the test in `[[verify]]`.

## Notes

- Found by plugin-wow (round v7, item 8) on the pilot's real phase 0 index. Filed by roko-speed at the lead's
  request (2026-10-19). Owner: roko-speed, after gap-6a319c, gap-5c3347 and gap-e02ec4.
- Related: bug-e529e3 (the summary's skip).
