"""Tests for the ViabilityBench report: pass^k, the metrics and the bundle check, on synthetic run records.

Every record is built from the schema's example and validates against `vb.run_record/1`; nothing calls a model.
Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_analysis.py -q
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
from fractions import Fraction
from pathlib import Path

import pytest

import metrics
import passk
import pilot_page
import report
import validate  # schema/validate.py, on sys.path through report

EXAMPLES = report.VB_ROOT / "schema" / "examples"
EXAMPLE = json.loads((EXAMPLES / "run-record.json").read_text())
EXPERIMENT = "PILOT-T"


def run_record(instance: str, seed: int = 1, *, run_id: str = "run-a", arm: str = "cheap_direct", label: int = 1,
               unknown: bool = False, visible: bool | None = None, status: str = "completed", cost: float | None = 0.02,
               billed: bool = True, vendor: float | None = None, verdict: str | None = None, honeypot: bool = False,
               started: str | None = None, finished: str | None = None, model: str | None = None,
               harness: str | None = None, checks: dict[str, int | None] | None = None) -> dict:
    """A valid run record for one (instance, seed). The visible checks pass iff VS = 1 unless `visible` says. Its one
    attempt ran the example's gpt-oss-120b unless `model` names another. `harness` sets
    `provenance.network_policy.harness` (3317) -- "" (the default) is the arm's own, usual harness. `checks` overrides
    individual `vs.checks` fields on top of the label-derived defaults (e.g. `checks={"hidden": 1, "integrity": 0}`
    for a VS- = 0 row whose hidden suite passed but whose A1 tamper check tripped), so a caller can pick which of the
    independent VF = 0 causes (App. D.12) a synthetic row represents."""
    record = json.loads(json.dumps(EXAMPLE))
    family, ladder = ("PL", 5) if instance.startswith("PL") else (instance.split("-")[0], int(instance.split("-")[1][1:]))
    record.update(experiment_id=EXPERIMENT, run_id=run_id, arm=arm, seed=seed,
                  record_id=f"sha256:{run_id}/{instance}/{seed}")
    if harness:
        record["provenance"]["network_policy"] = {"harness": harness}
    record["task"].update(family=family, instance_id=instance, ladder=ladder, is_honeypot=honeypot)
    record["execution"]["status"] = status
    attempt = record["execution"]["attempts"][0]
    attempt["attempt_key"] = f"{run_id}/{instance}.s{seed}:1"
    if model:
        attempt.update(model_requested=model, model_reported=model)
    if verdict:
        attempt["gate_verdict"] = verdict
    if started:
        record["execution"].update(started_at=started, finished_at=finished)
    record["visible"]["passed"] = bool(label and not unknown) if visible is None else visible
    record["vs"] = {"label": 0 if unknown else label, "unknown": unknown,
                    "checks": {"completion": int(status == "completed"), "visible_clean": int(record["visible"]["passed"]),
                               "hidden": None if unknown else label, "integrity": 1, **(checks or {})},
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


def write_run(experiment_dir: Path, records: list[dict], line: str = "BL0", model: str = "gpt-oss-120b") -> Path:
    """A run directory as `vb run` leaves it: a vb.run_manifest/1 planning exactly these runs, records, a ledger.
    `model` is the run's pinned model (`vb run --model`), which the manifest records."""
    first = records[0]
    instances = sorted({record["task"]["instance_id"] for record in records})
    seeds = sorted({record["seed"] for record in records})
    path = experiment_dir / first["run_id"]
    path.mkdir(parents=True)
    manifest = {"schema_version": "vb.run_manifest/1", "experiment_id": first["experiment_id"],
                "run_id": first["run_id"], "arm": first["arm"], "model": model, "seeds": seeds,
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


def test_an_already_satisfied_run_is_counted_apart_and_never_as_a_pass():
    # gap-9eb1e1: Roko's final attempt changed nothing and its checks passed on the tree as it was. The runner
    # records that run as failed (S05 §0.1 (a) counts only `passed` as Roko's completion), and the report counts it
    # in a metric of its own: never a reported pass, a false green or a VS.
    satisfied = run_record("F4-l3-0002", 1, run_id="run-s", arm="roko_fixed", label=0, visible=False, status="failed",
                           verdict="already_satisfied")
    runs = [satisfied, run_record("F4-l3-0002", 2, run_id="run-s", arm="roko_fixed", verdict="passed"),
            run_record("F4-l3-0002", 3, run_id="run-s", arm="roko_fixed", label=0, visible=True, verdict="unverified")]
    assert metrics.final_verdict(satisfied) == "already_satisfied" and not metrics.reported_pass(satisfied)
    found = metrics.arm_metrics(runs, EXPERIMENT, "roko_fixed")
    assert the(found, "already_satisfied_runs", arm="roko_fixed").value == 1
    assert the(found, "already_satisfied_runs", arm="roko_fixed").n == 3
    assert the(found, "unverified_runs", arm="roko_fixed").value == 1  # the unverified run only
    assert the(found, "false_greens", arm="roko_fixed").value == 0
    assert the(found, "false_green_rate", arm="roko_fixed").n == 1  # the passed run is the one reported pass
    assert the(found, "vs_rate", arm="roko_fixed").value == pytest.approx(1 / 3)
    assert metrics.false_greens(runs) == []

    # Arms without a Roko gate never get the count.
    assert not [m for m in metrics.arm_metrics(all_records(), EXPERIMENT, "cheap_direct")
                if m.metric == "already_satisfied_runs"]


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
    assert level["record_filter"] == ('experiment_id == "PILOT-T" and arm == "cheap_direct" and harness == "" and '
                                      'task.family != "PL" and task.ladder == 3 and execution.status not in '
                                      '["infra_error", "leak_suspected"] and task.is_honeypot == false')
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


def switch_model(path: Path) -> None:
    """The first record's attempt ran another model than the rest of its run: a single-model arm may not switch."""
    lines = path.read_text().splitlines()
    record = json.loads(lines[0])
    record["execution"]["attempts"][0].update(model_requested="glm-4.7", model_reported="glm-4.7")
    path.write_text("\n".join([json.dumps(record), *lines[1:]]) + "\n")


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
    (lambda b: switch_model(b / "run-a" / "records.jsonl"), "run run-a of arm cheap_direct requested 2 models"),
], ids=["foreign-run-id", "no-run-ids", "unlisted-false-green", "second-snapshot", "unledgered-attempt",
        "no-metrics", "copied-run", "switched-model"])
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


def no_attempt(record: dict) -> dict:
    """The record as a budget refusal leaves it: no attempt, so no model call and $0."""
    record["execution"]["attempts"] = []
    record["costs"].update(api_equiv_usd=0.0, billed_usd=0.0, without_cache_usd=0.0)
    assert validate.validate("run-record", record) == []
    return record


def test_an_arm_with_two_models_is_reported_per_model():
    # S09 §4.2: cheap_direct runs gpt-oss-120b on seeds 1-3 and the pool's best cheap model on seeds 1-2, in one
    # experiment. The two share (task, seed) pairs, yet neither is a repeat, and no metric pools them.
    oss = [run_record("F4-l1-0001", 1, run_id="run-oss"), run_record("F4-l1-0001", 2, run_id="run-oss"),
           run_record("F4-l1-0001", 3, run_id="run-oss", label=0, visible=True),  # a false green
           run_record("F1-l3-0003", 1, run_id="run-oss"), run_record("F1-l3-0003", 2, run_id="run-oss", label=0),
           run_record("F1-l3-0003", 3, run_id="run-oss")]
    glm = [run_record("F4-l1-0001", 1, run_id="run-glm", model="glm-4.7", cost=0.05),
           run_record("F4-l1-0001", 2, run_id="run-glm", model="glm-4.7", cost=0.05),
           run_record("F1-l3-0003", 1, run_id="run-glm", model="glm-4.7", cost=0.05),
           no_attempt(run_record("F1-l3-0003", 2, run_id="run-glm", label=0, status="aborted_cap"))]
    records = [*oss, *glm]
    metrics.check_unique(records)
    assert metrics.cells(records) == [("cheap_direct", "glm-4.7", ""), ("cheap_direct", "gpt-oss-120b", "")]
    written, found = report.build(records, EXPERIMENT, ks=(2,), analysis_commit="abc", computed_at="now")

    def get(metric: str, model: str, cost_basis: str | None = None) -> metrics.Metric:
        [match] = [m for m in found if (m.metric, m.model, m.cell, m.cost_basis) == (metric, model, "all", cost_basis)]
        return match

    # gpt-oss-120b: 2/3 on both tasks; glm-4.7: 2/2 and 1/2, the refused run staying in its model's cell with VS = 0.
    assert get("vs_rate", "gpt-oss-120b").value == pytest.approx(2 / 3) and get("vs_rate", "gpt-oss-120b").n == 6
    assert get("vs_rate", "glm-4.7").value == pytest.approx(0.75) and get("vs_rate", "glm-4.7").n == 4
    assert get("pass_hat_2", "gpt-oss-120b").value == pytest.approx(1 / 3)  # C(2,2)/C(3,2) on each task
    assert get("pass_hat_2", "glm-4.7").value == pytest.approx(0.5)
    assert get("usd_per_vs", "gpt-oss-120b", "api_equiv_usd").value == pytest.approx(6 * 0.02 / 4)
    assert get("usd_per_vs", "glm-4.7", "api_equiv_usd").value == pytest.approx(3 * 0.05 / 3)
    assert get("cap_censored_runs", "glm-4.7").value == 1 and get("cap_censored_runs", "gpt-oss-120b").value == 0
    assert get("false_greens", "gpt-oss-120b").value == 1 and get("false_greens", "glm-4.7").value == 0
    for metric in found:  # every number comes from one model's runs
        assert {row["run_id"] for row in metric.cut.rows} == {"run-oss" if metric.model == "gpt-oss-120b" else
                                                              "run-glm"}
    for row in written["records"]:
        model = "glm-4.7" if row["run_ids"] == ["run-glm"] else "gpt-oss-120b"
        assert row["run_ids"] in (["run-glm"], ["run-oss"]) and row["arms"] == ["cheap_direct"]
        assert row["record_filter"].startswith(f'experiment_id == "{EXPERIMENT}" and arm == "cheap_direct" and '
                                               f'model == "{model}" and harness == "" and task.family != "PL"')
        assert validate.validate("metric-record", row) == []
    [green] = written["false_greens"]
    assert (green["model"], green["run_id"], green["instance_id"], green["seed"]) == (
        "gpt-oss-120b", "run-oss", "F4-l1-0001", 3)
    printed = report.render(written, found)
    assert "cheap_direct (glm-4.7): 4 runs in run-glm" in printed
    assert "cheap_direct (gpt-oss-120b): 6 runs in run-oss" in printed
    assert "- cheap_direct (gpt-oss-120b) F4-l1-0001 seed 3 (completed): run run-oss," in printed
    with pytest.raises(metrics.MetricsError, match=r"its cells are cheap_direct \(glm-4.7\), cheap_direct "
                                                   r"\(gpt-oss-120b\), not cheap_direct$"):
        metrics.arm_metrics(records, EXPERIMENT, "cheap_direct")  # the pooled cell is refused

    # A run of a single-model arm may not switch models, and a model-less run cannot join one of two models.
    switched = run_record("F4-l1-0001", 3, run_id="run-glm", model="glm-4.7")
    switched["execution"]["attempts"].append(dict(switched["execution"]["attempts"][0], model_requested="gpt-oss-120b"))
    with pytest.raises(metrics.MetricsError, match="run run-glm of arm cheap_direct requested 2 models"):
        metrics.check_unique([*records, switched])
    orphan = no_attempt(run_record("F4-l3-0002", 1, run_id="run-z", label=0, status="aborted_cap"))
    with pytest.raises(metrics.MetricsError, match="run run-z of arm cheap_direct made no model call"):
        metrics.check_unique([*records, orphan])
    # A routed arm switches models by design, so its cell stays the arm; so does an arm that ran one model.
    routed = run_record("F4-l1-0001", run_id="run-full", arm="roko_full", verdict="passed")
    routed["execution"]["attempts"].append(dict(routed["execution"]["attempts"][0], model_requested="glm-4.7"))
    assert ("roko_full", None, "") in metrics.cells([*records, routed])
    assert metrics.cells(all_records()) == [("cheap_direct", None, ""), ("fd_api", None, ""), ("roko_fixed", None, "")]
    assert all("model" not in item for item in metrics.false_greens(all_records()))


def test_cells_separates_harnesses_sharing_one_arm_id():
    # bug-40de03 (3317): cheap_direct_msa's mini-swe-agent runs share the nominal arm id "cheap_direct" with the
    # bash-only loop's arm (S08 §4.9 decision 3: billed/capped together, deliberately not distinguished by id), so
    # only provenance.network_policy.harness tells them apart. cells()/arm_metrics() must not pool them.
    bash = [run_record("F4-l1-0001", 1, run_id="run-bash"), run_record("F4-l1-0001", 2, run_id="run-bash"),
            run_record("F4-l1-0001", 3, run_id="run-bash", label=0)]
    msa = [run_record("F4-l1-0001", 1, run_id="run-msa", harness="mini-swe-agent"),
           run_record("F4-l1-0001", 2, run_id="run-msa", harness="mini-swe-agent")]
    records = [*bash, *msa]
    metrics.check_unique(records)  # same instance and seeds 1-2 in both harnesses, yet neither is a duplicate
    assert metrics.cells(records) == [("cheap_direct", None, ""), ("cheap_direct", None, "mini-swe-agent")]

    def vs_rate(found: list[metrics.Metric]) -> metrics.Metric:
        [match] = [m for m in found if (m.metric, m.cell) == ("vs_rate", "all")]
        return match

    bash_only = metrics.arm_metrics(records, EXPERIMENT, "cheap_direct", harness="")
    msa_only = metrics.arm_metrics(records, EXPERIMENT, "cheap_direct", harness="mini-swe-agent")
    assert vs_rate(bash_only).value == pytest.approx(2 / 3) and vs_rate(bash_only).n == 3
    assert vs_rate(msa_only).value == pytest.approx(1.0) and vs_rate(msa_only).n == 2
    for metric in bash_only:
        assert {row["run_id"] for row in metric.cut.rows} == {"run-bash"}
    for metric in msa_only:
        assert {row["run_id"] for row in metric.cut.rows} == {"run-msa"}
    # harness=None (the default) still pools every harness of the arm, matching behavior before bug-40de03.
    assert vs_rate(metrics.arm_metrics(records, EXPERIMENT, "cheap_direct")).n == 5
    assert metrics.cell_name("cheap_direct", None, "mini-swe-agent") == "cheap_direct [mini-swe-agent]"
    assert metrics.cell_name("cheap_direct", None, "") == "cheap_direct"
    printed = report.render(*report.build(records, EXPERIMENT, ks=(2,), analysis_commit="abc", computed_at="now"))
    assert "cheap_direct: 3 runs in run-bash" in printed
    assert "cheap_direct [mini-swe-agent]: 2 runs in run-msa" in printed


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


def test_plan_level_false_green_matches_hidden_suite_failure():
    """App. D.12: a plan-level false green is specifically a hidden-suite failure, not any VF = 0 while the
    whole-plan gate passed. VF = 0 for three independent reasons; only the first is a false green (gap-cfe656)."""
    rows = [
        run_record("PL01-0001", run_id="run-p", arm="roko_plan", label=0, verdict="passed", visible=True),
        # hidden suite failed (visible_clean and integrity otherwise clean): a plan-level false green.
        run_record("PL02-0001", run_id="run-p", arm="roko_plan", label=0, verdict="passed", visible=False,
                   checks={"hidden": 1}),
        # the base repo's visible tests regressed; the hidden suite itself passed: not a false green.
        run_record("PL03-0001", run_id="run-p", arm="roko_plan", label=0, verdict="passed", visible=True,
                   checks={"hidden": 1, "integrity": 0}),
        # S05's A1 tamper check tripped; the hidden suite itself passed: not a false green.
    ]
    section, _ = metrics.plan_slice(rows, EXPERIMENT)
    assert section["plan_level_false_greens"] == ["roko_plan:PL01-0001"]
    assert section["plan_level_vf0_gate_passed"] == ["roko_plan:PL01-0001", "roko_plan:PL02-0001",
                                                      "roko_plan:PL03-0001"]


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


# --- the pilot page (gap-d9e9fe) ---------------------------------------------------------------------------------

PILOT_TASKS = ("F1-l1-0001", "F1-l1-0002", "F4-l1-0001", "F4-l1-0002", "F1-l3-0001", "F4-l3-0001")
# cheap_direct's runs on the six tasks x seeds 1-3; anything not listed is a VS = 1 run whose visible checks passed.
CHEAP = {("F1-l1-0002", 2): {"label": 0},
         ("F4-l1-0001", 3): {"label": 0, "visible": True},  # a false green
         ("F4-l1-0002", 1): {"unknown": True, "visible": True},  # unknown: 0 in the headline, 1 in the bound
         ("F1-l3-0001", 1): {"label": 0}, ("F1-l3-0001", 2): {"label": 0, "status": "aborted_cap"},
         ("F1-l3-0001", 3): {"label": 0, "status": "infra_error"},  # excluded and counted
         ("F4-l3-0001", 1): {"label": 0, "visible": True}, ("F4-l3-0001", 2): {"label": 0, "visible": True},
         ("F4-l3-0001", 3): {"label": 0}}
INTERVAL = re.compile(r"^[+-]?\$?\d+\.\d+ \[[+-]?\$?\d+\.\d+, ([+-]?\$?\d+\.\d+\]|∞\))[†‡]*")
RUN_LINK = re.compile(r"\[([A-Za-z0-9._-]+)\]\(\1/\)")


def pilot_record(experiment: str, instance: str, seed: int, **kw) -> dict:
    record = run_record(instance, seed, **kw)
    record.update(experiment_id=experiment, record_id=f"sha256:{experiment}/{record['run_id']}/{instance}/{seed}")
    return record


def pilot_manifest(directory: Path, experiment: str, blocks: list[tuple[str, str, str]]) -> None:
    """A vb.experiment/1 manifest: each block (id, arm file, model) on PILOT_TASKS x seeds 1-3."""
    tables = "".join(f'\n[[block]]\nid = "{block}"\nstream = "pilot"\narm = "{arm}"\nmodel = "{model}"\nseeds = "1-3"\n'
                     f'line = "BL0"\ninstances = {json.dumps(list(PILOT_TASKS))}\n' for block, arm, model in blocks)
    (directory / f"{experiment.lower()}.toml").write_text(
        f'schema_version = "vb.experiment/1"\nid = "{experiment}"\nrequires_lock = false\nrequires_live = []\n'
        f'order = {{ kind = "as_listed" }}\n{tables}')


@pytest.fixture
def pilot_sources(tmp_path: Path) -> dict:
    """Two summary bundles on one suite, as `vb report --bundle` writes them: PILOT-T-A's cheap_direct (one run,
    seeds 1-3) and PILOT-T-B's fd_claude (a run per seed); PILOT-T-B's manifest also plans roko_fixed, which has no
    run. PILOT-T-A's bundle holds a verifier-CI record covering five of the six tasks."""
    results, experiments = tmp_path / "results", tmp_path / "experiments"
    experiments.mkdir()
    cheap = [pilot_record("PILOT-T-A", instance, seed, run_id="cd-1", **CHEAP.get((instance, seed), {}))
             for instance in PILOT_TASKS for seed in (1, 2, 3)]
    write_run(results / "PILOT-T-A", cheap)
    for seed in (1, 2, 3):
        claude = [pilot_record("PILOT-T-B", instance, seed, run_id=f"fc-s{seed}", arm="fd_claude", cost=0.3,
                               billed=False, vendor=0.31 if seed == 1 else 0.3, model="claude-opus-5-5",
                               label=0 if (instance, seed) == ("F4-l3-0001", 2) else 1) for instance in PILOT_TASKS]
        write_run(results / "PILOT-T-B", claude, model="claude-opus-5-5")
    pilot_manifest(experiments, "PILOT-T-A", [("cheap-direct", "cheap_direct", "gpt-oss-120b")])
    pilot_manifest(experiments, "PILOT-T-B", [("roko-fixed", "roko_fixed", "gpt-oss-120b"),
                                              ("fd-claude", "fd_claude", "claude-opus-5-5")])
    bundles = {}
    for experiment in ("PILOT-T-A", "PILOT-T-B"):
        bundles[experiment] = tmp_path / experiment.lower()
        assert report.main(["--experiment", experiment, "--results", str(results), "--bundle",
                            str(bundles[experiment]), "--out", str(tmp_path / f"{experiment}.json")]) == 0
    cells = [{"cell": task, "family": task[:2].lower(), "latent": "v1", "level": int(task[4]), "seed": 1,
              "problems": [], "errors": []} for task in PILOT_TASKS[:5]]
    (bundles["PILOT-T-A"] / "verifier-ci.json").write_text(json.dumps(
        {"schema_version": "vb.verifier_ci/1", "cells": cells, "green": 5, "secret": "sha256:fixture",
         "levels": [1, 3], "seeds": [1, 2], "latents": ["v1"], "seconds": 1.5}))
    budget = budget_file(tmp_path / "budget.toml", {"BL0": 10, "BL8": 8}, "\n[programme]\nstop_usd = 400\n")
    return {"tmp": tmp_path, "results": results, "experiments": experiments, "bundles": bundles, "budget": budget,
            "out": tmp_path / "pilot"}


def make_pilot(sources: dict, *extra: str) -> tuple[str, dict]:
    argv = ["--pilot", *map(str, sources["bundles"].values()), "--bundle", str(sources["out"]), "--experiments",
            str(sources["experiments"]), "--budget", str(sources["budget"]), "--b", "200", *extra]
    assert report.main(argv) == 0
    return (sources["out"] / "REPORT.md").read_text(), json.loads((sources["out"] / "metrics.json").read_text())


def page_tables(page: str) -> dict[str, list[dict[str, str]]]:
    """Each markdown table on the page under its section heading, as rows of {column: cell}."""
    tables: dict[str, list[dict[str, str]]] = {}
    section, header = "", None
    for line in page.splitlines():
        if line.startswith("## "):
            section, header = line[3:], None
        elif line.startswith("|") and not line.startswith("|---"):
            cells = [cell.strip() for cell in re.split(r"(?<!\\)\|", line)[1:-1]]
            if header is None:
                header = cells
                tables.setdefault(section, [])
            else:
                tables[section].append(dict(zip(header, cells, strict=True)))
        elif not line.startswith("|"):
            header = None if not line.strip() else header
    return tables


def test_pilot_page_has_intervals_and_run_ids(pilot_sources, capsys):
    page, doc = make_pilot(pilot_sources)
    out = pilot_sources["out"]
    runs = {path.name for path in out.iterdir() if path.is_dir()}
    assert runs == {"cd-1", "fc-s1", "fc-s2", "fc-s3"} and (out / "verifier-ci.json").is_file()
    assert "**pilot, descriptive: two Python families (F1, F4), 6 tasks × 3 seeds, not pre-registered.**" in page

    # Every estimate in the arm, level and contrast tables shows its interval, and every row its runs.
    tables = page_tables(page)
    estimates = {"VS rate", "VS, unknown = 1", "pass^3", "$/VS, U′ (BCa)", "$/VS, R (BCa)",
                 "\\|U′ − R\\| / R (BCa)", "false-green rate", "all levels", "ℓ1", "ℓ3"}
    shown = 0
    for section in ("Arms, all levels", "By level ℓ", "Paired contrasts"):
        assert tables[section], section
        for row in tables[section]:
            pending = any("pending" in cell for cell in row.values())
            for column, cell in row.items():
                if column in estimates and re.match(r"[+-]?\$?\d", cell):
                    assert INTERVAL.match(cell), f"{section}: {column} = {cell!r} has no interval"
                    shown += 1
            linked = RUN_LINK.findall(row["runs"])
            assert pending or linked, f"{section}: a row without run ids: {row}"
            assert set(linked) <= runs
    assert shown >= 30  # 12 in the arm table, 17 by level, 8 contrasts
    arms = {row["arm"]: row for row in tables["Arms, all levels"]}
    assert arms["**cheap_direct**"]["VS rate"].startswith("0.50 [")  # (1 + 2/3 + 2/3 + 2/3 + 0 + 0) / 6
    assert arms["**cheap_direct**"]["runs kept / planned"] == "17 / 18"  # the infra_error run is excluded
    assert arms["**cheap_direct**"]["cap-censored · infra_error · leak_suspected"].startswith("1 · 1 · 0: ")
    assert "F1-l3-0001 seed 3 infra_error in [cd-1](cd-1/)" in arms["**cheap_direct**"][
        "cap-censored · infra_error · leak_suspected"]
    assert arms["**cheap_direct**"]["VS, unknown = 1"].startswith("0.56 [")  # F4-l1-0002 is then 3 of 3
    assert INTERVAL.match(arms["**fd_claude**"]["$/VS, R (BCa)"])
    assert arms["**fd_claude**"]["spend, U′ / billed"] == "$5.4000 / $0.0000"
    assert arms["**roko_fixed**"]["runs kept / planned"] == "0 / 18"
    assert arms["**roko_fixed**"]["VS rate"] == "pending (seeds 1, 2, 3)"

    # The pending arm and its seeds; the cost ratios against fd_claude, pending for roko_fixed.
    assert "`PILOT-T-B` block `roko-fixed` (roko_fixed, gpt-oss-120b): **pending**, 0 of 18 planned runs; seeds 1, 2, 3." \
        in page
    contrasts = {row["contrast"]: row for row in tables["Paired contrasts"]}
    assert INTERVAL.match(contrasts["$/VS ratio, cheap_direct ÷ fd_claude (BCa)"]["all levels"])
    assert contrasts["$/VS ratio, roko_fixed ÷ fd_claude (BCa)"]["all levels"] == "pending: roko_fixed pending"
    assert contrasts["VS rate, roko_fixed − cheap_direct"]["all levels"] == "pending: roko_fixed pending"

    # The false greens, F4's first, each with its run and record; the verifier-CI record; the open decision; D3.
    listed = [(item["instance_id"], item["seed"], item["run_id"]) for item in doc["false_greens"]]
    assert sorted(listed) == [("F4-l1-0001", 3, "cd-1"), ("F4-l1-0002", 1, "cd-1"), ("F4-l3-0001", 1, "cd-1"),
                              ("F4-l3-0001", 2, "cd-1")]
    assert page.index("**F4: 4 false greens**") < page.index("## Verifier CI")
    for instance, seed, run_id in listed:
        assert f"| cheap_direct | {instance} | {instance[4]} | {seed} | completed | [{run_id}]({run_id}/) |" in page
    assert "green cells F1 3/3, F4 2/2" in page and "Tasks with no CI cell: F4-l3-0001." in page
    assert "dec-3d5714" in page and "over-covers" in page
    assert "D3's bar is shown for context only; the pilot does not test H1." in page and "Y = 0.3" in page

    # metrics.json: every number is a valid MetricRecord with run ids in the bundle; every estimate has its interval.
    assert doc["experiment_id"] == pilot_page.PILOT_ID and doc["experiments"] == ["PILOT-T-A", "PILOT-T-B"]
    estimate = re.compile(r"^(vs_rate|pass_hat_3|usd_per_vs|false_green_rate|cost_gap_ur)(_unknown_as_1|_vendor)?$|"
                          r"_(diff|ratio)$")
    for row in doc["records"]:
        assert validate.validate("metric-record", row) == [] and set(row["run_ids"]) <= runs
        if estimate.search(row["metric"]) and row["value"] is not None:
            assert row["ci_method"].startswith("paired_bootstrap_") and len(row["ci"]) == 2, row
            assert row["ci"][0] <= row["value"] <= row["ci"][1] or row["metric"].startswith("pass_hat"), row
    ratio = next(row for row in doc["records"] if row["metric"] == "usd_per_vs_ratio" and "ladder" not in row)
    assert ratio["experiment_id"] == pilot_page.PILOT_ID and ratio["arms"] == ["cheap_direct", "fd_claude"]
    assert ratio["run_ids"] == ["cd-1", "fc-s1", "fc-s2", "fc-s3"] and ratio["ci_method"] == "paired_bootstrap_bca"
    capsys.readouterr()
    assert report.main(["--check", str(out), "--budget", str(pilot_sources["budget"])]) == 0, capsys.readouterr().out


def test_pilot_bootstrap_scores_each_task_draw_apart():
    """A replicate that draws one task twice scores it as two tasks: pass^3 per draw, never pooled by task id."""
    cell = pilot_page.Cell("PILOT-T-A", "cheap_direct", None, "")
    rows = [pilot_record("PILOT-T-A", "F4-l1-0001", seed, run_id="cd-1", label=int(seed != 3)) for seed in (1, 2, 3)]
    units = pilot_page._units({cell: rows})
    sizes = pilot_page._sizes(units)
    s1, s2, s3 = units
    bag = [s1, s1, s1, s3, s3, s2]  # the task drawn twice: seeds (1, 1, 1), then (3, 3, 2)
    draws = pilot_page._draws(bag, sizes)
    assert [[unit["seed"] for unit in draw] for draw in draws] == [[1, 1, 1], [3, 3, 2]]
    assert pilot_page.pass_k_of(cell, 3)(draws) == 0.5  # 1 and 0; pooled by task id it would be C(4,3)/C(6,3)
    assert pilot_page.vs_rate_of(cell)(draws) == pytest.approx(2 / 3)  # (3/3 + 1/3) / 2
    with pytest.raises(pilot_page.PilotError, match="not whole task draws"):
        pilot_page._draws([s1, s2], sizes)
    # On the data the estimate is passk's own; every replicate sees whole draws of three seeds.
    got = pilot_page.interval(units, pilot_page.pass_k_of(cell, 3), b=300, seed=1)
    assert got.estimate == 0.0 and got.method == "percentile" and got.low == 0.0 and got.high <= 1.0
    # A ratio with no VS in some replicates: those count as unbounded and are counted.
    usd = pilot_page.interval(units, pilot_page.usd_per_vs_of(cell), ratio=True, undefined=pilot_page.UNBOUNDED,
                              b=2000, seed=1)
    assert usd.estimate == pytest.approx(0.02 * 3 / 2) and usd.undefined > 0 and usd.fallback


def test_pilot_refuses_two_suites_and_lists_the_superseded_bundle(pilot_sources, capsys):
    """An earlier pilot on another suite (Pilot A before its re-tune) is refused, and listed when superseded."""
    old = pilot_sources["tmp"] / "pilot-t-old"
    records = [pilot_record("PILOT-T-OLD", instance, seed, run_id="cd-old", **CHEAP.get((instance, seed), {}))
               for instance in PILOT_TASKS for seed in (1, 2, 3)]
    for record in records:
        record["suite"] = {**record["suite"], "hash": "sha256:older"}
    write_run(pilot_sources["results"] / "PILOT-T-OLD", records)
    assert report.main(["--experiment", "PILOT-T-OLD", "--results", str(pilot_sources["results"]), "--bundle",
                        str(old), "--out", str(pilot_sources["tmp"] / "old.json")]) == 0
    capsys.readouterr()
    argv = ["--pilot", str(pilot_sources["bundles"]["PILOT-T-B"]), str(old), "--bundle", str(pilot_sources["out"]),
            "--experiments", str(pilot_sources["experiments"]), "--budget", str(pilot_sources["budget"]), "--b", "50"]
    assert report.main(argv) == 1
    assert "the bundles ran 2 suites" in capsys.readouterr().err and not pilot_sources["out"].exists()

    # Named with --superseded, it is listed with its runs and suite, and no number uses it; through `vb report`. A
    # note longer than a file name, with a slash in it, is a note, not a path.
    note = "roko_fixed waits for its provider (a test note); see experiments/pilot_b.toml. " + "Still waiting. " * 30
    vb = report.VB_ROOT / "driver" / "vb.py"
    done = subprocess.run([sys.executable, str(vb), "report", "--pilot", *map(str, pilot_sources["bundles"].values()),
                           "--superseded", str(old), "--note", note,
                           "--bundle", str(pilot_sources["out"]), "--experiments", str(pilot_sources["experiments"]),
                           "--budget", str(pilot_sources["budget"]), "--b", "50"],
                          capture_output=True, text=True, timeout=120, check=False)
    assert done.returncode == 0, done.stderr
    page = (pilot_sources["out"] / "REPORT.md").read_text()
    assert "- **Not merged:** " in page and "suite `sha256:older`" in page and "it ran another suite" in page
    assert "cheap_direct × seeds 1, 2, 3 (18 runs: cd-old)" in page
    assert f"- {note}" in page and "--note 'roko_fixed waits for its provider" in page
    assert "cd-old" not in json.dumps(json.loads((pilot_sources["out"] / "metrics.json").read_text())["records"])


def test_check_holds_a_merged_pilot_bundle_to_its_experiments(pilot_sources, capsys):
    make_pilot(pilot_sources)
    out, budget = pilot_sources["out"], str(pilot_sources["budget"])
    capsys.readouterr()

    rewrite_json(out / "metrics.json", lambda doc: doc.update(experiments=["PILOT-T-A"]))
    assert report.main(["--check", str(out), "--budget", budget]) == 1
    printed = capsys.readouterr().out
    assert 'experiments ["PILOT-T-A"] are not the bundle\'s runs\' ["PILOT-T-A", "PILOT-T-B"]' in printed
    assert "experiment PILOT has no runs in the bundle" in printed  # without the merge, the contrasts' id is foreign

    rewrite_json(out / "metrics.json", lambda doc: doc.update(experiments=["PILOT-T-A", "PILOT-T-B"]))
    assert report.main(["--check", str(out), "--budget", budget]) == 0
    rewrite_json(out / "metrics.json", lambda doc: doc.pop("experiments"))
    assert report.main(["--check", str(out), "--budget", budget]) == 1
    assert "experiment PILOT has no runs in the bundle" in capsys.readouterr().out
