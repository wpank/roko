"""Pilot B's manifest (`pilot_b.toml`, 3308): its dry run, its budget with and without the $14 BL0 cap, and an offline
rehearsal of every block through `vb campaign`, with a fake `roko` and the fake `claude` of the driver's tests on the
pilot's real instances and the stub provider on loopback. Nothing is billed and no network is used.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_pilot_b.py -q
"""

from __future__ import annotations

import json
import re
import sys
from collections import Counter
from pathlib import Path

import campaign
import layout
import ledger
import secret
import validate
import vb
from common import canary, hmac_seed
from stub_provider import StubServer
from test_run_cli import FAKE_CLAUDE, RESULT

PILOT_B = layout.VB_ROOT / "experiments" / "pilot_b.toml"
# A stand-in for roko on any task: `plan run` makes two calls to the provider its roko.toml names, retrying a failed
# call twice as a client would, and records one attempt. It leaves the tree alone, so its runs complete with VS = 0.
PILOT_ROKO = r'''#!__PYTHON__
import datetime, http.client, json, os, sys, time, tomllib, urllib.error, urllib.request
from pathlib import Path

args = sys.argv[1:]
if args == ["--version"]:
    print("roko 0.1.0 (git 0fa4e0fa4e)")
    sys.exit(0)
if "validate" in args:
    sys.exit(0)
repo, slug = Path(args[args.index("--repo") + 1]), Path(args[args.index("run") + 1]).name
config = tomllib.loads(Path(os.environ["ROKO_CONFIG"]).read_text())
[provider], [model] = config["providers"].values(), config["models"]
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
answered = 0
for turn in range(2):
    for _ in range(3):
        request = urllib.request.Request(provider["base_url"] + "/chat/completions", data=json.dumps(
            {"model": model, "messages": [{"role": "user", "content": "Step %d of the task." % turn}]}).encode(),
            headers={"Content-Type": "application/json",
                     "Authorization": "Bearer " + os.environ[provider["api_key_env"]]})
        try:
            json.loads(opener.open(request, timeout=30).read())
        except (OSError, ValueError, http.client.HTTPException):  # an injected fault: retry
            time.sleep(0.2)
            continue
        answered += 1
        break
passed = answered == 2
roko, now = repo / ".roko", datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%S.%fZ")
(roko / "state" / "graph" / slug).mkdir(parents=True)
(roko / "episodes.jsonl").write_text(json.dumps({
    "task_id": "T01", "model": model, "backend": "cerebras", "success": passed, "turns": 2, "completed_at": now,
    "failure_reason": None if passed else "provider: no answer",
    "extra": {"plan_id": slug, "attempt_key": "graph-%s-1:%s:T01:1" % (slug, slug)}}) + "\n")
verdicts = {"roko.gate.verdict@1": {"value": {"verdicts": {"T01": "passed"}}}} if passed else {}
(roko / "state" / "graph" / slug / "checkpoint.json").write_text(json.dumps(
    {"plan_id": slug, "status": "succeeded" if passed else "failed", "extensions": verdicts}))
sys.exit(0 if passed else 1)
'''


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def budget_with_bl0_cap(tmp_path: Path, cap: float) -> ledger.Budget:
    """experiments/budget.toml with BL0's cap set to `cap`, as before or after dec-1089ec."""
    text = ledger.BUDGET_FILE.read_text()
    head, sep, rest = text.partition('id = "BL0"')
    rest, count = re.subn(r"(?m)^cap_usd = \S+", f"cap_usd = {cap}", rest, count=1)
    assert sep and count == 1
    path = tmp_path / f"budget-{cap:g}.toml"
    path.write_text(head + sep + rest)
    return ledger.load_budget(path)


def test_pilot_b_dry_run_lists_every_block_with_its_line_and_estimate(tmp_path, capsys):
    assert vb.main(["campaign", "--manifest", str(PILOT_B), "--dry-run", "--include", "provider-fault", "--results",
                    str(tmp_path / "results"), "--secret-file", str(tmp_path / "none")]) == 0
    shown = json.loads(capsys.readouterr().out)
    blocks = {block["block"]: block for block in shown["blocks"]}
    assert list(blocks) == ["roko-fixed", "fd-claude", "provider-fault", "mini-swe"]
    assert all(block["line"] == "BL0" and "planned_usd" in block for block in blocks.values())
    assert (blocks["roko-fixed"]["runs"], blocks["roko-fixed"]["planned_usd"], blocks["roko-fixed"]["worst_case_usd"]) \
        == (60, 4.50, 9.00)
    assert (blocks["fd-claude"]["runs"], blocks["fd-claude"]["planned_usd"], blocks["fd-claude"]["billed"]) == (
        60, 0.0, False)
    assert blocks["fd-claude"]["off_hours"] and blocks["provider-fault"]["runs"] == 5
    assert blocks["provider-fault"]["disturbances"] == ["provider_fault"]
    assert "cheap_direct_msa" in blocks["mini-swe"]["unavailable"]  # 3317's arm, not there yet: a note, no refusal
    assert shown["ok"] and shown["runs"] == 125
    assert [unit["unit"] for unit in shown["units"]] == [unit.key for unit in campaign.units(
        campaign.load(PILOT_B), include=["provider-fault"])]


def test_pilot_b_seeds_2_3_fit_only_under_the_14_dollar_bl0_cap(tmp_path):
    """After Pilot A's planned spend, roko-fixed's three seeds ($9 at their worst) pass BL0's $10 cap and fit under the
    $14 of decision 3301 (dec-1089ec); seed 1 alone fits under either."""
    results = tmp_path / "results"
    for line, usd in (("BL0", 3.60), ("BL8", 1.50)):  # what Pilot A was planned to bill
        book = ledger.Ledger(results / "PILOT-A" / f"a-{line}" / "ledger.jsonl", line=line, experiment_id="PILOT-A",
                             run_id=f"a-{line}", price_snapshot_id=ledger.DEFAULT_SNAPSHOT)
        book.path.parent.mkdir(parents=True)
        book.append(attempt_key=f"a-{line}/k:1", provider="cerebras", model_reported="gpt-oss-120b",
                    usage=ledger.vb_usage(1000, 100), cost=ledger.Cost(usd, usd, "provider_usage"), billed=True,
                    reserved_usd=usd)
    manifest = campaign.load(PILOT_B)

    def problems(budget: ledger.Budget, plan: campaign.Manifest = manifest, include: tuple = ()) -> list[str]:
        return campaign.check(vb, plan, budget=budget, results_root=results, include=include).problems

    [refused] = problems(budget_with_bl0_cap(tmp_path, 10))
    assert refused == ("block roko-fixed: line BL0: at its worst, $3.6000 held + $0.0000 for the blocks before it + "
                       "$9.0000 for it would pass $10.00")
    assert problems(budget_with_bl0_cap(tmp_path, 14)) == []
    assert problems(budget_with_bl0_cap(tmp_path, 14), include=("provider-fault",)) == []  # $13.35 of $14
    seed_1 = manifest.blocks[0].__class__(**{**manifest.blocks[0].__dict__, "seeds": (1,), "seeds_text": "1",
                                             "planned_usd": 1.50, "max_cost_usd": 3.00})
    alone = campaign.Manifest(**{**manifest.__dict__, "blocks": (seed_1, *manifest.blocks[1:])})
    assert problems(budget_with_bl0_cap(tmp_path, 10), alone) == []  # $3.60 + $3.00 at worst


def test_pilot_b_manifest_rehearses_offline(tmp_path):
    """Every block of the manifest, the optional provider_fault row included, runs offline through `vb campaign` with
    fakes for roko and claude, two instances per unit: schema-valid records and ledger rows for each block, and the
    fd_claude runs carry both U' and R."""
    results, work = tmp_path / "results", tmp_path / "work"
    secret_file = secret.create(tmp_path / "config" / "secret")
    roko = tmp_path / "bin" / "roko"
    roko.parent.mkdir()
    roko.write_text(PILOT_ROKO.replace("__PYTHON__", sys.executable))
    roko.chmod(0o755)
    claude, claude_log = tmp_path / "bin" / "claude", tmp_path / "claude.jsonl"
    config = tmp_path / "claude.json"
    config.write_text(json.dumps({"scenario": "solve", "log": str(claude_log), "result": RESULT, "files": {},
                                  "needles": [*secret.load(secret_file).needles, canary.RELEASE_CANARY],
                                  "hidden_url": "", "visible": ""}))
    claude.write_text(FAKE_CLAUDE.replace("__CONFIG__", repr(str(config))))
    claude.chmod(0o755)
    arms = {}
    for name, old, new in (("roko_fixed", 'binary = "target/debug/roko"', f"binary = {json.dumps(str(roko))}"),
                           ("fd_claude", 'program = "claude"', f"program = {json.dumps(str(claude))}")):
        text = (layout.ARMS_DIR / f"{name}.toml").read_text()
        assert old in text
        arms[name] = tmp_path / f"{name}.rehearsal.toml"
        arms[name].write_text(text.replace(old, new))
    with StubServer(lambda body: "Done.") as stub:
        assert vb.main(["campaign", "--manifest", str(PILOT_B), "--provider-url", stub.url, "--results",
                        str(results), "--work", str(work), "--secret-file", str(secret_file), "--include",
                        "provider-fault", "--limit", "2", "--arm-file", f"roko_fixed={arms['roko_fixed']}",
                        "--arm-file", f"fd_claude={arms['fd_claude']}", "--transcripts"]) == 0
        assert stub.requests and {request["model"] for request in stub.requests} == {"gpt-oss-120b"}
    out = results / "PILOT-B"
    finished = [event for event in read_jsonl(out / campaign.LOG) if event["event"] == "finish"]
    assert sorted(event["unit"] for event in finished) == sorted(
        ["roko-fixed-s1", "roko-fixed-s2", "roko-fixed-s3", "fd-claude-s1", "fd-claude-s2", "fd-claude-s3",
         "provider-fault-s1"]) and all(event["exit"] == 0 for event in finished)
    by_block: dict[str, list[dict]] = {}
    for event in finished:
        run_dir = out / event["run_id"]
        records, rows = read_jsonl(run_dir / "records.jsonl"), read_jsonl(run_dir / "ledger.jsonl")
        assert len(records) == 2 and all(validate.validate("run-record", record) == [] for record in records)
        assert rows and all(validate.validate("ledger", row) == [] for row in rows)
        by_block.setdefault(event["unit"].rsplit("-s", 1)[0], []).extend(records)
    assert Counter(record["arm"] for record in by_block["roko-fixed"]) == {"roko_fixed": 6}
    assert {record["execution"]["status"] for record in by_block["roko-fixed"]} == {"completed"}
    claude_runs = by_block["fd-claude"]
    assert Counter(record["arm"] for record in claude_runs) == {"fd_claude": 6}
    for record in claude_runs:  # U' (the headline, from modelUsage) and R (the CLI's own figure) on every run
        [attempt] = record["execution"]["attempts"]
        assert attempt["cli"]["u_prime_usd"] > 0 and attempt["cli"]["r_usd"] == RESULT["total_cost_usd"]
        assert record["costs"]["api_equiv_usd"] == attempt["cli"]["u_prime_usd"]
        assert record["costs"]["vendor_usd"] == RESULT["total_cost_usd"] and record["costs"]["billed_usd"] == 0.0
    faulted = by_block["provider-fault"]
    assert all(record["stream"]["perturbations_active"] == ["provider_fault"] for record in faulted)
    assert {record["task"]["instance_id"] for record in faulted} <= set(campaign.load(PILOT_B).blocks[2].instances)
    [fault_run] = [event["run_id"] for event in finished if event["unit"] == "provider-fault-s1"]
    profiles = {row["profile"]["name"] for row in read_jsonl(out / fault_run / "proxy.jsonl")}
    assert profiles == {"http_5xx", "rate_limit"}  # the spec's first two positions
