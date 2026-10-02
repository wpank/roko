"""Pilot A's manifest (`pilot_a.toml`, 3307): its dry run, and an offline rehearsal of all 65 of its runs on the pilot's
real F1 and F4 instances, through `vb campaign` and the stub provider on loopback. Nothing is billed and no network is
used: the paid run (gap-c33709) is the same command with --allow-network and the key file.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_pilot_a.py -q
"""

from __future__ import annotations

import json
from collections import Counter
from pathlib import Path

import campaign
import layout
import ledger
import validate
import vb
from common import hmac_seed
from stub_provider import StubServer, bash

PILOT_A = layout.VB_ROOT / "experiments" / "pilot_a.toml"


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def test_pilot_a_fd_api_stream_is_its_recorded_draw():
    draw = hmac_seed.surface_stream("stream", "pilot_fd_api/draw/1")
    first = draw.choice(["F1", "F4"])
    families = [first, "F4" if first == "F1" else "F1"]
    drawn = [draw.child(f"l{level}").choice([f"{families[(level - 1) % 2]}-l{level}-000{n}" for n in (1, 2)])
             for level in range(1, 6)]
    stream = vb.load_stream("pilot_fd_api")
    assert stream.instances == drawn and set(drawn) <= set(vb.load_stream("pilot").instances)
    assert [instance[:2] for instance in drawn] == ["F1", "F4", "F1", "F4", "F1"]


def test_pilot_a_dry_run_prints_65_runs_within_bl0_and_bl8(tmp_path, capsys):
    budget = ledger.load_budget()
    assert vb.main(["campaign", "--manifest", str(PILOT_A), "--dry-run", "--results", str(tmp_path / "results"),
                    "--secret-file", str(hmac_seed.write_secret_file(tmp_path / "config" / "secret"))]) == 0
    shown = json.loads(capsys.readouterr().out)
    assert shown["ok"] and shown["experiment"] == "PILOT-A" and shown["runs"] == 65
    blocks = {block["block"]: block for block in shown["blocks"]}
    assert (blocks["cheap-direct"]["runs"], blocks["fd-api"]["runs"]) == (60, 5)
    assert all(block["network"] and block["billed"] for block in blocks.values())
    for line, planned in (("BL0", 3.60), ("BL8", 1.50)):
        scope = shown["budget"][f"line {line}"]
        assert scope["planned_usd"] == planned <= scope["cap_usd"] == budget.lines[line].cap_usd
    assert shown["budget"]["experiment cap pilot"]["planned_usd"] == 5.10 <= 15
    assert [unit["unit"] for unit in shown["units"]] == [unit.key for unit in campaign.units(campaign.load(PILOT_A))]


def test_pilot_a_manifest_rehearses_offline(tmp_path):
    """Every unit of the manifest runs, offline, through `vb campaign`: 65 schema-valid records on the real pilot
    instances, 65 priced ledger rows on BL0 and BL8, and every model call on the loopback stub."""
    results, work = tmp_path / "results", tmp_path / "work"
    secret_file = hmac_seed.write_secret_file(tmp_path / "config" / "secret")
    with StubServer(lambda body: bash("echo VB_SUBMIT", "Submit at once.")) as stub:
        assert vb.main(["campaign", "--manifest", str(PILOT_A), "--provider-url", stub.url, "--results",
                        str(results), "--work", str(work), "--secret-file", str(secret_file)]) == 0
        calls = Counter(request["model"] for request in stub.requests)
    assert calls == {"gpt-oss-120b": 60, "gpt-5.4": 5}  # one call per run, every one on the stub
    out = results / "PILOT-A"
    finished = [event for event in read_jsonl(out / campaign.LOG) if event["event"] == "finish"]
    assert [(event["unit"], event["exit"]) for event in finished] == [
        ("cheap-direct-s1", 0), ("fd-api-s1", 0), ("cheap-direct-s2", 0), ("cheap-direct-s3", 0)]
    run_dirs = [path for path in sorted(out.iterdir()) if path.is_dir() and not path.name.startswith(".")]
    records = [record for path in run_dirs for record in read_jsonl(path / "records.jsonl")]
    rows = [row for path in run_dirs for row in read_jsonl(path / "ledger.jsonl")]
    assert len(records) == 65 and all(validate.validate("run-record", record) == [] for record in records)
    assert Counter((record["arm"], record["seed"]) for record in records) == {
        ("cheap_direct", 1): 20, ("cheap_direct", 2): 20, ("cheap_direct", 3): 20, ("fd_api", 1): 5}
    assert {record["task"]["instance_id"] for record in records if record["arm"] == "fd_api"} == set(
        vb.load_stream("pilot_fd_api").instances)
    assert {record["task"]["family"] for record in records} == {"F1", "F4"}  # the real pilot families
    assert all(record["experiment_id"] == "PILOT-A" and record["provenance"]["network_policy"]["network"] == "none"
               for record in records)
    assert len(rows) == 65 and all(validate.validate("ledger", row) == [] for row in rows)
    assert all(row["api_equiv_usd"] is not None and row["source"] == "provider_usage" for row in rows)  # priced
    assert Counter(row["line"] for row in rows) == {"BL0": 60, "BL8": 5}
    assert all(json.loads((path / "manifest.json").read_text())["offline"] is True for path in run_dirs)
