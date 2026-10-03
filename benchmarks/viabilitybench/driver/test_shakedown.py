"""The shakedown suite (3314): one test per live-run defect (D1-D7, D10), each a tiny one-task plan with low turn
caps, driving the real Roko binary through `run_roko` with `stub_provider.py` behind the metering proxy. Every
scenario is offline (a loopback stub): nothing here spends money.

`VB_REQUIRE_REAL_ROKO=1` fails the file at collection instead of skipping it when no binary is at `VB_TEST_ROKO_BIN`
(or `target/debug/roko`): the gate must not report a quiet green from a suite that never ran.

Expect red until the phase-1 fixes (11xx, 12xx) land; that is this file's point (module docstring of 3314's spec).

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_shakedown.py -q
"""

from __future__ import annotations

import json
import os
import re
from pathlib import Path

import pytest

import layout
import vb
from common import hmac_seed
from stub_provider import Blank, Chunks, ErrorStatus, StubServer
from test_run_roko import ARM, PIN, REAL_ROKO, TOY_STREAM

LADDER_ARM = layout.ARMS_DIR / "roko_ladder.toml"
MID = "glm-4.7"
FAST_CAPS = {  # low caps (3314's plan): well under a minute, but roomy enough for a gate rerun and helper calls
    "turns_per_attempt": "8", "input_tokens_per_attempt": "100000", "turns_per_task": "16",
    "input_tokens_per_task": "200000", "wallclock_s": "90", "model_calls_per_task": "24", "usd_per_task": "1.0",
    "max_output_tokens": "8192", "command_timeout_s": "60",
}
WRITE_OPS = (
    {"role": "assistant", "content": None, "tool_calls": [{"index": 0, "id": "call_1", "type": "function",
                                      "function": {"name": "write_file", "arguments": json.dumps(
                                          {"path": "calc/ops.py",
                                           "content": "def clamp(value, low, high):\n"
                                                      "    return max(low, min(value, high))\n"})}}]},
)
# The base already holds calc/ops.py, and Roko's implementer contract (RequireToolBeforeEdit) refuses a write to a
# file the attempt has not read: a scenario that must change it reads it first (bug-ef82eb).
READ_OPS = (
    {"role": "assistant", "content": None, "tool_calls": [{"index": 0, "id": "call_0", "type": "function",
                                      "function": {"name": "read_file",
                                                   "arguments": json.dumps({"path": "calc/ops.py"})}}]},
)
DONE = "Done."

if os.environ.get("VB_REQUIRE_REAL_ROKO") == "1" and not os.access(REAL_ROKO, os.X_OK):
    raise RuntimeError(f"VB_REQUIRE_REAL_ROKO=1 but no executable roko binary at {REAL_ROKO}: build target/debug/roko "
                       "or set VB_TEST_ROKO_BIN")
shakedown = pytest.mark.skipif(not os.access(REAL_ROKO, os.X_OK),
                               reason=f"no roko binary at {REAL_ROKO}; build it or set VB_TEST_ROKO_BIN")


@pytest.fixture
def places(tmp_path: Path) -> dict[str, Path]:
    secret = hmac_seed.write_secret_file(tmp_path / "private-config" / "secret")
    return {"results": tmp_path / "results", "work": tmp_path / "work", "secret": secret}


def fast_arm(tmp_path: Path, path: Path, binary: Path = REAL_ROKO, **caps: str) -> Path:
    """`path`'s arm with the real binary and low caps (module docstring), for a one-task shakedown plan."""
    text = path.read_text()
    old = 'binary = "target/debug/roko"'
    assert old in text
    text = text.replace(old, f"binary = {json.dumps(str(binary))}")
    for name, value in {**FAST_CAPS, **caps}.items():
        text, count = _replace_cap(text, name, value)
        assert count == 1, f"{path}: no [caps] {name} to replace"
    out = tmp_path / f"shakedown-{path.stem}.toml"
    out.write_text(text)
    return out


def _replace_cap(text: str, name: str, value: str) -> tuple[str, int]:
    return re.subn(rf"(?m)^{name} = \S+", f"{name} = {value}", text, count=1)


def run_vb(places: dict[str, Path], arm: Path, url: str, model: str = PIN, *extra: str) -> int:
    return vb.main(["run", "--experiment", "TEST-SHAKEDOWN", "--run-id", "run-1", "--stream", TOY_STREAM, "--arm",
                    str(arm), "--model", model, "--seeds", "1", "--limit", "1", "--provider-url", url, "--proxy",
                    "--results", str(places["results"]), "--work", str(places["work"]), "--secret-file",
                    str(places["secret"]), *extra])


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def run_dir(places: dict[str, Path]) -> Path:
    return places["results"] / "TEST-SHAKEDOWN" / "run-1"


def sequence(*replies: object):
    """A `respond` that plays `replies` in order by a global call count, repeating the last one: unlike
    `stub_provider.scripted`, which indexes by the current attempt's own turn count, so a retried attempt's first
    call would replay step 0. These scenarios need the model's call-by-call history across every attempt, since an
    attempt itself retrying (or not) is what each one tests."""
    calls = {"n": 0}

    def respond(body: dict) -> object:
        calls["n"] += 1
        return replies[min(calls["n"] - 1, len(replies) - 1)]

    return respond


@shakedown
def test_shakedown_d1_blank_answer_does_not_isolate_the_task(places, tmp_path):
    # G01/D1: a blank answer must not isolate the task for good. The first attempt gets an empty reply (no content,
    # no tool call); if that still ends the task's only attempt, the fix has not landed. A second attempt that
    # solves the task, reading calc/ops.py before it writes it (READ_OPS), must be allowed to run and complete.
    arm = fast_arm(tmp_path, ARM)
    respond = sequence(Blank(), *READ_OPS, *WRITE_OPS, DONE)
    with StubServer(respond) as stub:
        assert run_vb(places, arm, stub.url) == 0
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    assert len(record["execution"]["attempts"]) >= 2, "the task was isolated after the blank answer: only one attempt"
    assert record["execution"]["status"] == "completed", record["execution"]["reason"]


@shakedown
def test_shakedown_d2_stream_chunks_keep_every_field(places, tmp_path):
    # G02/D2: a tool call's name and arguments, streamed across separate chunks, must both survive. A parser that
    # keeps only one field per chunk ends up with no tool call (or a malformed one) and the file is never written.
    arm = fast_arm(tmp_path, ARM)
    split = Chunks(deltas=(
        {"role": "assistant", "content": None, "tool_calls": [{"index": 0, "id": "call_1", "type": "function",
                         "function": {"name": "write_file", "arguments": ""}}]},
        {"tool_calls": [{"index": 0, "function": {"arguments": '{"path": "calc/ops.py", "content": "def clamp('}}]},
        {"tool_calls": [{"index": 0, "function": {
            "arguments": 'value, low, high):\\n    return max(low, min(value, high))\\n"}'}}]},
    ), finish_reason="tool_calls")
    respond = sequence(split, DONE)
    with StubServer(respond) as stub:
        assert run_vb(places, arm, stub.url) == 0
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    first = record["execution"]["attempts"][0]
    assert first["checks"] == [], f"the split tool call was flagged, not just unsuccessful: {first['checks']}"
    assert record["execution"]["status"] != "infra_error", record["execution"]["reason"]


@shakedown
def test_shakedown_d3_provider_error_climbs_the_ladder(places, tmp_path):
    # G12/D3: a provider error on the cheap rung must climb the ladder (to the next rung up), not stay stuck on a
    # rung that never answers.
    arm = fast_arm(tmp_path, LADDER_ARM)

    def respond(body: dict) -> object:
        return ErrorStatus(500) if body.get("model") == PIN else DONE

    with StubServer(respond) as stub:
        assert run_vb(places, arm, stub.url) == 0
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    dispatched = [attempt["model_dispatched"] for attempt in record["execution"]["attempts"]]
    assert MID in dispatched, f"never escalated off the failing rung: dispatched {dispatched}"
    assert record["execution"]["status"] == "completed", record["execution"]["reason"]


@shakedown
def test_shakedown_d4_open_circuit_never_substitutes_outside_the_rungs(places, tmp_path):
    # G13/D4: a failover substitute's failure must not count against the rung that was routed. After the cheap
    # rung's one hiccup (task 1 escalates past it), a fresh task must still start on the cheap rung, not some
    # other model the first task's failure pushed it away from.
    arm = fast_arm(tmp_path, LADDER_ARM)
    calls = {"n": 0}

    def respond(body: dict) -> object:
        calls["n"] += 1
        if body.get("model") == PIN and calls["n"] == 1:
            return ErrorStatus(500)
        return DONE

    with StubServer(respond) as stub:
        assert run_vb(places, arm, stub.url, PIN, "--limit", "2") == 0
    records = {record["task"]["instance_id"]: record for record in read_jsonl(run_dir(places) / "records.jsonl")}
    assert len(records) == 2
    second = records["F1-l1-0002"]
    first_dispatched = second["execution"]["attempts"][0]["model_dispatched"]
    assert first_dispatched == PIN, f"the second task did not start on the cheap rung: {first_dispatched}"


@shakedown
def test_shakedown_d5_finished_long_answer_is_not_denied(places, tmp_path):
    # G05/D5: a long but legitimately finished answer must not be denied as if the stream cap had cut it off.
    arm = fast_arm(tmp_path, ARM)
    long_reasoning = Chunks(deltas=({"role": "assistant", "content": "I am analyzing the task. " * 120},
        {"content": None, "tool_calls": [{"index": 0, "id": "call_1", "type": "function",
                                          "function": WRITE_OPS[0]["tool_calls"][0]["function"]}]}),
        finish_reason="tool_calls")
    respond = sequence(long_reasoning, DONE)
    with StubServer(respond) as stub:
        assert run_vb(places, arm, stub.url) == 0
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    assert "aborted_cap" not in (record["execution"]["status"], record["execution"]["reason"]), record["execution"]


@shakedown
def test_shakedown_d6_openai_usage_reaches_roko_costs(places, tmp_path):
    # D6 (PK07 gap-f548c1, R4 #4a): the OpenAI-compatible backend must ask for usage on a streamed call
    # (`stream_options.include_usage`), and that usage must reach an attempt as a known (non-null) cost. The stub
    # reads calc/ops.py before it writes it (READ_OPS), so the task completes instead of ending gate-failed.
    arm = fast_arm(tmp_path, ARM)
    respond = sequence(*READ_OPS, *WRITE_OPS, DONE)
    with StubServer(respond) as stub:
        assert run_vb(places, arm, stub.url) == 0
        streamed = [request for request in stub.requests if request.get("stream")]
    assert streamed and all(request.get("stream_options", {}).get("include_usage") is True for request in streamed)
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    attempts = record["execution"]["attempts"]
    assert attempts and any(attempt["api_equiv_usd"] is not None for attempt in attempts), attempts


@shakedown
def test_shakedown_d7_auth_failure_keeps_healthy_circuits_closed(places, tmp_path):
    # G04/D7: an auth failure on one rung's provider must take just that provider out of the run, not open a
    # general circuit that also denies a healthy rung's calls.
    arm = fast_arm(tmp_path, LADDER_ARM)

    def respond(body: dict) -> object:
        return ErrorStatus(401, auth=True) if body.get("model") == PIN else DONE

    with StubServer(respond) as stub:
        assert run_vb(places, arm, stub.url) == 0
    [record] = read_jsonl(run_dir(places) / "records.jsonl")
    dispatched = [attempt["model_dispatched"] for attempt in record["execution"]["attempts"]]
    assert MID in dispatched, f"the healthy rung was never reached after the auth failure: dispatched {dispatched}"
    assert record["execution"]["status"] == "completed", record["execution"]["reason"]


@shakedown
def test_shakedown_d10_attempt_rows_carry_tokens_and_cost(places, tmp_path):
    # D10 (R4 #4b): S01's attempts.jsonl verdict rows must carry real token counts and an amount, not nulls.
    arm = fast_arm(tmp_path, ARM)
    respond = sequence(*WRITE_OPS, DONE)
    with StubServer(respond) as stub:
        assert run_vb(places, arm, stub.url) == 0
    s01 = run_dir(places) / "s01" / "F1-l1-0001.s1"
    rows = [row for path in sorted(s01.glob("runs/*/attempts.jsonl")) for row in read_jsonl(path)]
    verdicts = [row for row in rows if row.get("schema_version") == "roko.verdict/1"]  # the settled verdict, not
    # the "roko.attempt_open/1" row an attempt starts with, which carries no usage yet
    assert verdicts, "no S01 verdict rows were copied out of the workspace"
    for row in verdicts:
        usage = row.get("usage") or {}
        cost = row.get("cost") or {}
        assert any(usage.get(key) for key in ("tokens_in", "tokens_out")), f"verdict carries no tokens: {row}"
        assert cost.get("api_equiv_usd") is not None, f"verdict carries no amount: {row}"
