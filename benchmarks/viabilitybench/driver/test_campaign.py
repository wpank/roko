"""Offline tests of `vb campaign` (campaign.py, 3306): manifests validated, estimated and run block by block, with the
toy family and the stub provider. No test calls a provider: every unit runs against a loopback stub.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_campaign.py -q
"""

from __future__ import annotations

import json
import shutil
from pathlib import Path

import pytest

import campaign
import layout
import ledger
import validate
import vb
from common import hmac_seed
from stub_provider import StubServer, bash, scripted

TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")
CORRECT = ("cat > calc/ops.py <<'EOF'\ndef clamp(value, low, high):\n"
           "    \"\"\"Return value limited to the range [low, high].\"\"\"\n    return max(low, min(value, high))\nEOF")
SOLVE = scripted({"": [bash(CORRECT, "Implement it."), bash("echo VB_SUBMIT")]})
EXPERIMENT = "TEST-CAMPAIGN"


@pytest.fixture
def places(tmp_path: Path) -> dict[str, Path]:
    secret = hmac_seed.write_secret_file(tmp_path / "private-config" / "secret")
    return {"tmp": tmp_path, "results": tmp_path / "results", "work": tmp_path / "work", "secret": secret}


def manifest(places: dict[str, Path], *blocks: str, order: str = 'order = { kind = "as_listed" }',
             name: str = "experiment.toml") -> Path:
    """A manifest of `blocks`, each the body of a [[block]] table."""
    text = (f'schema_version = "vb.experiment/1"\nid = "{EXPERIMENT}"\nrequires_lock = false\nrequires_live = []\n'
            f"{order}\n" + "".join(f"\n[[block]]\n{body}\n" for body in blocks))
    path = places["tmp"] / name
    path.write_text(text)
    return path


def block(block_id: str, **fields: object) -> str:
    """A toy block on cheap_direct, with `fields` changed or added."""
    values = {"id": block_id, "stream": TOY_STREAM, "arm": "cheap_direct", "model": "gpt-oss-120b", "seeds": "1",
              "line": "BL0", "planned_usd": 0.05, "max_cost_usd": 1.0, **fields}
    return "\n".join(f"{key} = {json.dumps(value)}" for key, value in values.items() if value is not None)


def run_campaign(places: dict[str, Path], path: Path, url: str, *extra: str) -> int:
    return vb.main(["campaign", "--manifest", str(path), "--provider-url", url, "--results", str(places["results"]),
                    "--work", str(places["work"]), "--secret-file", str(places["secret"]), *extra])


def dry_run(places: dict[str, Path], path: Path, url: str, capsys, *extra: str) -> tuple[int, dict]:
    code = run_campaign(places, path, url, "--dry-run", *extra)
    return code, json.loads(capsys.readouterr().out)


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def test_campaign_refuses_a_block_over_its_line_cap(places, capsys):
    """A block whose planned spend would take its line past the cap, and one whose first task could not start because
    the ledger already holds most of the line, are refused before any call: no request, no run directory."""
    cap = ledger.load_budget().lines["BL0"].cap_usd
    over = manifest(places, block("small"), block("big", planned_usd=cap))
    with StubServer(SOLVE) as stub:
        code, shown = dry_run(places, over, stub.url, capsys)
        assert code == 2 and not shown["ok"]
        [problem] = shown["problems"]
        assert problem.startswith("block big: line BL0: $0.0000 held + $0.0500 planned before it") and \
            f"would pass ${cap:.2f}" in problem
        assert run_campaign(places, over, stub.url) == 2  # and the run is refused the same way
        assert "line BL0" in capsys.readouterr().err

        # The ledger already holds all but $0.10 of BL0, and the block's two tasks can bill $0.29 each at their worst.
        book = ledger.Ledger(places["results"] / "OTHER" / "run-x" / "ledger.jsonl", line="BL0", experiment_id="OTHER",
                             run_id="run-x", price_snapshot_id=ledger.DEFAULT_SNAPSHOT)
        book.path.parent.mkdir(parents=True)
        book.append(attempt_key="run-x/k:1", provider="cerebras", model_reported="gpt-oss-120b",
                    usage=ledger.vb_usage(1000, 100), cost=ledger.Cost(cap - 0.10, cap - 0.10, "provider_usage"),
                    billed=True, reserved_usd=cap)
        tight = manifest(places, block("small", planned_usd=0.01), name="tight.toml")
        code, shown = dry_run(places, tight, stub.url, capsys)
        assert code == 2 and shown["budget"]["line BL0"]["held_usd"] == pytest.approx(cap - 0.10)
        assert shown["problems"][0].startswith(f"block small: line BL0: at its worst, ${cap - 0.10:.4f} held")
        assert "+ $0.5786 for it would pass" in shown["problems"][0]  # two tasks at $0.28932, under max_cost_usd $1
        assert run_campaign(places, tight, stub.url) == 2
        assert stub.requests == [] and not (places["results"] / EXPERIMENT).exists()


def test_campaign_refuses_an_unknown_model_and_a_reserved_line(places, capsys, tmp_path):
    arm = (layout.ARMS_DIR / "cheap_direct.toml").read_text().replace(
        'models_allow = ["gpt-oss-120b"]', 'models_allow = ["gpt-oss-120b", "mystery-model-9"]')
    (tmp_path / "priced.toml").write_text(arm)
    priced = ("--arm-file", f"cheap_direct={tmp_path / 'priced.toml'}")  # the arm allows a model without a price
    cases = {"not-allowed": (block("not-allowed", model="gpt-5.4"), (), "allows gpt-oss-120b, not gpt-5.4"),
             "unpriced": (block("unpriced", model="mystery-model-9"), priced,
                          "mystery-model-9 has no row in prices-2026-09-28"),
             "held-back": (block("held-back", line="BL12"), (), "budget line BL12 is held back"),
             "no-arm": (block("no-arm", arm="nobody"), (), "no such file"),
             "no-plan": (block("no-plan", planned_usd=None), (), "a billed block needs planned_usd")}
    with StubServer(SOLVE) as stub:
        for name, (body, extra, expected) in cases.items():
            path = manifest(places, body, name=f"{name}.toml")
            code, shown = dry_run(places, path, stub.url, capsys, *extra)
            assert code == 2 and any(expected in problem for problem in shown["problems"]), (name, shown["problems"])
            assert run_campaign(places, path, stub.url, *extra) == 2
        assert stub.requests == [] and not (places["results"] / EXPERIMENT).exists()
    # Without a loopback provider, a network block needs its --max-cost-usd, which must cover one task.
    for body, expected in ((block("cheap", max_cost_usd=0.1), "one task can cost up to $0.29"),
                           (block("free", max_cost_usd=None), "a network run needs max_cost_usd")):
        code = vb.main(["campaign", "--manifest", str(manifest(places, body, name="net.toml")), "--dry-run",
                        "--results", str(places["results"]), "--secret-file", str(places["secret"])])
        shown = json.loads(capsys.readouterr().out)
        assert code == 2 and any(expected in problem for problem in shown["problems"]), shown["problems"]


def test_a_valid_manifest_rehearses_offline_and_resumes(places, capsys):
    """Two blocks, the second on an instance subset of the stream: --units 1 runs the first, a rerun goes on with the
    second, and a third run finds nothing left. Each unit is a real `vb run`, with valid records and ledger rows."""
    path = manifest(places, block("first"), block("second", seeds="2", instances=["F1-l1-0002"]))
    with StubServer(SOLVE) as stub:
        code, shown = dry_run(places, path, stub.url, capsys)
        assert code == 0 and shown["ok"] and shown["runs"] == 3 and shown["planned_usd"] == pytest.approx(0.10)
        assert [unit["unit"] for unit in shown["units"]] == ["first", "second"]
        assert shown["budget"]["line BL0"]["planned_usd"] == pytest.approx(0.10)
        assert run_campaign(places, path, stub.url, "--units", "1") == 0
        first_calls = len(stub.requests)
        assert run_campaign(places, path, stub.url) == 0
        assert run_campaign(places, path, stub.url) == 0  # every unit finished: nothing runs
        calls = len(stub.requests)
    assert first_calls == 4 and calls == 6  # two calls per task: first has two tasks, second one
    out = places["results"] / EXPERIMENT
    events = read_jsonl(out / campaign.LOG)
    assert [(event["event"], event["unit"], event.get("exit")) for event in events] == [
        ("start", "first", None), ("finish", "first", 0), ("start", "second", None), ("finish", "second", 0)]
    assert events[2]["instances"] == ["F1-l1-0002"] and events[0]["secret_fingerprint"].startswith("sha256:")
    for run_id, instances in (("first-1", {"F1-l1-0001", "F1-l1-0002"}), ("second-1", {"F1-l1-0002"})):
        records = read_jsonl(out / run_id / "records.jsonl")
        assert {record["task"]["instance_id"] for record in records} == instances
        assert all(validate.validate("run-record", record) == [] for record in records)
        assert all(record["vs"]["label"] == 1 and record["experiment_id"] == EXPERIMENT for record in records)
        assert all(validate.validate("ledger", row) == [] for row in read_jsonl(out / run_id / "ledger.jsonl"))
    assert read_jsonl(out / "second-1" / "records.jsonl")[0]["seed"] == 2
    assert (out / campaign.STREAMS / "second.toml").is_file()  # the subset's stream, in a directory reports skip


def test_a_unit_that_left_records_stops_the_campaign_until_it_is_moved(places, capsys):
    path = manifest(places, block("only"))
    out = places["results"] / EXPERIMENT
    (out / "only-1").mkdir(parents=True)
    (out / "only-1" / "records.jsonl").write_text('{"partial": true}\n')
    (out / campaign.LOG).write_text(json.dumps({"event": "start", "unit": "only", "run_id": "only-1"}) + "\n")
    with StubServer(SOLVE) as stub:
        assert run_campaign(places, path, stub.url) == 2
        assert "left records or ledger rows" in capsys.readouterr().err and stub.requests == []
        abandoned = places["results"] / f"{EXPERIMENT}{campaign.ABANDONED}"
        abandoned.mkdir()
        shutil.move(out / "only-1", abandoned / "only-1")
        assert run_campaign(places, path, stub.url) == 0  # retried under a new run id
    assert len(read_jsonl(out / "only-2" / "records.jsonl")) == 2


def test_max_cost_usd_caps_the_whole_campaign(places, capsys):
    """The campaign's --max-cost-usd: each billed unit gets its share of its block's cap or what the experiment's
    books leave under the campaign's, whichever is smaller, and a unit left less than one task's worst case does not
    start."""
    path = manifest(places, block("first"), block("second", max_cost_usd=2.0))
    out = places["results"] / EXPERIMENT
    with StubServer(SOLVE) as stub:
        assert run_campaign(places, path, stub.url, "--max-cost-usd", "0") == 2
        assert run_campaign(places, path, stub.url, "--max-cost-usd", "0.2") == 2  # one toy task may cost $0.29
        assert "less than one task's worst case" in capsys.readouterr().err and stub.requests == []
        code, shown = dry_run(places, path, stub.url, capsys, "--max-cost-usd", "1.2")
        assert code == 0 and any(note.startswith("--max-cost-usd $1.20: ") for note in shown["notes"])
        assert run_campaign(places, path, stub.url, "--max-cost-usd", "1.2") == 0
    caps = {}
    for run_id in ("first-1", "second-1"):
        argv = json.loads((out / run_id / "manifest.json").read_text())["argv"]
        caps[run_id] = float(argv[argv.index("--max-cost-usd") + 1])
    spent = sum(row["billed_usd"] for row in read_jsonl(out / "first-1" / "ledger.jsonl"))
    assert spent > 0 and caps["first-1"] == 1.0  # its block's $1.00: the campaign's $1.20 had room for it
    assert 1.2 - spent - 1e-6 <= caps["second-1"] <= 1.2 - spent + 1e-12  # what first left, under its $2.00 share


def test_an_instance_keeps_one_secret_across_campaigns(places, capsys):
    """S09 §4.1: an instance that another campaign ran under another secret file is refused."""
    other = places["results"] / "OTHER-EXP" / campaign.LOG
    other.parent.mkdir(parents=True)
    other.write_text(json.dumps({"event": "start", "unit": "x", "run_id": "x-1", "instances": ["F1-l1-0002"],
                                 "secret_fingerprint": "sha256:0000000000000000"}) + "\n")
    with StubServer(SOLVE) as stub:
        code, shown = dry_run(places, manifest(places, block("clash")), stub.url, capsys)
    assert code == 2
    [problem] = shown["problems"]
    assert "F1-l1-0002 ran under secret sha256:0000000000000000 (OTHER-EXP/x)" in problem
    # A block that names another secret's fingerprint is refused too.
    with StubServer(SOLVE) as stub:
        code, shown = dry_run(places, manifest(places, block("named", secret_fingerprint="sha256:1111111111111111"),
                                               name="named.toml"), stub.url, capsys)
    assert code == 2 and any("runs under secret sha256:1111111111111111" in item for item in shown["problems"])


def test_daily_interleave_orders_units_by_day_and_includes_optional_blocks_on_request(places):
    path = manifest(places, block("roko", seeds="1-3"), block("claude", seeds="1-3"), block("extra", optional=True),
                    order='order = { kind = "daily_interleave", seed = 7 }')
    loaded = campaign.load(path)
    order = [(unit.day, unit.key) for unit in campaign.units(loaded)]
    assert [day for day, _ in order] == [1, 1, 2, 2, 3, 3]
    assert sorted(order) == [(1, "claude-s1"), (1, "roko-s1"), (2, "claude-s2"), (2, "roko-s2"), (3, "claude-s3"),
                             (3, "roko-s3")]
    assert order == [(unit.day, unit.key) for unit in campaign.units(loaded)]  # the same every time
    with_extra = [unit.key for unit in campaign.units(loaded, include=["extra"])]
    assert "extra-s1" in with_extra and len(with_extra) == 7
    for text, expected in (('order = { kind = "daily_interleave" }', "needs a seed"),
                           ('order = { kind = "shuffled" }', "is not one of")):
        with pytest.raises(campaign.CampaignError, match=expected):
            campaign.load(manifest(places, block("a"), order=text, name="bad.toml"))
    with pytest.raises(campaign.CampaignError, match="block ids repeat"):
        campaign.load(manifest(places, block("a"), block("a"), name="twice.toml"))


def test_log1_refuses_to_start_without_the_lock(places, capsys, tmp_path):
    """3341 (S09 SC1): LOG1, a live `E-` experiment and a manifest that says requires_lock start only under a
    pre-registration lock that is committed and checks clean. Without one, `vb run` exits 2 before any directory or
    request, and the campaign's dry run lists the refusal and its run stops before any unit."""
    missing = tmp_path / "no-lock" / "prereg.lock.json"
    loose = tmp_path / "loose" / "prereg.lock.json"  # a lock file no repository tracks
    loose.parent.mkdir()
    loose.write_text('{"schema_version": "vb.prereg_lock/1"}\n')
    assert campaign.requires_lock("LOG1") and campaign.requires_lock("E-H5-live")
    assert not campaign.requires_lock("PILOT-A") and not campaign.requires_lock(EXPERIMENT)
    with StubServer(SOLVE) as stub:
        for experiment, lock, why in (("LOG1", missing, f"no pre-registration lock at {missing}"),
                                      ("E-P1-live", loose, f"the lock at {loose} is not committed")):
            code = vb.main(["run", "--experiment", experiment, "--stream", TOY_STREAM, "--arm", "cheap_direct",
                            "--model", "gpt-oss-120b", "--seeds", "1", "--provider-url", stub.url, "--results",
                            str(places["results"]), "--work", str(places["work"]), "--secret-file",
                            str(places["secret"]), "--lock", str(lock)])
            assert code == 2
            assert (f"experiment {experiment} runs only under the pre-registration lock: {why}"
                    in capsys.readouterr().err)
        assert stub.requests == [] and not places["results"].exists()
        path = manifest(places, block("locked"))
        path.write_text(path.read_text().replace("requires_lock = false", "requires_lock = true"))
        code, summary = dry_run(places, path, stub.url, capsys, "--lock", str(missing))
        assert code == 2 and summary["requires_lock"] is True
        assert f"requires_lock: no pre-registration lock at {missing}" in " ".join(summary["problems"])
        assert run_campaign(places, path, stub.url, "--lock", str(missing)) == 2
        assert "refused before any run: requires_lock" in capsys.readouterr().err
        assert stub.requests == [] and not (places["results"] / EXPERIMENT).exists()


def fake_census(rows: list[dict], sha: str = "abc1234") -> object:
    """A monkeypatch replacement for `campaign.census_report`, matching its real signature, that returns a fixed
    `roko.loop_census/1` report instead of running a roko binary (3359's own test plan: "tests with a fake
    census")."""
    def fake(repo=None, roko_bin=None):
        return {"schema": campaign.LOOPS_SCHEMA, "harness_sha": sha, "rows": rows}
    return fake


def test_live_manifest_refuses_loops_that_are_not_live(places, capsys, monkeypatch):
    """3359 (S09 §5): a manifest's requires_live loops must be LIVE (roko.loop_census/1's audit `state`) at the
    harness sha the campaign would run under. One flagged/dormant loop refuses before any call and writes a NOT
    RUN stub naming the loop and the sha; once every required loop is LIVE, the same manifest runs normally."""
    path = manifest(places, block("live"))
    path.write_text(path.read_text().replace("requires_live = []", 'requires_live = ["L-test-loop"]'))
    dormant = fake_census([{"loop": "L-test-loop", "state": "flagged", "reason": "dormant:no_learning"}])
    live = fake_census([{"loop": "L-test-loop", "state": "live"}])

    with StubServer(SOLVE) as stub:
        monkeypatch.setattr(campaign, "census_report", dormant)
        code, summary = dry_run(places, path, stub.url, capsys)
        assert code == 2 and summary["harness_sha"] == "abc1234" and summary["not_run"] == ["L-test-loop"]
        assert ("requires_live: not LIVE at harness abc1234: L-test-loop (flagged, dormant:no_learning)"
                in " ".join(summary["problems"]))
        assert run_campaign(places, path, stub.url) == 2
        assert "refused before any run: requires_live" in capsys.readouterr().err
        out = places["results"] / EXPERIMENT
        assert stub.requests == [] and not (out / campaign.LOG).exists()  # no unit was ever dispatched
        stub_path = out / campaign.NOT_RUN_FILE
        record = json.loads(stub_path.read_text())
        assert record["schema"] == campaign.NOT_RUN_SCHEMA and record["status"] == "not_run"
        assert record["experiment_id"] == EXPERIMENT and record["harness_sha"] == "abc1234"
        assert record["loops_not_live"] == ["L-test-loop"]
        assert stub.requests == []  # the stub was never called: the refusal happens before any dispatch

        monkeypatch.setattr(campaign, "census_report", live)
        code, summary = dry_run(places, path, stub.url, capsys)
        assert code == 0 and summary["problems"] == [] and summary["not_run"] == []
        assert run_campaign(places, path, stub.url) == 0
        assert stub.requests  # this time the run went ahead and reached the stub provider
