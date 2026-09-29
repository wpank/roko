"""Tests for the ViabilityBench report: pass^k, the metrics and the bundle check, on synthetic run records.

Every record is built from the schema's example and validates against `vb.run_record/1`; nothing calls a model.
Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_analysis.py -q
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from fractions import Fraction
from pathlib import Path

import pytest

import metrics
import passk
import report
import validate  # schema/validate.py, on sys.path through report

EXAMPLES = report.VB_ROOT / "schema" / "examples"
EXAMPLE = json.loads((EXAMPLES / "run-record.json").read_text())
EXPERIMENT = "PILOT-T"


def run_record(instance: str, seed: int = 1, *, run_id: str = "run-a", arm: str = "cheap_direct", label: int = 1,
               unknown: bool = False, visible: bool | None = None, status: str = "completed", cost: float | None = 0.02,
               billed: bool = True, vendor: float | None = None, verdict: str | None = None, honeypot: bool = False,
               started: str | None = None, finished: str | None = None) -> dict:
    """A valid run record for one (instance, seed). The visible checks pass iff VS = 1 unless `visible` says."""
    record = json.loads(json.dumps(EXAMPLE))
    family, ladder = ("PL", 5) if instance.startswith("PL") else (instance.split("-")[0], int(instance.split("-")[1][1:]))
    record.update(experiment_id=EXPERIMENT, run_id=run_id, arm=arm, seed=seed,
                  record_id=f"sha256:{run_id}/{instance}/{seed}")
    record["task"].update(family=family, instance_id=instance, ladder=ladder, is_honeypot=honeypot)
    record["execution"]["status"] = status
    attempt = record["execution"]["attempts"][0]
    attempt["attempt_key"] = f"{run_id}/{instance}.s{seed}:1"
    if verdict:
        attempt["gate_verdict"] = verdict
    if started:
        record["execution"].update(started_at=started, finished_at=finished)
    record["visible"]["passed"] = bool(label and not unknown) if visible is None else visible
    record["vs"] = {"label": 0 if unknown else label, "unknown": unknown,
                    "checks": {"completion": int(status == "completed"), "visible_clean": int(record["visible"]["passed"]),
                               "hidden": None if unknown else label, "integrity": 1},
                    "truth_suite_version": "1.0.0", "failed": [] if label and not unknown else ["hidden.cases"]}
    if cost is None:
        record["costs"] = {"api_equiv_usd": None, "billed_usd": None, "without_cache_usd": None, "vendor_usd": vendor,
                           "source": "unknown", "meter_cross_check_usd": None}
    else:
        record["costs"] = {"api_equiv_usd": cost, "billed_usd": cost if billed else 0.0, "without_cache_usd": cost,
                           "vendor_usd": vendor, "source": "provider_usage" if billed else "cli_usage",
                           "meter_cross_check_usd": None}
    assert validate.validate("run-record", record) == []
    return record


def ledger_row(record: dict, line: str) -> dict:
    row = json.loads((EXAMPLES / "ledger.json").read_text())
    costs = record["costs"]
    row.update(line=line, experiment_id=record["experiment_id"], run_id=record["run_id"],
               attempt_key=record["execution"]["attempts"][0]["attempt_key"], api_equiv_usd=costs["api_equiv_usd"],
               billed_usd=costs["billed_usd"], reserved_usd=0.13, source=costs["source"])
    assert validate.validate("ledger", row) == []
    return row


def write_run(experiment_dir: Path, records: list[dict], line: str = "BL0") -> Path:
    """A run directory as `vb run` leaves it: a vb.run_manifest/1 planning exactly these runs, records, a ledger."""
    first = records[0]
    instances = sorted({record["task"]["instance_id"] for record in records})
    seeds = sorted({record["seed"] for record in records})
    path = experiment_dir / first["run_id"]
    path.mkdir(parents=True)
    manifest = {"schema_version": "vb.run_manifest/1", "experiment_id": first["experiment_id"],
                "run_id": first["run_id"], "arm": first["arm"], "model": "gpt-oss-120b", "seeds": seeds,
                "tasks": len(instances), "runs": len(instances) * len(seeds), "price_snapshot_id": "prices-2026-09-28",
                "config_hash": EXAMPLE["config_hash"],
                "config": {"stream": {"id": "pilot", "instances": instances, "spec_variant": "precise"}}}
    (path / "manifest.json").write_text(json.dumps(manifest))
    (path / "records.jsonl").write_text("".join(json.dumps(record) + "\n" for record in records))
    (path / "ledger.jsonl").write_text("".join(json.dumps(ledger_row(record, line)) + "\n" for record in records))
    (path / "private").mkdir()  # holds canaries in a real run; a bundle must never copy it
    return path


def pilot_runs() -> dict[str, list[dict]]:
    """cheap_direct on 4 tasks x 3 seeds, fd_api on 2 x 1, roko_fixed on 2 x 1. The comments give each run's VS."""
    return {
        "run-a": [
            run_record("F4-l1-0001", 1), run_record("F4-l1-0001", 2), run_record("F4-l1-0001", 3),  # 1 1 1
            run_record("F4-l3-0002", 1, label=0, visible=True),  # a false green: visible pass, VS = 0
            run_record("F4-l3-0002", 2),
            run_record("F4-l3-0002", 3, label=0, visible=True),  # a false green
            run_record("F1-l3-0003", 1),
            run_record("F1-l3-0003", 2, unknown=True, visible=True),  # unknown: 0 in the headline, 1 in the bound
            run_record("F1-l3-0003", 3),
            run_record("F1-l5-0004", 1, label=0, status="aborted_cap"),  # stays in with VS = 0; cap censoring
            run_record("F1-l5-0004", 2, label=0, visible=True, status="infra_error"),  # excluded, never a false green
            run_record("F1-l5-0004", 3, label=0),
        ],
        "run-b": [run_record("F4-l1-0001", run_id="run-b", arm="fd_api", cost=0.2),
                  run_record("F4-l3-0002", run_id="run-b", arm="fd_api", label=0, visible=True, cost=0.2)],
        "run-c": [run_record("F4-l3-0002", run_id="run-c", arm="roko_fixed", label=0, visible=False, verdict="passed"),
                  run_record("F4-l1-0001", run_id="run-c", arm="roko_fixed", label=0, visible=True,
                             verdict="unverified")],  # not a reported pass: counted apart
    }


def budget_file(path: Path, caps: dict[str, float], extra: str = "") -> Path:
    """A vb.budget/1 file (gap-33d54b's experiments/budget.toml) with these line caps."""
    lines = "".join(f'\n[[line]]\nid = "{line}"\ncap_usd = {cap}\n' for line, cap in caps.items())
    path.write_text(f'schema_version = "vb.budget/1"\n{lines}{extra}')
    return path


@pytest.fixture
def pilot(tmp_path: Path) -> dict[str, Path]:
    results = tmp_path / "results"
    for run_id, records in pilot_runs().items():
        write_run(results / EXPERIMENT, records, line="BL8" if run_id == "run-b" else "BL0")
    budget = budget_file(tmp_path / "budget.toml", {"BL0": 10, "BL8": 8}, "\n[programme]\nstop_usd = 400\n")
    return {"results": results, "experiment": results / EXPERIMENT, "budget": budget, "tmp": tmp_path}


def all_records() -> list[dict]:
    return [record for records in pilot_runs().values() for record in records]


def the(found: list[metrics.Metric], metric: str, arm: str = "cheap_direct", cell: str = "all",
        cost_basis: str | None = None) -> metrics.Metric:
    matches = [m for m in found if m.metric == metric and m.cell == cell and m.cost_basis == cost_basis
               and {row["arm"] for row in m.cut.rows} == {arm}]
    assert len(matches) == 1, f"{metric} {arm} {cell}: {len(matches)} matches"
    return matches[0]


def make_report(pilot: dict[str, Path], *extra: str) -> dict:
    assert report.main(["--experiment", EXPERIMENT, "--results", str(pilot["results"]), *extra]) == 0
    return json.loads((pilot["experiment"] / "metrics.json").read_text())


def test_pass_k_matches_hand_computed_cases():
    assert passk.task_pass_k(3, 3, 3) == 1
    assert passk.task_pass_k(2, 3, 2) == Fraction(1, 3)  # C(2,2)/C(3,2)
    assert passk.task_pass_k(4, 5, 3) == Fraction(2, 5)  # C(4,3)/C(5,3) = 4/10
    assert passk.task_pass_k(2, 5, 3) == 0  # fewer successes than k
    assert passk.task_pass_k(3, 5, 1) == Fraction(3, 5)  # pass^1 is the success share
    with pytest.raises(ValueError, match="has no pass"):
        passk.task_pass_k(1, 2, 3)

    labels = {"a": [1, 1, 0], "b": [1, 1, 1], "c": [0, 0, 0], "d": [1], "e": [1, 1, 1, 0, 1]}
    three = passk.pass_k(labels, 3)
    assert three.value == 0.35  # a: C(2,3) = 0; b: 1; c: 0; e: C(4,3)/C(5,3) = 2/5; d has one run. (0+1+0+0.4)/4
    assert three.tasks == ("a", "b", "c", "e") and three.skipped == ("d",)
    two = passk.pass_k(labels, 2)
    assert two.value == float((Fraction(1, 3) + 1 + 0 + Fraction(6, 10)) / 4)  # a: 1/C(3,2); e: C(4,2)/C(5,2)
    assert passk.pass_k({"d": [1]}, 3).value is None
    strata = passk.stratified(labels, {"a": "F1-l1", "b": "F1-l1", "c": "F4-l2", "d": "F4-l2", "e": "F4-l2"}, 3)
    assert strata["F1-l1"].value == 0.5 and strata["F4-l2"].value == float(Fraction(2, 5) / 2)

    # Through the report: cheap_direct's pass^3 over its tasks with three kept runs (F1-l5-0004 has two).
    found = metrics.arm_metrics(all_records(), EXPERIMENT, "cheap_direct")
    assert the(found, "pass_hat_3").value == pytest.approx(1 / 3)  # tasks 0001: 1, 0002: 0, 0003: 0
    assert the(found, "pass_hat_3").n == 3 and "1 task(s) with fewer than 3 runs skipped" in the(
        found, "pass_hat_3").estimator
    assert the(found, "pass_hat_3_unknown_as_1").value == pytest.approx(2 / 3)  # 0003 is then 3 of 3
    assert the(found, "pass_hat_3", cell="F4-l3").value == 0
    one_seed = the(metrics.arm_metrics(all_records(), EXPERIMENT, "fd_api"), "pass_hat_3", arm="fd_api")
    assert one_seed.value is None and one_seed.n == 0 and "null: no task has 3 runs" in one_seed.estimator


def test_labels_censoring_and_exclusions_follow_appendix_d1():
    found = metrics.arm_metrics(all_records(), EXPERIMENT, "cheap_direct")
    # Per-task shares: 0001 3/3, 0002 1/3, 0003 2/3 (unknown = 0), 0004 0/2 (the infra_error run is out).
    assert the(found, "vs_rate").value == pytest.approx(0.5) and the(found, "vs_rate").n == 11
    assert the(found, "vs_rate_unknown_as_1").value == pytest.approx(7 / 12)
    assert the(found, "vs_rate", cell="l3").value == pytest.approx(0.5)  # (1/3 + 2/3) / 2
    assert the(found, "cap_censored_runs").value == 1 and the(found, "infra_error_runs").value == 1
    assert the(found, "infra_error_runs").n == 12 and the(found, "leak_suspected_runs").value == 0
    # 9 reported passes, 3 of them with VS = 0 in the headline, 2 when unknown counts as 1.
    assert the(found, "false_greens").value == 3
    assert the(found, "false_green_rate").value == pytest.approx(3 / 9) and the(found, "false_green_rate").n == 9
    assert the(found, "false_green_rate_unknown_as_1").value == pytest.approx(2 / 9)
    for metric in found:
        assert all(row["execution"]["status"] != "infra_error" for row in metric.cut.rows) or metric.metric in (
            "infra_error_runs", "leak_suspected_runs")

    roko = metrics.arm_metrics(all_records(), EXPERIMENT, "roko_fixed")
    assert the(roko, "false_greens", arm="roko_fixed").value == 1  # the gate passed a VS = 0 tree
    assert the(roko, "unverified_runs", arm="roko_fixed").value == 1  # a visible pass, but not a reported one

    honeypots = [run_record("F4-l2-0009", seed, label=int(seed == 1), honeypot=True) for seed in (1, 2)]
    mixed = metrics.arm_metrics([*all_records(), *honeypots], EXPERIMENT, "cheap_direct")
    assert the(mixed, "vs_rate").value == pytest.approx(0.5)  # F8 honeypots are not in the resolve rate
    assert the(mixed, "honest_conflict_rate").value == 0.5 and the(mixed, "honest_conflict_rate").n == 2


def test_cost_per_vs_follows_appendix_d2():
    found = metrics.arm_metrics(all_records(), EXPERIMENT, "cheap_direct")
    per_vs = the(found, "usd_per_vs", cost_basis="api_equiv_usd")
    assert per_vs.value == pytest.approx(11 * 0.02 / 6)  # every kept run's cost over 6 VS
    assert the(found, "spend_usd", cost_basis="billed_usd").value == pytest.approx(11 * 0.02)

    no_vs = metrics.arm_metrics([run_record("F4-l1-0001", seed, label=0) for seed in (1, 2)], EXPERIMENT,
                                "cheap_direct")
    assert the(no_vs, "usd_per_vs", cost_basis="api_equiv_usd").value is None
    assert the(no_vs, "usd_per_vs", cost_basis="api_equiv_usd").estimator.endswith("null: no VS")

    unknown = metrics.arm_metrics([run_record("F4-l1-0001", 1), run_record("F4-l1-0001", 2, cost=None)], EXPERIMENT,
                                  "cheap_direct")
    assert the(unknown, "usd_per_vs", cost_basis="api_equiv_usd").value is None  # never from the known run alone
    assert "1 run(s) without a known cost" in the(unknown, "usd_per_vs", cost_basis="api_equiv_usd").estimator
    assert the(unknown, "unknown_cost_runs").value == 1
    assert the(unknown, "spend_usd", cost_basis="api_equiv_usd").value is None

    # A subscription CLI arm: U' is the headline, R (the CLI's own figure) and their gap beside it, $0 billed.
    cli = [run_record("F4-l1-0001", seed, arm="fd_claude", cost=0.9, billed=False, vendor=1.0, label=int(seed != 3))
           for seed in (1, 2, 3)]
    found = metrics.arm_metrics(cli, EXPERIMENT, "fd_claude")
    assert the(found, "usd_per_vs", "fd_claude", cost_basis="api_equiv_usd").value == pytest.approx(2.7 / 2)
    assert the(found, "usd_per_vs_vendor", "fd_claude", cost_basis="api_equiv_usd").value == pytest.approx(1.5)
    assert the(found, "cost_gap_ur", "fd_claude", cost_basis="api_equiv_usd").value == pytest.approx(0.1)
    assert the(found, "spend_usd", "fd_claude", cost_basis="billed_usd").value == 0
    assert not [m for m in metrics.arm_metrics(all_records(), EXPERIMENT, "cheap_direct") if "vendor" in m.metric]


def test_every_false_green_is_listed_with_its_run_id(pilot, capsys):
    written = make_report(pilot)
    listed = {(item["arm"], item["run_id"], item["instance_id"], item["seed"]) for item in written["false_greens"]}
    assert listed == {("cheap_direct", "run-a", "F4-l3-0002", 1), ("cheap_direct", "run-a", "F4-l3-0002", 3),
                      ("cheap_direct", "run-a", "F1-l3-0003", 2), ("fd_api", "run-b", "F4-l3-0002", 1),
                      ("roko_fixed", "run-c", "F4-l3-0002", 1)}
    by_record = {record["record_id"]: record for record in all_records()}
    for item in written["false_greens"]:
        record = by_record[item["record_id"]]
        assert record["run_id"] == item["run_id"] and metrics.vs_minus(record) == 0
        assert item["reported_by"] == ("gate" if item["arm"] == "roko_fixed" else "visible")
    assert [item["status"] for item in written["excluded"]] == ["infra_error"]
    printed = capsys.readouterr().out
    assert "False greens (5):" in printed
    for arm, run_id, instance, seed in listed:
        assert f"- {arm} {instance} seed {seed} (completed): run {run_id}," in printed


def test_metric_records_validate_and_carry_provenance(pilot):
    written = make_report(pilot, "--k", "3,5")
    assert written["schema_version"] == "vb.metrics/1" and written["price_snapshot_id"] == "prices-2026-09-28"
    rows = written["records"]
    assert rows and all(validate.validate("metric-record", row) == [] for row in rows)
    for row in rows:
        assert row["run_ids"] and set(row["run_ids"]) <= {"run-a", "run-b", "run-c"}
        assert row["label_source"] == "vs_census" and row["preregistered"] is False and row["prereg_id"] is None
        assert (row["cost_basis"] is not None) == row["metric"].startswith(("usd_per_vs", "spend_usd", "cost_gap"))
        assert row["commits"] == ["725f21e05"] and row["seeds"] and row["config_hashes"] == [EXAMPLE["config_hash"]]
    level = next(row for row in rows if row["metric"] == "vs_rate" and row.get("ladder") == 3
                 and row["arms"] == ["cheap_direct"] and "task.family ==" not in row["record_filter"])
    assert level["record_filter"] == ('experiment_id == "PILOT-T" and arm == "cheap_direct" and task.family != "PL" '
                                      'and task.ladder == 3 and execution.status not in ["infra_error", '
                                      '"leak_suspected"] and task.is_honeypot == false')
    assert level["value"] == pytest.approx(0.5) and level["n"] == 6 and level["run_ids"] == ["run-a"]
    assert {row["metric"] for row in rows} >= {"pass_hat_3", "pass_hat_5", "false_green_rate", "usd_per_vs"}


def test_check_rejects_simulated_or_incomplete_bundle(pilot, capsys):
    bundle = pilot["tmp"] / "bundle"
    make_report(pilot, "--bundle", str(bundle))
    assert not (bundle / "run-a" / "private").exists()  # only the summary files are copied
    assert report.main(["--check", str(bundle), "--budget", str(pilot["budget"])]) == 0

    missing = pilot["tmp"] / "missing"
    shutil.copytree(bundle, missing)
    records = missing / "run-a" / "records.jsonl"
    records.write_text("".join(line + "\n" for line in records.read_text().splitlines()
                               if '"seed": 2' not in line or '"F4-l3-0002"' not in line))
    capsys.readouterr()
    assert report.main(["--check", str(missing), "--budget", str(pilot["budget"])]) == 1
    out = capsys.readouterr().out
    assert "no record for F4-l3-0002 seed 2" in out and "the manifest planned 12 runs; the bundle has 11" in out

    simulated = pilot["tmp"] / "simulated"
    shutil.copytree(bundle, simulated)
    records = simulated / "run-b" / "records.jsonl"
    lines = records.read_text().splitlines()
    lines[0] = json.dumps(json.loads(lines[0]) | {"simulated": True})
    records.write_text("\n".join(lines) + "\n")
    assert report.main(["--check", str(simulated), "--budget", str(pilot["budget"])]) == 1
    out = capsys.readouterr().out
    assert "records.jsonl:1: simulated is true; only real runs count" in out
    assert "$.simulated: must be false" in out


def rewrite_json(path: Path, change) -> None:
    doc = json.loads(path.read_text())
    change(doc)
    path.write_text(json.dumps(doc))


def drop_ledger_row(path: Path) -> None:
    path.write_text("".join(line + "\n" for line in path.read_text().splitlines()[1:]))


@pytest.mark.parametrize(("tamper", "expected"), [
    (lambda b: rewrite_json(b / "metrics.json", lambda d: d["records"][0].update(run_ids=["run-z"])),
     "run ids not in the bundle: run-z"),
    (lambda b: rewrite_json(b / "metrics.json", lambda d: d["records"][0].update(run_ids=[])),
     "$.run_ids: needs at least 1 item(s), has 0"),
    (lambda b: rewrite_json(b / "metrics.json", lambda d: d["false_greens"].pop()),
     "is not listed"),
    (lambda b: rewrite_json(b / "metrics.json", lambda d: d["records"][0].update(price_snapshot_id="prices-2027-01-01")),
     "priced from prices-2027-01-01, not prices-2026-09-28"),
    (lambda b: drop_ledger_row(b / "run-a" / "ledger.jsonl"), "no row for attempt run-a/F4-l1-0001.s1:1"),
    (lambda b: (b / "metrics.json").unlink(), "no readable metrics file"),
    (lambda b: shutil.copytree(b / "run-b", b / "run-b2"), "does not name its directory run-b2"),
], ids=["foreign-run-id", "no-run-ids", "unlisted-false-green", "second-snapshot", "unledgered-attempt",
        "no-metrics", "copied-run"])
def test_check_rejects_a_tampered_bundle(pilot, capsys, tamper, expected):
    bundle = pilot["tmp"] / "bundle"
    make_report(pilot, "--bundle", str(bundle))
    tamper(bundle)
    capsys.readouterr()
    assert report.main(["--check", str(bundle), "--budget", str(pilot["budget"])]) == 1
    assert expected in capsys.readouterr().out


def test_check_holds_the_ledgers_to_every_cap(pilot, capsys):
    bundle = pilot["tmp"] / "bundle"
    make_report(pilot, "--bundle", str(bundle))
    capsys.readouterr()

    def problems(budget: Path) -> str:
        assert report.main(["--check", str(bundle), "--budget", str(budget)]) == 1
        return capsys.readouterr().out

    # BL0 spent 14 x $0.02 = $0.28 (run-a and run-c); BL8 spent 2 x $0.20 (run-b).
    tight = budget_file(pilot["tmp"] / "tight.toml", {"BL0": 0.25, "BL8": 8})
    assert "budget line BL0: the ledgers spent $0.2800, over its cap of $0.25" in problems(tight)
    uncapped = budget_file(pilot["tmp"] / "uncapped.toml", {"BL0": 10})
    assert "budget line BL8 has spend ($0.4000) but no cap" in problems(uncapped)
    pilot_cap = budget_file(pilot["tmp"] / "pilot.toml", {"BL0": 10, "BL8": 8}, '\n[[experiment]]\nid = "pilot"\n'
                            f'experiment_ids = ["{EXPERIMENT}"]\nlines = ["BL0"]\ncap_usd = 0.5\n')
    out = problems(pilot_cap)
    assert "experiment cap pilot: the ledgers spent $0.6800, over its cap of $0.50" in out
    assert "experiment cap pilot: its runs booked to line BL8, outside its lines" in out
    stop = budget_file(pilot["tmp"] / "stop.toml", {"BL0": 10, "BL8": 8}, "\n[programme]\nstop_usd = 0.6\n")
    assert "the ledgers spent $0.6800, past the programme stop of $0.60" in problems(stop)
    assert "cannot read the caps" in problems(pilot["tmp"] / "none.toml")
    assert "it needs [[line]] tables" in problems(budget_file(pilot["tmp"] / "empty.toml", {}))


def test_the_repository_budget_file_loads():
    if not report.DEFAULT_BUDGET.exists():
        pytest.skip("experiments/budget.toml arrives with gap-33d54b")
    budget = report.load_budget(report.DEFAULT_BUDGET)
    assert {"BL0", "BL8", "BL13"} <= set(budget.lines) and budget.stop_usd


def test_report_refuses_repeats_simulations_and_second_snapshots(pilot):
    records = all_records()
    with pytest.raises(metrics.MetricsError, match="seed 1 and replicate 0 twice"):
        metrics.check_unique([*records, dict(records[0], run_id="run-z")])
    other = json.loads(json.dumps(records[0]))
    other.update(price_snapshot_id="prices-2027-01-01", seed=9)
    with pytest.raises(report.ReportError, match="2 price snapshots"):
        report.build([*records, other], EXPERIMENT, ks=(3,), analysis_commit="x", computed_at="y")
    lines = (pilot["experiment"] / "run-b" / "records.jsonl").read_text().splitlines()
    lines[0] = json.dumps(json.loads(lines[0]) | {"simulated": True})
    (pilot["experiment"] / "run-b" / "records.jsonl").write_text("\n".join(lines) + "\n")
    assert report.main(["--experiment", EXPERIMENT, "--results", str(pilot["results"])]) == 1
    assert not (pilot["experiment"] / "metrics.json").exists()


def test_plan_slice_rows_stay_out_of_level_analysis():
    start = "2026-10-12T10:00:00Z"
    rows = [
        run_record("F4-l5-0001", label=0, visible=False),
        run_record("PL01-0001", run_id="run-p", arm="roko_plan", verdict="passed", cost=0.3, started=start,
                   finished="2026-10-12T10:10:00Z"),
        run_record("PL02-0001", run_id="run-p", arm="roko_plan", label=0, verdict="passed", cost=0.3, started=start,
                   finished="2026-10-12T10:30:00Z"),  # a plan-level false green
        run_record("PL01-0001", run_id="run-q", arm="fd_claude", cost=2.0, billed=False, vendor=2.5, started=start,
                   finished="2026-10-12T10:20:00Z"),
        run_record("PL02-0001", run_id="run-q", arm="fd_claude", cost=2.0, billed=False, vendor=2.5, started=start,
                   finished="2026-10-12T10:40:00Z"),
    ]
    written, _ = report.build(rows, EXPERIMENT, ks=(3,), analysis_commit="abc", computed_at="now")
    level = [row for row in written["records"] if row.get("ladder") == 5]
    assert level and all(row["arms"] == ["cheap_direct"] and "task.family != \"PL\"" in row["record_filter"]
                         for row in level)
    vs5 = next(row for row in level if row["metric"] == "vs_rate" and "task.family ==" not in row["record_filter"])
    assert vs5["value"] == 0 and vs5["n"] == 1  # the PL rows' placeholder level 5 is not in it
    assert not metrics.arm_metrics(rows, EXPERIMENT, "fd_claude")  # PL-only arms have no per-task metrics

    section, pl = metrics.plan_slice(rows, EXPERIMENT)
    assert section["label"] == "PL (exploratory, one seed, 2 features)"
    assert section["features"] == ["PL01-0001", "PL02-0001"]
    plan, claude = section["arms"]["roko_plan"], section["arms"]["fd_claude"]
    assert (plan["verified"], plan["features"], claude["verified"]) == (1, 2, 2)
    assert plan["cpf_usd"] == pytest.approx(0.6) and claude["cpf_usd"] == pytest.approx(2.0)
    assert claude["cpf_vendor_usd"] == pytest.approx(2.5)
    assert plan["makespan_median_s"] == 1200 and plan["makespan_range_s"] == [600, 1800]
    assert section["ratios"]["pl_cpf_ratio"] == pytest.approx(0.3)
    assert section["ratios"]["pl_makespan_median_ratio"] == pytest.approx(1200 / 1800)
    assert section["discordant"] == {"fd_claude": ["PL02-0001"], "roko_plan": []}
    assert section["both_verified"] == ["PL01-0001"]
    assert section["plan_level_false_greens"] == ["roko_plan:PL02-0001"]
    cell = section["table"][1]["arms"]["roko_plan"]
    assert cell["run_id"] == "run-p" and cell["vf"] == 0 and cell["queue_wait_s"] is None
    rate = next(m for m in pl if m.metric == "pl_vf_rate" and {r["arm"] for r in m.cut.rows} == {"roko_plan"})
    assert rate.value == 0.5 and rate.ci_method == "clopper_pearson" and rate.ci == metrics.clopper_pearson(1, 2)
    assert written["plan_slice"] == section and not written["false_greens"]
    for row in written["records"]:
        assert validate.validate("metric-record", row) == []
        assert row["metric"].startswith("pl_") == ("task.family == \"PL\"" in row["record_filter"])


def test_clopper_pearson_matches_published_values():
    low, high = metrics.clopper_pearson(5, 10)
    assert low == pytest.approx(0.187086, abs=1e-6) and high == pytest.approx(0.812914, abs=1e-6)
    assert metrics.clopper_pearson(0, 6) == (0.0, pytest.approx(0.459258, abs=1e-6))
    assert metrics.clopper_pearson(6, 6) == (pytest.approx(0.540742, abs=1e-6), 1.0)


def test_vb_report_hands_its_flags_to_report_py(pilot):
    vb = report.VB_ROOT / "driver" / "vb.py"
    out = pilot["tmp"] / "metrics.json"
    done = subprocess.run([sys.executable, str(vb), "report", "--experiment", EXPERIMENT, "--results",
                           str(pilot["results"]), "--out", str(out), "--bundle", str(pilot["tmp"] / "bundle")],
                          capture_output=True, text=True, timeout=120, check=False)
    assert done.returncode == 0, done.stderr
    assert json.loads(out.read_text())["experiment_id"] == EXPERIMENT and "False greens (5):" in done.stdout
    checked = subprocess.run([sys.executable, str(vb), "report", "--check", str(pilot["tmp"] / "bundle"), "--budget",
                              str(pilot["budget"])], capture_output=True, text=True, timeout=120, check=False)
    assert checked.returncode == 0, checked.stdout + checked.stderr
