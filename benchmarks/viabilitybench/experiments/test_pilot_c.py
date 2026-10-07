"""Pilot C's manifest (`pilot_c.toml`, 3313): its dry run against BL14, and an offline rehearsal of its one block
through `vb campaign`, with a fake `roko` that escalates from the cheap rung to the mid rung and the stub provider
on loopback. Nothing is billed and no network is used.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_pilot_c.py -q
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path

import campaign
import layout
import validate
import vb
from stub_provider import StubServer

PILOT_C = layout.VB_ROOT / "experiments" / "pilot_c.toml"
# A stand-in for roko's `plan run` on the ladder (3312, 3313): attempt 1 dispatches the cheap rung and fails its
# gate; attempt 2 escalates to the next rung (mid) and passes. Each attempt calls its own rung's provider.
ESCALATING_ROKO = r'''#!__PYTHON__
import datetime, json, os, sys, tomllib, urllib.request
from pathlib import Path

args = sys.argv[1:]
if args == ["--version"]:
    print("roko 0.1.0 (git 0fa4e0fa4e)")
    sys.exit(0)
if "validate" in args:
    sys.exit(0)
repo, slug = Path(args[args.index("--repo") + 1]), Path(args[args.index("run") + 1]).name
config = tomllib.loads(Path(os.environ["ROKO_CONFIG"]).read_text())
models = list(config["models"])
cheap, mid = models[0], models[1]
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def call(model):
    provider_name = config["models"][model]["provider"]
    table = config["providers"][provider_name]
    request = urllib.request.Request(table["base_url"] + "/chat/completions", data=json.dumps(
        {"model": model, "messages": [{"role": "user", "content": "Implement it."}]}).encode(),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + os.environ[table["api_key_env"]]})
    opener.open(request, timeout=30).read()
    return config["models"][model]["provider"], datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


provider_1, completed_1 = call(cheap)
provider_2, completed_2 = call(mid)
roko = repo / ".roko"
(roko / "state" / "graph" / slug).mkdir(parents=True)
key_base = "graph-%s-1:%s:T01" % (slug, slug)
episodes = [
    {"task_id": "T01", "model": cheap, "backend": provider_1, "success": False, "turns": 1,
     "completed_at": completed_1, "failure_reason": "verify: 1/1 verify step(s) failed",
     "extra": {"plan_id": slug, "attempt_key": key_base + ":1"}},
    {"task_id": "T01", "model": mid, "backend": provider_2, "success": True, "turns": 1,
     "completed_at": completed_2, "failure_reason": None, "extra": {"plan_id": slug, "attempt_key": key_base + ":2"}},
]
(roko / "episodes.jsonl").write_text("".join(json.dumps(e) + "\n" for e in episodes))
(roko / "state" / "graph" / slug / "checkpoint.json").write_text(json.dumps(
    {"plan_id": slug, "status": "succeeded",
     "extensions": {"roko.gate.verdict@1": {"value": {"verdicts": {"T01": "passed"}}}}}))
sys.exit(0)
'''


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def test_pilot_c_dry_run_fits_bl14(capsys):
    assert vb.main(["campaign", "--manifest", str(PILOT_C), "--dry-run", "--results", "/tmp/vb-pilot-c-dry-run",
                    "--secret-file", "/tmp/vb-pilot-c-dry-run-no-secret"]) == 0
    shown = json.loads(capsys.readouterr().out)
    [block] = shown["blocks"]
    assert block["block"] == "roko-ladder" and block["line"] == "BL14" and block["runs"] == 60
    assert shown["ok"] and shown["budget"]["line BL14"]["cap_usd"] == 6.0


def test_pilot_c_manifest_rehearses_offline(tmp_path):
    """The manifest's one block runs offline through `vb campaign` with a fake roko that escalates: a schema-valid
    record per run, each with an escalate attempt, and every call on the loopback stub."""
    results, work = tmp_path / "results", tmp_path / "work"
    secret_file = vb.secret.create(tmp_path / "config" / "secret")
    binary = tmp_path / "bin" / "roko"
    binary.parent.mkdir()
    binary.write_text(ESCALATING_ROKO.replace("__PYTHON__", sys.executable))
    binary.chmod(0o755)
    text = (layout.ARMS_DIR / "roko_ladder.toml").read_text()
    old = 'binary = "target/debug/roko"'
    assert old in text
    arm = tmp_path / "roko_ladder.rehearsal.toml"
    arm.write_text(text.replace(old, f"binary = {json.dumps(str(binary))}"))
    with StubServer(lambda body: "Done.") as stub:
        assert vb.main(["campaign", "--manifest", str(PILOT_C), "--provider-url", stub.url, "--results",
                        str(results), "--work", str(work), "--secret-file", str(secret_file), "--limit", "1",
                        "--arm-file", f"roko_ladder={arm}"]) == 0
        assert {request["model"] for request in stub.requests} == {"gpt-oss-120b", "glm-4.7"}
    out = results / "PILOT-C"
    finished = [event for event in read_jsonl(out / campaign.LOG) if event["event"] == "finish"]
    assert sorted(event["unit"] for event in finished) == ["roko-ladder-s1", "roko-ladder-s2", "roko-ladder-s3"]
    assert all(event["exit"] == 0 for event in finished)
    records = [record for event in finished for record in read_jsonl(out / event["run_id"] / "records.jsonl")]
    rows = [row for event in finished for row in read_jsonl(out / event["run_id"] / "ledger.jsonl")]
    assert len(records) == 3 and all(validate.validate("run-record", record) == [] for record in records)
    assert Counter(record["arm"] for record in records) == {"roko_ladder": 3}
    assert {record["execution"]["status"] for record in records} == {"completed"}
    for record in records:
        classes = [attempt["cost_class"] for attempt in record["execution"]["attempts"]]
        assert classes == ["execute", "escalate"]
        dispatched = [attempt["model_dispatched"] for attempt in record["execution"]["attempts"]]
        assert dispatched == ["gpt-oss-120b", "glm-4.7"]
        assert record["execution"]["attempts"][1]["checks"] == []  # the escalation is not a mismatch
        assert record["costs"]["by_class"]["escalate"] > 0 and record["costs"]["by_class"]["execute"] > 0
    assert len(rows) == 6 and all(validate.validate("ledger", row) == [] for row in rows)  # two attempts per run
    assert Counter(row["line"] for row in rows) == {"BL14": 6}
