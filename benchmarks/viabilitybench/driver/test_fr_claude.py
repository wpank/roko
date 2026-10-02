"""Offline test of the fr_claude arm (3318): the Roko arm on claude-opus-5-5 through the Claude CLI, with no
metering proxy in front of it. A fake `roko` leaves the records a real Claude-CLI-dispatched session would (S01's
verdict carrying its own `modelUsage` and `total_cost_usd`), and the attempt is priced from them: U' (tokens x the
snapshot, source `cli_usage`) and R, the CLI's own figure, kept as `vendor_usd`.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_fr_claude.py -q
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

import layout
import validate
import vb
from common import hmac_seed
from stub_provider import StubServer

TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")
ARM = layout.ARMS_DIR / "fr_claude.toml"
PIN = "claude-opus-5-5"
# A stand-in for roko dispatching the Claude CLI (3318): it makes no network call itself (the fake never spawns
# claude; `run_roko`'s egress proxy and sandbox are exercised regardless, since they wrap every roko process), and
# leaves the records a real session would: one passed attempt, and S01's verdict carrying its own `modelUsage` and
# `total_cost_usd` instead of API-style per-call usage.
FAKE_ROKO = r'''#!__PYTHON__
import datetime, json, sys
from pathlib import Path

args = sys.argv[1:]
if args == ["--version"]:
    print("roko 0.1.0 (git 0fa4e0fa4e)")
    sys.exit(0)
if "validate" in args:
    sys.exit(0)
repo, slug = Path(args[args.index("--repo") + 1]), Path(args[args.index("run") + 1]).name
(repo / "calc" / "ops.py").write_text("def clamp(value, low, high):\n    return max(low, min(value, high))\n")
roko, now = repo / ".roko", datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")
run_id = "graph-%s-1" % slug
key = "%s:%s:T01:1" % (run_id, slug)
(roko / "state" / "graph" / slug).mkdir(parents=True)
(roko / "episodes.jsonl").write_text(json.dumps({
    "task_id": "T01", "model": "claude-opus-5-5", "backend": "anthropic", "success": True, "turns": 3,
    "completed_at": now, "extra": {"plan_id": slug, "attempt_key": key, "outcome": "passed"}}) + "\n")
(roko / "state" / "graph" / slug / "checkpoint.json").write_text(json.dumps(
    {"plan_id": slug, "status": "succeeded",
     "extensions": {"roko.gate.verdict@1": {"value": {"verdicts": {"T01": "passed"}}}}}))
verdict = {"schema_version": "roko.verdict/1", "plan_id": slug, "task_id": "T01", "attempt": 1, "attempt_key": key,
          "outcome": "passed",
          "executed": {"provider": "anthropic", "model_requested": "claude-opus-5-5",
                       "model_dispatched": "claude-opus-5-5", "model_reported": "claude-opus-5-5",
                       "models_reported": [], "model_mismatch": False, "failover_chain": [],
                       "failover_reason": None, "turns": 3,
                       "modelUsage": {"claude-opus-5-5": {"inputTokens": 4000, "outputTokens": 300,
                                                          "cacheReadInputTokens": 0,
                                                          "cacheCreationInputTokens": 0}},
                       "total_cost_usd": 0.5}}
(roko / "runs" / run_id).mkdir(parents=True)
(roko / "runs" / run_id / "attempts.jsonl").write_text(json.dumps(verdict) + "\n")
sys.exit(0)
'''


@pytest.fixture
def places(tmp_path: Path) -> dict[str, Path]:
    secret = hmac_seed.write_secret_file(tmp_path / "private-config" / "secret")
    return {"results": tmp_path / "results", "work": tmp_path / "work", "secret": secret}


def arm_with(tmp_path: Path, binary: Path) -> Path:
    text = ARM.read_text()
    old = 'binary = "target/debug/roko"'
    assert old in text
    arm = tmp_path / "fr_claude_test.toml"
    arm.write_text(text.replace(old, f"binary = {json.dumps(str(binary))}"))
    return arm


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def test_fr_claude_arm_prices_cli_usage_from_verdicts(places, tmp_path):
    binary = tmp_path / "bin" / "roko"
    binary.parent.mkdir()
    binary.write_text(FAKE_ROKO.replace("__PYTHON__", sys.executable))
    binary.chmod(0o755)
    arm = arm_with(tmp_path, binary)
    with StubServer(lambda body: "Done.") as stub:  # unused: the fake roko never calls it
        assert vb.main(["run", "--experiment", "TEST-FR-CLAUDE", "--run-id", "run-1", "--stream", TOY_STREAM,
                        "--arm", str(arm), "--model", PIN, "--seeds", "1", "--limit", "1", "--provider-url",
                        stub.url, "--results", str(places["results"]), "--work", str(places["work"]),
                        "--secret-file", str(places["secret"])]) == 0
        assert stub.requests == []  # no metering proxy, no direct client call either: Roko's own CLI child would
    out = places["results"] / "TEST-FR-CLAUDE" / "run-1"
    [record] = read_jsonl(out / "records.jsonl")
    assert validate.validate("run-record", record) == []
    assert record["arm"] == "fr_claude" and record["execution"]["status"] == "completed"
    [attempt] = record["execution"]["attempts"]
    assert attempt["checks"] == [] and attempt["model_dispatched"] == PIN and attempt["model_reported"] == PIN
    snapshot_row = vb.ledger.load_snapshot().row(PIN)
    expected = vb.ledger.price({"tokens_in": 4000, "tokens_out": 300, "tokens_cache_read": 0,
                               "tokens_cache_write_1h": 0, "tokens_reasoning": 0}, snapshot_row)
    assert attempt["api_equiv_usd"] == pytest.approx(expected.api_equiv_usd) and expected.api_equiv_usd > 0
    assert attempt["vendor_usd"] == pytest.approx(0.5)
    assert record["costs"]["source"] == "cli_usage"
    assert record["costs"]["api_equiv_usd"] == pytest.approx(expected.api_equiv_usd)
    assert record["costs"]["vendor_usd"] == pytest.approx(0.5)
    assert record["costs"]["billed_usd"] == 0.0  # the subscription: nothing billed, U' still recorded
    rows = read_jsonl(out / "ledger.jsonl")
    assert len(rows) == 1 and validate.validate("ledger", rows[0]) == [] and rows[0]["api_equiv_usd"] > 0
    # Roko's claude children ran under this task's own egress proxy (3305, 3318), not a loopback-only sandbox.
    egress = record["provenance"]["network_policy"]["egress"]
    assert egress == {"allow": ["api.anthropic.com:443"], "admitted": 0, "refused": []}
