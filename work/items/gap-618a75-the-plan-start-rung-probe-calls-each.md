+++
id = "gap-618a75"
kind = "gap"
title = "The plan-start rung probe calls each rung's model the way an attempt does, streaming and with the attempt's tools, so it catches a streaming-path failure before the run"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-46"
anchors = ["crates/roko-cli/src/dispatch/rung_probe.rs::run_probe", "crates/roko-cli/src/dispatch/rung_probe.rs::probe_rung", "crates/roko-cli/src/dispatch/rung_probe.rs::probe_ladder"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["q-85053f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn rung_probe_streams_like_an_attempt' crates/roko-cli/ && cargo test -p roko-cli rung_probe_streams_like_an_attempt"
+++

## Problem

The rung probe (task 1121) checks each ladder rung once at plan start, cached for 24 h, so that a rung whose model
can't run is skipped before any attempt is spent on it. It calls `agent.run(&input, &ctx)`
(`crates/roko-cli/src/dispatch/rung_probe.rs::run_probe`, `:202-210`), the non-streaming path, with one echo tool.
Attempts stream (`stream: true`) and get 8-14 tools. In agent H's live run (2026-10-15, HEAD 7b6a5af44), all 8 probe
calls had `stream` unset and 0 SSE events on the wire, while every attempt call streamed (H, finding N2).

R3's GLM failure (2026-10-02, defect D2) was in roko's SSE parser: GLM sends each tool call in a chunk with
`content: ""`, which the old parser turned into blank turns. A non-streaming probe would most likely have passed GLM
then, and the run would have spent its attempts on a rung that could not work.

## Why it matters

Goal `core`. The probe exists to keep a broken rung from eating a run's attempts and budget. It can only catch
failures on the path it exercises, so a streaming-parser or streamed tool-call regression in a provider adapter gets
through.

## Where

- `crates/roko-cli/src/dispatch/rung_probe.rs`: `probe_ladder` (`:107`), `probe_rung` (`:164`), `run_probe` (`:202`),
  `echo_tool` (`:233`); results cached in `.roko/learn/rung-probes.json`.

## Current state

The probe checks that the provider answers and that the model calls a tool. D2 itself is fixed (`9dd006133`, 1110),
so this is about the next streaming failure, not a live one.

## Plan

1. Drive the probe through the same entry point an attempt uses for that provider (the streaming call, as the Graph
   dispatch path does), consuming the stream to the end and requiring a parsed tool call and a final answer.
2. Keep the probe cheap: one short prompt and one tool. Whether to offer the attempt's full tool list is a cost
   trade-off (tool schemas are most of the probe's input tokens); one tool through the streaming parser is the
   minimum that would have caught D2.
3. Test `rung_probe_streams_like_an_attempt`: against a scripted OpenAI-compatible SSE server that sends a tool call
   in GLM's chunk shape, the probe uses the streaming request and passes; one that streams only blank deltas fails it.

## Done when

`rung_probe_streams_like_an_attempt` passes (the `[[verify]]` command).

## Notes

Evidence: `tmp/status-quo/2026-10-15/live/H-live-run.md` "Findings beyond D1-D16", row N2 (`:347`). Probe spend is a separate matter: it appears in no
cost record (N3), and decision 3 (2026-10-15) keeps it charged to attempt 1 (q-85053f).
