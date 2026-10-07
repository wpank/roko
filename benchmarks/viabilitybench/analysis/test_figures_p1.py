"""Dry run of the P1 figure and table scripts (F2-F5, F9; T3, T7, T9, T11) on synthetic MetricRecords (task 9513).

Every record is built from the schema's example by `figlib.synthetic_record`, says `simulated: true` and validates
against `vb.metric_record/1` once that flag is set aside. The numbers are random draws from a fixed seed, so they are
not results and never reach the paper. Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_figures_p1.py -q
"""

from __future__ import annotations

import json
import math
import random
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest

import fig_f2_pareto
import fig_f3_envelope
import fig_f4_passk
import fig_f5_spec_interaction
import fig_f9_routing
import figlib
import metrics
import tab_t3_headline
import tab_t7_envelope
import tab_t9_costs
import tab_t11_plan_slice

# script -> the panels (figures) or parts (tables) a full fixture must draw
SCRIPTS = {
    fig_f2_pareto: ("a", "b"),
    fig_f3_envelope: ("a", "b", "c", "d"),
    fig_f4_passk: ("a", "b", "c", "d"),
    fig_f5_spec_interaction: ("a", "b"),
    fig_f9_routing: ("a", "b", "c"),
    tab_t3_headline: ("T3", "footer"),
    tab_t7_envelope: ("T7",),
    tab_t9_costs: ("T9",),
    tab_t11_plan_slice: ("a", "b", "c"),
}
LOG1 = {"cheap_direct": ("gpt-oss-120b", "kimi-k2.6"), "roko_fixed": (None,), "fd_claude": (None,),
        "fd_claude_lite": (None,), "fd_codex": (None,)}
LIVE = {"roko_full": (None,), "fd_api": (None,)}
EXT = {"roko_full": (None,), "fd_claude": (None,), "cheap_direct": (None,)}
FAMILIES = ("F1", "F2", "F3", "F4", "F5", "F7")
POLICIES = ("H4-B0:cheapest", "H4-B0:best", "H4-B1", "H4-B2", "H4-B3", "lcb_aci@0.7", "lcb_aci@0.8", "lcb_aci@0.9",
            "cascade", "hybrid")


class Fixture:
    """Synthetic records with report.py's filters (`metrics.Clause`), drawn from one seeded generator."""

    def __init__(self, seed: int = 9513):
        self.rng = random.Random(seed)
        self.records: list[dict] = []

    def rate(self) -> tuple[float, list[float]]:
        value = round(self.rng.uniform(0.2, 0.92), 4)
        half = round(self.rng.uniform(0.02, 0.07), 4)
        return value, [round(max(0.0, value - half), 4), round(min(1.0, value + half), 4)]

    def usd(self, low: float = 0.01, high: float = 1.5) -> tuple[float, list[float]]:
        value = round(math.exp(self.rng.uniform(math.log(low), math.log(high))), 5)
        return value, [round(value * 0.8, 6), round(value * 1.25, 6)]

    def add(self, metric: str, value: float | None, ci: list[float] | None, *, arms: tuple[str, ...],
            experiment: str, model: str | None = None, level: int | None = None, family: str | None = None,
            extra: tuple = (), cost_basis: str | None = None, n: int = 360, seeds: tuple = (1, 2, 3)) -> dict:
        clauses = [metrics.Clause("experiment_id", "==", experiment)]
        clauses.append(metrics.Clause("arm", "==", arms[0]) if len(arms) == 1 else
                       metrics.Clause("arm", "in", list(arms)))
        if model:
            clauses.append(metrics.Clause("model", "==", model))
        clauses += [metrics.Clause("task.family", "!=", "PL"),
                    metrics.Clause("execution.status", "not in", list(metrics.EXCLUDED))]
        if family:
            clauses.append(metrics.Clause("task.family", "==", family))
        if level:
            clauses.append(metrics.Clause("task.ladder", "==", level))
        clauses += [metrics.Clause(field, "==", want) for field, want in extra]
        record = figlib.synthetic_record(metric, value, arms=sorted(arms), experiment_id=experiment, ci=ci, n=n,
                                         record_filter=" and ".join(map(str, clauses)), ladder=level,
                                         cost_basis=cost_basis, seeds=seeds)
        self.records.append(record)
        return record

    def build(self) -> list[dict]:
        cells = [(arm, model, "LOG1") for arm, models in LOG1.items() for model in models]
        cells += [(arm, model, "E-P1-live") for arm, models in LIVE.items() for model in models]
        for arm, model, experiment in cells:
            cut = {"arms": (arm,), "experiment": experiment, "model": model}
            for metric in ("vs_rate", "vs_rate_unknown_as_1", "visible_pass_rate", "outcome_sd"):
                self.add(metric, *self.rate(), **cut)
            self.add("usd_per_vs", *self.usd(), cost_basis="api_equiv_usd", **cut)
            self.add("spend_usd", self.usd(1, 40)[0], None, cost_basis="billed_usd", **cut)
            self.add("spend_usd", self.usd(1, 40)[0], None, cost_basis="api_equiv_usd", **cut)
            for metric in ("estimated_cost_runs", "unknown_cost_runs", "cap_censored_runs"):
                self.add(metric, self.rng.choice((0, 0, 1, 3)), None, **cut)
            for k in (1, 2, 3):
                self.add(f"pass_hat_{k}", *self.rate(), n=120, **cut)
                self.add(f"pass_hat_{k}", *self.rate(), n=30, extra=(("stream.id", figlib.PASS5_STREAM),), **cut)
            for k in (4, 5):
                self.add(f"pass_hat_{k}", *self.rate(), n=30, extra=(("stream.id", figlib.PASS5_STREAM),), **cut)
            self.add("cc_3", *self.rate(), n=120, **cut)
            for source in ("provider_usage", "cli_usage", "estimated", "mock", "unknown"):
                self.add("cost_source_runs", self.rng.randint(0, 120), None, extra=(("costs.source", source),), **cut)
            for metric in ("cost_gap_ur", "cost_gap_ur_median", "cost_gap_ur_p90", "ledger_export_gap", "meter_gap",
                           "cache_ttl_5m_delta", "usage_missing_share"):
                self.add(metric, round(self.rng.uniform(0, 0.08), 4), None, **cut)
            for level in range(1, 6):
                self.add("vs_rate", *self.rate(), level=level, n=72, **cut)
                self.add("usd_per_vs", *self.usd(), level=level, n=72, cost_basis="api_equiv_usd", **cut)
                self.add("pass_hat_3", *self.rate(), level=level, n=24, **cut)
                for family in FAMILIES:
                    self.add("vs_rate", *self.rate(), level=level, family=family, n=12, **cut)
                    self.add("pass_hat_3", *self.rate(), level=level, family=family, n=4, **cut)
            for task in range(1, 9):
                self.add("cost_cv", round(self.rng.uniform(0.05, 1.2), 4), None, n=3,
                         extra=(("task.instance_id", f"F1-l{task % 5 + 1}-{task:03d}"),), **cut)
            if arm in ("roko_full", "fd_claude") or model == "gpt-oss-120b":
                for quintile in range(1, 6):
                    self.add("vs_rate_by_irt_quintile", *self.rate(), n=72,
                             extra=(("irt.quintile", quintile),), **cut)
        pair = {"arms": ("roko_full", "fd_claude"), "experiment": "E-P1-live"}
        for level in range(1, 6):
            for metric in ("envelope_ratio_r", "envelope_ratio_r_level"):
                self.add(metric, *self._ratio(0.75, 1.1), level=level, **pair)
            for metric in ("envelope_ratio_c", "envelope_ratio_c_level"):
                self.add(metric, *self._ratio(0.05, 0.5), level=level, cost_basis="api_equiv_usd", **pair)
        self.add("envelope_ratio_r", *self._ratio(0.75, 1.1), **pair)
        self.add("envelope_ratio_c", *self._ratio(0.05, 0.5), cost_basis="api_equiv_usd", **pair)
        self.add("envelope_level", 3, None, **pair)
        for hypothesis, decision in (("H1", 1), ("H2", 0)):
            self.add("holm_reject", decision, None, extra=(("hypothesis", hypothesis),), **pair)
        for arm, models in EXT.items():
            for model in models:
                cut = {"arms": (arm,), "experiment": "E-P1-ext", "model": model, "n": 60}
                self.add("vs_rate", *self.rate(), **cut)
                self.add("usd_per_vs", *self.usd(), cost_basis="api_equiv_usd", **cut)
                self.add("spend_usd", self.usd(1, 40)[0], None, cost_basis="billed_usd", **cut)
        self.add("envelope_ratio_r", *self._ratio(0.6, 1.0), experiment="E-P1-ext", arms=("roko_full", "fd_claude"))
        self.add("envelope_ratio_c", *self._ratio(0.05, 0.5), experiment="E-P1-ext", arms=("roko_full", "fd_claude"),
                 cost_basis="api_equiv_usd")
        self._h3()
        self._routing()
        return self.records

    def _ratio(self, low: float, high: float) -> tuple[float, list[float]]:
        value = round(self.rng.uniform(low, high), 4)
        return value, [round(value * 0.9, 4), round(value * 1.1, 4)]

    def _h3(self) -> None:
        stream = ("stream.id", figlib.H3_STREAM)
        for arm, model, variants in (("roko_fixed", "gpt-oss-120b", ("vague", "precise", "refined")),
                                     ("roko_fixed", "glm-4.7", ("vague", "precise")),
                                     ("fr_claude", None, ("vague", "precise"))):
            cut = {"arms": (arm,), "experiment": "LOG1", "model": model}
            for variant in variants:
                self.add("vs_rate", *self.rate(), n=96, extra=(stream, ("task.spec_variant", variant)), **cut)
                self.add("usd_per_vs", *self.usd(), n=96, cost_basis="api_equiv_usd",
                         extra=(stream, ("task.spec_variant", variant)), **cut)
                if variant != "refined":
                    for task in range(1, 9):
                        self.add("vs_rate", round(self.rng.choice((0, 0.5, 1)), 1), None, n=2,
                                 extra=(stream, ("task.instance_id", f"F2-l2-{task:03d}"),
                                        ("task.spec_variant", variant)), **cut)
        self.add("interaction_rd", round(self.rng.uniform(-0.1, 0.3), 4), [-0.05, 0.31], n=48,
                 arms=("roko_fixed", "fr_claude"), experiment="LOG1", extra=(stream,))

    def _routing(self) -> None:
        for policy in POLICIES:
            cut = {"arms": (policy,), "experiment": "R-H4"}
            self.add("vs_rate", *self.rate(), **cut)
            self.add("usd_per_vs", *self.usd(), cost_basis="api_equiv_usd", **cut)
            regret = 0.0
            for position in range(20, 121, 20):
                regret += round(self.rng.uniform(0, 3), 3)
                self.add("regret_cum", round(regret, 3), [round(regret * 0.8, 3), round(regret * 1.2, 3)],
                         extra=(("stream.position", position),), n=position, **cut)
            shares = [self.rng.uniform(0.1, 1) for _ in range(4)]
            total = sum(shares)
            rounded = [round(share / total, 4) for share in shares]
            rounded[-1] = round(1 - sum(rounded[:-1]), 4)
            for rung, share in enumerate(rounded[:3], 1):
                self.add("escalation_share", share, None, extra=(("rung", rung),), **cut)
            self.add("unresolved_share", rounded[3], None, **cut)
        # The live roko_full cell names the policy it ran, so F9 can draw that policy's replay interval behind it.
        live = next(record for record in self.records if record["metric"] == "vs_rate"
                    and record["experiment_id"] == "E-P1-live" and record["arms"] == ["roko_full"]
                    and "ladder" not in record and '"task.' not in record["record_filter"].split("task.family")[-1])
        live["record_filter"] += f" and {metrics.Clause('policy', '==', 'lcb_aci@0.8')}"


def plan_slice() -> tuple[list[dict], dict]:
    """E-PL records and the plan_slice section they name, as report.py writes them."""
    fixture = Fixture(seed=9511)
    runs = {"roko_plan": "dry-run-pl-roko", "fd_claude": "dry-run-pl-claude"}
    for arm, run_id in runs.items():
        cut = {"arms": (arm,), "experiment": "E-PL", "n": 6, "seeds": (1,)}
        records = [fixture.add("pl_verified_features", 4, None, **cut),
                   fixture.add("pl_vf_rate", 4 / 6, list(metrics.clopper_pearson(4, 6)), **cut),
                   fixture.add("pl_cpf_usd", *fixture.usd(0.5, 5), cost_basis="api_equiv_usd", **cut)]
        spans = sorted(round(fixture.rng.uniform(600, 3600)) for _ in range(3))
        for name, span in zip(("min", "median", "max"), spans):
            records.append(fixture.add(f"pl_makespan_{name}_s", span, None, **cut))
        if arm == "roko_plan":
            records += [fixture.add("pl_planner_share", 0.31, None, cost_basis="api_equiv_usd", **cut),
                        fixture.add("pl_realized_parallelism_median", 2.4, None, **cut),
                        fixture.add("pl_tasks_escalated_share", 0.2, None, **cut),
                        fixture.add("pl_integrations_rejected", 1, None, **cut)]
        else:
            records.append(fixture.add("pl_cpf_vendor_usd", *fixture.usd(0.5, 5), cost_basis="api_equiv_usd", **cut))
        for record in records:
            record["run_ids"] = [run_id]
    for name in ("pl_cpf_ratio", "pl_makespan_median_ratio"):
        record = fixture.add(name, round(fixture.rng.uniform(0.2, 1.5), 4), None, arms=("roko_plan", "fd_claude"),
                             experiment="E-PL", n=6, seeds=(1,))
        record["run_ids"] = sorted(runs.values())
    table = []
    for index in range(6):
        cells = {arm: {"vf": int(index % 3 != 0), "unknown": index == 5, "status": "completed",
                       "cost_usd": round(fixture.rng.uniform(0.2, 3), 4),
                       "vendor_usd": round(fixture.rng.uniform(0.2, 3), 4) if arm == "fd_claude" else None,
                       "makespan_s": round(fixture.rng.uniform(600, 3600)), "queue_wait_s": None, "run_id": run_id,
                       "record_id": f"sha256:dry-run/{arm}/{index}"} for arm, run_id in runs.items()}
        table.append({"feature": f"PL-{index + 1:02d}", "seed": 1, "arms": cells})
    section = {"label": "PL (exploratory, one seed, 6 features)", "features": [row["feature"] for row in table],
               "table": table, "arms": {arm: {} for arm in runs}, "ratios": {},
               "discordant": {"roko_plan": ["PL-02"], "fd_claude": []}, "both_verified": ["PL-03", "PL-05"],
               "plan_level_false_greens": ["roko_plan:PL-04"], "not_recorded": "queue waits (roko_plan); queue waits "
               "(fd_claude); total work / critical path (the plan's task graph is not in the run records)"}
    return fixture.records, section


@pytest.fixture(scope="module")
def records() -> list[dict]:
    p1 = Fixture().build()
    pl, _ = plan_slice()
    return p1 + pl


def write_fixture(path: Path, records: list[dict], *, simulated: bool | None = True) -> Path:
    """A vb.metrics/1 document holding the records and the plan_slice section, as report.py writes metrics.json."""
    _, section = plan_slice()
    doc = {"schema_version": figlib.METRICS_VERSION, "experiment_id": "DRY-RUN", "records": records,
           "plan_slice": section}
    if simulated is not None:
        doc["simulated"] = simulated
    path.write_text(json.dumps(doc), encoding="utf-8")
    return path


def outputs(out: Path, module) -> tuple[Path, dict]:
    stem = module.SPEC.stem
    body = out / f"{stem}{'.svg' if module.SPEC.kind == 'figure' else '.md'}"
    return body, json.loads((out / f"{stem}.data.json").read_text(encoding="utf-8"))


def test_p1_figures_dry_run(tmp_path, records):
    """All nine render from the synthetic records; every sidecar value is a record's value, with its id."""
    source = write_fixture(tmp_path / "metrics.json", records)
    by_id = {figlib.canonical_id(record): record for record in records}
    for module, panels in SCRIPTS.items():
        out = tmp_path / module.SPEC.stem
        assert module.main([str(source), "--out", str(out), "--dry-run"]) == 0, module.SPEC.script
        body, sidecar = outputs(out, module)
        assert sidecar["schema_version"] == figlib.SIDECAR_VERSION and sidecar["dry_run"] is True
        assert sidecar["id"] == module.SPEC.id and sidecar["caption"].startswith(figlib.DRY_RUN_STAMP)
        assert sidecar["values"], module.SPEC.script
        for value in sidecar["values"]:
            record = by_id[value["record_id"]]  # the id is the record's canonical digest
            assert (value["metric"], value["value"], value["n"]) == (record["metric"], record["value"], record["n"])
            assert value["ci"] == record.get("ci") and value["ci_method"] == record["ci_method"]
        used = {value["record_id"] for value in sidecar["values"]}
        for derived in sidecar["derived"]:
            assert derived["from"] and set(derived["from"]) <= used, derived
        for row in sidecar["rows"]:
            assert set(row["named_by"]) <= used and all(row["run_id"] in by_id[rec]["run_ids"]
                                                        for rec in row["named_by"])
        drawn = {value["panel"] for value in sidecar["values"] + sidecar["rows"]}
        assert set(panels) <= drawn, (module.SPEC.script, sorted(drawn))
        text = body.read_text(encoding="utf-8")
        assert figlib.DRY_RUN_STAMP in text
        if module.SPEC.kind == "figure":
            root = ET.fromstring(text)
            assert root.get("data-dry-run") == "true"
            assert {g.get("data-panel") for g in root.iter("{http://www.w3.org/2000/svg}g")} >= set(panels)
        else:
            assert "| " in text and "\n: " in text  # pipe tables and a pandoc caption


def test_p1_figures_dry_run_is_refused_in_paper_mode(tmp_path, records):
    """A paper-mode run refuses the simulated fixture and writes nothing."""
    source = write_fixture(tmp_path / "metrics.json", records)
    for module in SCRIPTS:
        out = tmp_path / "paper" / module.SPEC.stem
        assert module.main([str(source), "--out", str(out)]) == 1, module.SPEC.script
        assert not out.exists()


def test_p1_figures_dry_run_refuses_unmarked_or_bad_records(tmp_path, records, capsys):
    """A dry run draws only records marked simulated, and still refuses empty run ids and P1 estimated labels."""
    unmarked = [dict(records[0])] + records[1:]
    del unmarked[0]["simulated"]
    no_runs = [dict(records[0], run_ids=[])] + records[1:]
    estimated = [dict(records[0], label_source="vs_estimated")] + records[1:]
    for name, rows, message in (("unmarked", unmarked, "does not say simulated: true"),
                                ("no-runs", no_runs, "no run_ids"),
                                ("estimated", estimated, "P1 numbers use vs_census labels only")):
        source = write_fixture(tmp_path / f"{name}.json", rows)
        assert fig_f2_pareto.main([str(source), "--out", str(tmp_path / name), "--dry-run"]) == 1
        assert message in capsys.readouterr().err
        assert not (tmp_path / name).exists()
    unmarked_doc = write_fixture(tmp_path / "document.json", records, simulated=None)
    assert tab_t3_headline.main([str(unmarked_doc), "--out", str(tmp_path / "document"), "--dry-run"]) == 1


def test_p1_figures_dry_run_needs_a_ci_on_every_mark(tmp_path, records, capsys):
    rows = [dict(record) for record in records]
    target = next(row for row in rows if row["metric"] == "vs_rate" and row["arms"] == ["fd_claude"]
                  and row["experiment_id"] == "LOG1" and "ladder" not in row
                  and "stream.id" not in row["record_filter"])
    del target["ci"]
    source = write_fixture(tmp_path / "metrics.json", rows)
    assert fig_f2_pareto.main([str(source), "--out", str(tmp_path / "out"), "--dry-run"]) == 1
    assert "has no ci" in capsys.readouterr().err


def test_p1_figures_dry_run_paper_mode_draws_real_records(tmp_path, records):
    """The same scripts in paper mode draw records that are not simulated, unstamped; T11 checks the suite hash."""
    real = [{key: value for key, value in record.items() if key != "simulated"} for record in records]
    bundle = tmp_path / "bundle"
    bundle.mkdir()
    source = write_fixture(bundle / "metrics.json", real, simulated=None)
    for run_id in sorted({run_id for record in real if record["experiment_id"] == "E-PL"
                          for run_id in record["run_ids"]}):
        (bundle / run_id).mkdir()
        (bundle / run_id / "manifest.json").write_text(json.dumps({"run_id": run_id, "suite": {"id": "plan_slice",
                                                                                               "hash": "sha256:s1"}}))
        (bundle / run_id / "records.jsonl").write_text(json.dumps({"record_id": f"{run_id}/1", "suite": {
            "id": "plan_slice", "hash": "sha256:s1"}}) + "\n")
    for module in SCRIPTS:
        out = tmp_path / "paper" / module.SPEC.stem
        assert module.main([str(source), "--out", str(out)]) == 0, module.SPEC.script
        body, sidecar = outputs(out, module)
        assert sidecar["dry_run"] is False and figlib.DRY_RUN_STAMP not in body.read_text(encoding="utf-8")
    assert "one hash, sha256:s1" in (tmp_path / "paper" / "t11-plan-slice" / "t11-plan-slice.md").read_text()
    records_file = bundle / "dry-run-pl-roko" / "records.jsonl"
    records_file.write_text(json.dumps({"record_id": "x", "suite": {"id": "plan_slice", "hash": "sha256:other"}}))
    assert tab_t11_plan_slice.main([str(source), "--out", str(tmp_path / "t11-bad")]) == 1


def test_p1_figures_dry_run_reads_report_filters():
    """parse_filter reads back exactly what metrics.Clause writes, so a cut is the filter that was applied."""
    clauses = [metrics.Clause("experiment_id", "==", "LOG1"), metrics.Clause("model", "==", "glm-4.7"),
               metrics.Clause("execution.status", "not in", ["infra_error", "leak_suspected"]),
               metrics.Clause("task.ladder", "==", 3), metrics.Clause("task.is_honeypot", "==", False),
               metrics.Clause("arm", "in", ["roko_plan", "fd_claude"]), metrics.Clause("note", "==", "a and b")]
    text = " and ".join(map(str, clauses))
    assert figlib.parse_filter(text) == tuple((c.field, c.op, c.value) for c in clauses)
    assert figlib.parse_filter(None) == ()
    with pytest.raises(figlib.FigureError):
        figlib.parse_filter("task.ladder => 3")


def test_p1_figures_find_a_record_whose_only_extra_clause_is_its_own_harness(tmp_path):
    """BASE_FIELDS must name every field report.py's own records carry unconditionally, or Inputs.where() treats a
    record as narrowed on a field the query never asked about and never returns it. harness (bug-40de03) is one:
    every record carries it, "" for the arm's own, usual one, so a plain pooled query must still find it."""
    record = figlib.synthetic_record("vs_rate", 0.5, arms=["roko_full"], experiment_id="LOG1",
                                     record_filter='experiment_id == "LOG1" and arm == "roko_full" and harness == '
                                                   '"" and task.family != "PL"')
    path = tmp_path / "metrics.jsonl"
    path.write_text(json.dumps(record) + "\n")
    inputs = figlib.load([path], dry_run=True, p1=False)
    found = inputs.where("vs_rate", experiment="LOG1", arm="roko_full")
    assert len(found) == 1 and found[0].series == "roko_full" and found[0].harness == ""


def test_p1_figures_dry_run_axes_and_ids():
    axis = figlib.log_axis([0.013, 0.4], "usd")
    assert (axis.lo, axis.hi) == (0.01, 1.0)
    assert [label for _, label in axis.ticks] == ["$0.01", "$0.02", "$0.05", "$0.1", "$0.2", "$0.5", "$1"]
    assert figlib.rate_axis("coverage", 0.8, 1.0, 0.05).note == "zoomed axis [0.8, 1]"
    assert figlib.rate_axis("VS rate").note == ""
    record = figlib.synthetic_record("vs_rate", 0.5, arms=["roko_full"], ci=[0.4, 0.6])
    assert figlib.canonical_id(record) == figlib.canonical_id(json.loads(json.dumps(record, indent=2)))
    assert figlib.canonical_id(record).startswith("sha256:") and len(figlib.canonical_id(record)) == 71
