#!/usr/bin/env python3
"""The gate pages: G0, the pilot's go/no-go (S09 §4.7 and E1a/E1b; 3309). G2's checks join this module (3348).

    gates.py G0 --experiment PILOT-A --experiment PILOT-B [--results DIR] [--runs DIR ...] [--verifier-ci FILE]
                [--reconcile FILE ...] [--sc2 FILE] [--runaway DIR] [--out DIR]

G0 decides whether the campaign may spend (LOG1's $152, 3346). Each of its checks is one function of the evidence,
which returns the check's value, its threshold, the run ids behind it, and a verdict: `pass`, `fail`, or `not
evaluated` when its evidence is missing. A missing file never reads as a pass. The page's verdict is GO when every
check passes, NO-GO when one fails, and INCOMPLETE when none fails and some were not evaluated.

**The evidence.**
- The run directories (`report.load_runs`): `--experiment ID` under the results root (`--results`, else
  `$VB_RESULTS`, else the driver's default), and `--runs DIR`, an experiment directory or a committed bundle. Their run
  records, ledger rows, manifests and metering-proxy logs.
- `--verifier-ci FILE`: `ci/verify_verifiers.py --json FILE` (`vb.verifier_ci/1`).
- `--reconcile FILE`, once per provider: `vb ledger reconcile --provider P --csv EXPORT --json > FILE`.
- `--sc2 FILE`, filled by hand (`vb.sc2/1`): the spot check of S08 SC2, one `[[item]]` per output checked, with its
  `run_id`, `instance_id`, `seed` and the checker's own `human_label` (0 or 1); the census label is read from the
  records. A `[direct_loop]` table records S08 decision 3: `choice` and `reason`.
- `--runaway DIR`: a synthetic runaway's run (or its experiment directory), whose agent never submits. Without it the
  check looks for a pilot run the runaway detector stopped.

**The checks** (`THRESHOLDS["G0"]`, one table, whose sha256 `g0.json` carries so the pre-registration lock (3341) can
hash it): verifier CI green for F1 and F4; the ledger within ±5% of each billed provider's usage export; a runaway
stopped by the caps; no canary hit; the SC2 spot check; the ladder bands (gpt-oss-120b on cheap_direct VS ≥ 0.6 at
level 1 and ≤ 0.3 at level 5, fd_claude VS ≥ 0.4 at level 5); each billed arm's measured $/task at most twice S09
§3's plan; U′ and R for every fd_claude run, with |ΣU′ − ΣR|/ΣR; the proxy's fault rates within 2 points of their
profiles (with at least `min_requests` requests per profile); every roko_fixed run's records ingested, with no model
mismatch outside `infra_error`; and the direct-loop choice. VS rates are `metrics.vs_rate` over the runs the report
keeps (`infra_error` and `leak_suspected` left out).

**The page.** `go-no-go.md` and `g0.json` (`vb.gate/1`) go to `--out`, by default beside the evidence: the parent of
the first `--runs` directory, else the results root. They are files, so `report.py --check` still accepts a bundle
that holds them. The exit status is 0 for GO, 1 otherwise, and 2 for a usage error.

API:
    THRESHOLDS; thresholds_sha256(gate) -> str
    Evidence; load_evidence(args) -> Evidence
    Check(id, title, verdict, value, threshold, run_ids, detail)
    g0(evidence) -> list[Check]; verdict(checks) -> str
    page(gate, checks, evidence) -> (g0_json: dict, markdown: str)
    main(argv) -> int
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import math
import os
import sys
import tomllib
from dataclasses import asdict, dataclass, field
from pathlib import Path

import metrics
import report

GATE_SCHEMA = "vb.gate/1"
SC2_SCHEMA = "vb.sc2/1"
PASS, FAIL, NOT_EVALUATED = "pass", "fail", "not evaluated"
RUNAWAY_REASONS = ("model_calls", "identical_calls", "usd")  # the runaway detector's stops (driver/caps.py)
THRESHOLDS: dict[str, dict[str, dict]] = {
    "G0": {
        "verifier_ci": {"families": ["f1", "f4"], "min_green": 1.0},
        "ledger_reconcile": {"providers": ["cerebras", "openai"], "tolerance": 0.05},
        "runaway_killed": {"reasons": list(RUNAWAY_REASONS), "max_calls": 30},
        "canary_hits": {"max": 0},
        "sc2_spot_check": {"min_checked": 20, "min_each_label": 10, "min_agree_share": 0.95},
        "ladder_cheap_l1": {"arm": "cheap_direct", "model": "gpt-oss-120b", "level": 1, "min_vs": 0.6},
        "ladder_cheap_l5": {"arm": "cheap_direct", "model": "gpt-oss-120b", "level": 5, "max_vs": 0.3},
        "ladder_frontier_l5": {"arm": "fd_claude", "model": None, "level": 5, "min_vs": 0.4},
        "cost_per_task": {"plan_usd": {"cheap_direct": 0.06, "fd_api": 0.30, "roko_fixed": 0.075}, "max_ratio": 2.0},
        "u_prime_and_r": {"arm": "fd_claude", "min_captured": 1.0},
        "fault_rates": {"max_abs_diff": 0.02, "min_requests": 20},
        "roko_ingest": {"arm": "roko_fixed"},
        "direct_loop_choice": {"choices": ["mini-loop", "mini-swe-agent"]},
    },
}


@dataclass
class Check:
    id: str
    title: str
    verdict: str  # PASS, FAIL or NOT_EVALUATED
    value: object
    threshold: str
    run_ids: list[str] = field(default_factory=list)
    detail: str = ""


@dataclass
class Evidence:
    runs: list[report.Run]
    verifier_ci: dict | None = None
    reconciles: dict[str, dict] = field(default_factory=dict)  # provider -> `vb ledger reconcile --json`
    sc2: dict | None = None
    runaway: list[report.Run] | None = None
    sources: dict[str, object] = field(default_factory=dict)  # what was read, for the page
    problems: list[str] = field(default_factory=list)  # evidence that could not be read

    @property
    def records(self) -> list[dict]:
        return [record for run in self.runs for _, record in run.records]

    @property
    def ledger(self) -> list[dict]:
        return [row for run in self.runs for _, row in run.ledger]


def thresholds_sha256(gate: str) -> str:
    text = json.dumps(THRESHOLDS[gate], sort_keys=True, separators=(",", ":"))
    return "sha256:" + hashlib.sha256(text.encode("utf-8")).hexdigest()


def g0(evidence: Evidence) -> list[Check]:
    """Every G0 check, in the order of S09 §4.7's table."""
    limits = THRESHOLDS["G0"]
    checks = [_verifier_ci(evidence, limits["verifier_ci"]), _ledger_reconcile(evidence, limits["ledger_reconcile"]),
              _runaway(evidence, limits["runaway_killed"]), _canaries(evidence, limits["canary_hits"]),
              _sc2(evidence, limits["sc2_spot_check"])]
    checks += [_ladder(evidence, name, limits[name])
               for name in ("ladder_cheap_l1", "ladder_cheap_l5", "ladder_frontier_l5")]
    checks += [_cost(evidence, arm, plan, limits["cost_per_task"]["max_ratio"])
               for arm, plan in limits["cost_per_task"]["plan_usd"].items()]
    checks += [_u_prime_and_r(evidence, limits["u_prime_and_r"]), _fault_rates(evidence, limits["fault_rates"]),
               _roko_ingest(evidence, limits["roko_ingest"]), _direct_loop(evidence, limits["direct_loop_choice"])]
    return checks


def verdict(checks: list[Check]) -> str:
    if any(check.verdict == FAIL for check in checks):
        return "NO-GO"
    return "GO" if all(check.verdict == PASS for check in checks) else "INCOMPLETE"


def page(gate: str, checks: list[Check], evidence: Evidence, *, experiments: list[str]) -> tuple[dict, str]:
    """The gate's JSON document and its Markdown page."""
    computed = dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
    outcome = verdict(checks)
    doc = {"schema_version": GATE_SCHEMA, "gate": gate, "experiments": experiments, "computed_at": computed,
           "analysis_commit": report.analysis_commit(), "verdict": outcome, "thresholds": THRESHOLDS[gate],
           "thresholds_sha256": thresholds_sha256(gate), "evidence": evidence.sources,
           "evidence_problems": evidence.problems, "checks": [asdict(check) for check in checks]}
    counts = {name: sum(check.verdict == name for check in checks) for name in (PASS, FAIL, NOT_EVALUATED)}
    lines = [f"# {gate} go/no-go: {outcome}", "",
             f"Experiments {', '.join(experiments) or '-'}; computed {computed} at {doc['analysis_commit']}; "
             f"thresholds {doc['thresholds_sha256']}. {counts[PASS]} pass, {counts[FAIL]} fail, "
             f"{counts[NOT_EVALUATED]} not evaluated. A check without its evidence is not evaluated, never passed.",
             "", "| check | value | threshold | verdict | run ids |", "|---|---|---|---|---|"]
    for check in checks:
        runs = ", ".join(check.run_ids[:6]) + (f" (+{len(check.run_ids) - 6})" if len(check.run_ids) > 6 else "")
        lines.append(f"| {check.title} | {_show(check.value)} | {check.threshold} | **{check.verdict}** | "
                     f"{runs or '-'} |")
    lines += ["", "## Details", ""]
    lines += [f"- **{check.title}** ({check.verdict}): {check.detail}" for check in checks if check.detail]
    lines += ["", "## Evidence", ""] + [f"- {name}: {_show(source)}" for name, source in evidence.sources.items()]
    lines += [f"- could not read: {problem}" for problem in evidence.problems]
    return doc, "\n".join(lines) + "\n"


def load_evidence(args: argparse.Namespace) -> Evidence:
    results = Path(args.results or os.environ.get("VB_RESULTS") or report.DEFAULT_RESULTS).expanduser()
    directories = [results / name for name in args.experiment or []] + [Path(path) for path in args.runs or []]
    if not directories:
        raise report.ReportError("name the pilot's runs: --experiment ID or --runs DIR")
    evidence = Evidence(runs=[])
    for directory in directories:
        evidence.runs += report.load_runs(directory)
    evidence.sources["runs"] = [str(directory) for directory in directories]
    evidence.problems += [problem for run in evidence.runs for problem in run.problems]
    if args.verifier_ci:
        evidence.verifier_ci = _json_file(Path(args.verifier_ci), evidence, "vb.verifier_ci/1")
        evidence.sources["verifier_ci"] = str(args.verifier_ci)
    for path in args.reconcile or []:
        doc = _json_file(Path(path), evidence)
        if doc is not None and isinstance(doc.get("provider"), str):
            evidence.reconciles[doc["provider"]] = doc
        elif doc is not None:
            evidence.problems.append(f"{path}: not the JSON of `vb ledger reconcile --json`")
    if args.reconcile:
        evidence.sources["reconcile"] = [str(path) for path in args.reconcile]
    if args.sc2:
        try:
            with Path(args.sc2).open("rb") as handle:
                doc = tomllib.load(handle)
            if doc.get("schema_version") != SC2_SCHEMA:
                raise ValueError(f"schema_version must be {SC2_SCHEMA!r}")
            evidence.sc2 = doc
        except (OSError, ValueError) as err:
            evidence.problems.append(f"{args.sc2}: {err}")
        evidence.sources["sc2"] = str(args.sc2)
    if args.runaway:
        path = Path(args.runaway)
        try:
            evidence.runaway = [report.load_run(path)] if (path / "manifest.json").is_file() else report.load_runs(path)
        except report.ReportError as err:
            evidence.problems.append(f"{path}: {err}")
        evidence.sources["runaway"] = str(path)
    return evidence


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="gates.py", description="The gate pages (S09 §4.7).", allow_abbrev=False)
    parser.add_argument("gate", choices=sorted(THRESHOLDS))
    parser.add_argument("--experiment", action="append", help="an experiment under the results root (repeatable)")
    parser.add_argument("--results", type=Path, help="default: $VB_RESULTS, then " + str(report.DEFAULT_RESULTS))
    parser.add_argument("--runs", action="append", help="an experiment directory or a bundle (repeatable)")
    parser.add_argument("--verifier-ci", type=Path, help="verify_verifiers.py --json output")
    parser.add_argument("--reconcile", action="append", type=Path,
                        help="vb ledger reconcile --json output, one per provider (repeatable)")
    parser.add_argument("--sc2", type=Path, help="the hand-filled spot check (vb.sc2/1)")
    parser.add_argument("--runaway", type=Path, help="a synthetic runaway's run or experiment directory")
    parser.add_argument("--out", type=Path, help="where go-no-go.md and g0.json go (default: beside the evidence)")
    args = parser.parse_args(argv)
    try:
        evidence = load_evidence(args)
    except (report.ReportError, OSError) as err:
        print(f"gates: {err}", file=sys.stderr)
        return 2
    checks = g0(evidence)
    doc, markdown = page(args.gate, checks, evidence, experiments=sorted(
        {record["experiment_id"] for record in evidence.records}))
    out = args.out or (Path(args.runs[0]).parent if args.runs else
                       Path(args.results or os.environ.get("VB_RESULTS") or report.DEFAULT_RESULTS).expanduser())
    out.mkdir(parents=True, exist_ok=True)
    name = args.gate.lower()
    (out / f"{name}.json").write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    (out / "go-no-go.md").write_text(markdown, encoding="utf-8")
    print(markdown)
    print(f"gates: {args.gate} {doc['verdict']}; {out / 'go-no-go.md'}", file=sys.stderr)
    return 0 if doc["verdict"] == "GO" else 1


# ---------------------------------------------------------------- the checks


def _verifier_ci(evidence: Evidence, limit: dict) -> Check:
    title, threshold = "Verifier CI, F1 and F4", f"{limit['min_green']:.0%} of cells green"
    doc = evidence.verifier_ci
    if doc is None:
        return Check("verifier_ci", title, NOT_EVALUATED, None, threshold,
                     detail="no verifier-CI JSON (--verifier-ci, from ci/verify_verifiers.py --json)")
    cells = doc.get("cells") if isinstance(doc.get("cells"), list) else []
    value, missing, red = {}, [], []
    for family in limit["families"]:
        mine = [cell for cell in cells if isinstance(cell, dict) and cell.get("family") == family]
        if not mine:
            missing.append(family)
            continue
        green = sum(1 for cell in mine if not cell.get("problems"))
        value[family] = f"{green}/{len(mine)}"
        if green < limit["min_green"] * len(mine):
            red += [str(cell.get("cell")) for cell in mine if cell.get("problems")]
    if red:
        return Check("verifier_ci", title, FAIL, value, threshold, detail=f"red cells: {', '.join(red[:10])}")
    if missing:
        return Check("verifier_ci", title, NOT_EVALUATED, value, threshold,
                     detail=f"the CI JSON has no cells of {', '.join(missing)}")
    return Check("verifier_ci", title, PASS, value, threshold, detail=f"secret {doc.get('secret')}")


def _ledger_reconcile(evidence: Evidence, limit: dict) -> Check:
    title, threshold = "Ledger against the providers' usage exports", f"every drift within ±{limit['tolerance']:.0%}"
    billed = {}  # provider -> run ids with billed rows
    for row in evidence.ledger:
        if row.get("billed") is not False and row.get("billed_usd") and row["provider"] in limit["providers"]:
            billed.setdefault(row["provider"], set()).add(row["run_id"])
    if not billed:
        return Check("ledger_reconcile", title, NOT_EVALUATED, None, threshold,
                     detail=f"no billed ledger row of {', '.join(limit['providers'])} in these runs")
    value, failed, missing = {}, [], []
    for provider in sorted(billed):
        doc = evidence.reconciles.get(provider)
        if doc is None:
            missing.append(provider)
            continue
        entries = [entry for entry in [*doc.get("comparisons", []), *doc.get("totals", [])] if isinstance(entry, dict)]
        drifts = [abs(entry["drift"]) for entry in entries if isinstance(entry.get("drift"), (int, float))]
        undefined = [entry for entry in entries if entry.get("drift") is None and entry.get("flagged")]
        value[provider] = round(max(drifts), 6) if drifts else None
        if undefined or any(drift > limit["tolerance"] for drift in drifts):
            failed.append(provider)
    run_ids = sorted(set().union(*billed.values()))
    if failed:
        return Check("ledger_reconcile", title, FAIL, value, threshold, run_ids,
                     detail=f"drift past ±{limit['tolerance']:.0%} for {', '.join(failed)}")
    if missing:
        return Check("ledger_reconcile", title, NOT_EVALUATED, value, threshold, run_ids,
                     detail=f"no reconcile JSON for {', '.join(missing)} (--reconcile)")
    return Check("ledger_reconcile", title, PASS, value, threshold, run_ids, detail="largest |drift| per provider")


def _runaway(evidence: Evidence, limit: dict) -> Check:
    title = "Synthetic runaway stopped"
    threshold = (f"every run stopped by the runaway detector ({', '.join(limit['reasons'])}) within "
                 f"{limit['max_calls']} calls")
    synthetic = evidence.runaway is not None
    runs = evidence.runaway if synthetic else evidence.runs
    records = [record for run in runs for _, record in run.records]
    stopped = [record for record in records if record["execution"]["status"] == "aborted_cap"
               and record["execution"]["reason"] in limit["reasons"]]
    calls = {record["run_id"]: sum(attempt.get("calls") or 0 for attempt in record["execution"]["attempts"])
             for record in stopped}
    value = [{"run_id": record["run_id"], "reason": record["execution"]["reason"],
              "calls": sum(attempt.get("calls") or 0 for attempt in record["execution"]["attempts"])}
             for record in stopped]
    if synthetic:
        if not records:
            return Check("runaway_killed", title, NOT_EVALUATED, None, threshold,
                         detail="the --runaway directory holds no run record")
        loose = [record["run_id"] for record in records if record not in stopped]
        over = [run_id for run_id, count in calls.items() if count > limit["max_calls"]]
        detail = (f"not stopped by the detector: {', '.join(loose)}" if loose else
                  f"past {limit['max_calls']} calls: {', '.join(over)}" if over else "the synthetic runaway's runs")
        return Check("runaway_killed", title, FAIL if loose or over else PASS, value, threshold, _run_ids(records),
                     detail=detail)
    if not stopped:
        return Check("runaway_killed", title, NOT_EVALUATED, None, threshold,
                     detail="no synthetic runaway (--runaway), and no pilot run hit the runaway detector")
    over = [run_id for run_id, count in calls.items() if count > limit["max_calls"]]
    detail = "pilot runs the runaway detector stopped" + (f"; past the cap: {', '.join(over)}" if over else "")
    return Check("runaway_killed", title, FAIL if over else PASS, value, threshold, sorted(calls), detail=detail)


def _canaries(evidence: Evidence, limit: dict) -> Check:
    title, threshold = "Canary hits", f"at most {limit['max']}"
    records = evidence.records
    if not records:
        return Check("canary_hits", title, NOT_EVALUATED, None, threshold, detail="no run records")
    hits = [record for record in records if record["provenance"]["canary_hits"] > 0
            or record["execution"]["status"] == "leak_suspected"]
    total = sum(record["provenance"]["canary_hits"] for record in records)
    run_ids = sorted({record["run_id"] for record in (hits or records)})
    if total > limit["max"] or hits:
        places = sorted({place for record in hits for place in record["provenance"].get("canary_places", [])})
        return Check("canary_hits", title, FAIL, total, threshold, run_ids,
                     detail=f"{len(hits)} run(s) leak_suspected or with hits, at {', '.join(places) or '-'}")
    return Check("canary_hits", title, PASS, total, threshold, run_ids, detail=f"over {len(records)} runs")


def _sc2(evidence: Evidence, limit: dict) -> Check:
    title = "SC2 spot check"
    threshold = (f"at least {limit['min_checked']} outputs ({limit['min_each_label']} of each census label), "
                 f"{limit['min_agree_share']:.0%} agreeing")
    doc = evidence.sc2
    items = doc.get("item") if isinstance(doc, dict) else None
    if not isinstance(items, list) or not items:
        return Check("sc2_spot_check", title, NOT_EVALUATED, None, threshold,
                     detail="no hand-filled spot check (--sc2, vb.sc2/1 with [[item]] tables)")
    by_key = {(record["run_id"], record["task"]["instance_id"], record["seed"]): record for record in evidence.records}
    agree, labels, unknown = 0, {0: 0, 1: 0}, []
    for item in items:
        record = by_key.get((item.get("run_id"), item.get("instance_id"), item.get("seed")))
        if record is None or item.get("human_label") not in (0, 1):
            unknown.append(f"{item.get('run_id')}/{item.get('instance_id')}.s{item.get('seed')}")
            continue
        census = metrics.vs_minus(record)
        labels[census] += 1
        agree += census == item["human_label"]
    checked = len(items) - len(unknown)
    value = f"{agree}/{checked}"
    run_ids = sorted({str(item.get("run_id")) for item in items})
    if unknown:
        return Check("sc2_spot_check", title, NOT_EVALUATED, value, threshold, run_ids,
                     detail=f"items without a run record or a 0/1 human_label: {', '.join(unknown[:5])}")
    if checked < limit["min_checked"] or min(labels.values()) < limit["min_each_label"]:
        return Check("sc2_spot_check", title, NOT_EVALUATED, value, threshold, run_ids,
                     detail=f"{checked} outputs checked, {labels[1]} with VS = 1 and {labels[0]} with VS = 0")
    ok = agree >= limit["min_agree_share"] * checked
    return Check("sc2_spot_check", title, PASS if ok else FAIL, value, threshold, run_ids,
                 detail=f"checked by {doc.get('checked_by', '?')} on {doc.get('date', '?')}")


def _ladder(evidence: Evidence, name: str, limit: dict) -> Check:
    model = f" {limit['model']}" if limit["model"] else ""
    bound = f"≥ {limit['min_vs']}" if "min_vs" in limit else f"≤ {limit['max_vs']}"
    title, threshold = f"Ladder: {limit['arm']}{model} at level {limit['level']}", f"VS {bound}"
    kept = [record for record in evidence.records if record["arm"] == limit["arm"]
            and record["task"]["family"] != metrics.PLAN_SLICE and record["task"]["ladder"] == limit["level"]
            and record["execution"]["status"] not in metrics.EXCLUDED
            and (limit["model"] is None or _model(record) == limit["model"])]
    rate = metrics.vs_rate(kept)
    if rate is None:
        return Check(name, title, NOT_EVALUATED, None, threshold, detail="no kept run at that level")
    ok = rate >= limit["min_vs"] if "min_vs" in limit else rate <= limit["max_vs"]
    return Check(name, title, PASS if ok else FAIL, round(rate, 4), threshold, _run_ids(kept),
                 detail=f"{len(kept)} runs on {len({metrics.task_key(record) for record in kept})} tasks")


def _cost(evidence: Evidence, arm: str, plan: float, max_ratio: float) -> Check:
    name, title = f"cost_per_task.{arm}", f"Measured $/task, {arm}"
    threshold = f"≤ {max_ratio:g} × the plan's ${plan:g}"
    mine = [record for record in evidence.records if record["arm"] == arm]
    if not mine:
        return Check(name, title, NOT_EVALUATED, None, threshold, detail=f"no {arm} run")
    unknown = [record for record in mine if record["costs"]["billed_usd"] is None]
    if unknown:
        return Check(name, title, NOT_EVALUATED, None, threshold, _run_ids(mine),
                     detail=f"{len(unknown)} of {len(mine)} runs have an unknown cost")
    per_task = math.fsum(record["costs"]["billed_usd"] for record in mine) / len(mine)
    ratio = per_task / plan
    return Check(name, title, PASS if ratio <= max_ratio else FAIL, round(per_task, 6), threshold, _run_ids(mine),
                 detail=f"{ratio:.2f} × the plan over {len(mine)} runs")


def _u_prime_and_r(evidence: Evidence, limit: dict) -> Check:
    title, threshold = f"U′ and R for every {limit['arm']} run", f"{limit['min_captured']:.0%} captured; gap reported"
    mine = [record for record in evidence.records if record["arm"] == limit["arm"]]
    if not mine:
        return Check("u_prime_and_r", title, NOT_EVALUATED, None, threshold, detail=f"no {limit['arm']} run")
    both = [record for record in mine if record["costs"]["api_equiv_usd"] is not None
            and record["costs"]["vendor_usd"] is not None]
    u_prime = math.fsum(record["costs"]["api_equiv_usd"] for record in both)
    r_usd = math.fsum(record["costs"]["vendor_usd"] for record in both)
    gap = round(abs(u_prime - r_usd) / r_usd, 6) if r_usd else None
    value = {"captured": f"{len(both)}/{len(mine)}", "u_r_gap": gap}
    ok = len(both) >= limit["min_captured"] * len(mine)
    return Check("u_prime_and_r", title, PASS if ok else FAIL, value, threshold, _run_ids(mine),
                 detail=f"ΣU′ ${u_prime:.4f}, ΣR ${r_usd:.4f}, |ΣU′ − ΣR|/ΣR over the runs with both")


def _fault_rates(evidence: Evidence, limit: dict) -> Check:
    title = "Injected fault rates"
    threshold = f"within ±{limit['max_abs_diff'] * 100:g} points of each profile's p, over ≥ {limit['min_requests']}"
    profiles: dict[tuple[str, float], list[bool]] = {}
    run_ids = set()
    for run in evidence.runs:
        log = run.path / report.PROXY_LOG
        if not any("provider_fault" in record["stream"].get("perturbations_active", []) for _, record in run.records):
            continue
        for row in _jsonl(log):
            profile = row.get("profile") if isinstance(row.get("profile"), dict) else {}
            if profile.get("name") in (None, "clean"):
                continue
            key = (profile["name"], float(profile.get("p", 1.0)))
            profiles.setdefault(key, []).append(row.get("fault_injected") == profile["name"])
            run_ids.add(run.path.name)
    if not profiles:
        return Check("fault_rates", title, NOT_EVALUATED, None, threshold,
                     detail="no provider_fault run with its proxy log in these runs")
    value, short, off = {}, [], []
    for (name, p), hits in sorted(profiles.items()):
        label = f"{name}(p={p:g})"
        value[label] = f"{sum(hits)}/{len(hits)}"
        if len(hits) < limit["min_requests"]:
            short.append(label)
        elif abs(sum(hits) / len(hits) - p) > limit["max_abs_diff"]:
            off.append(label)
    if off:
        return Check("fault_rates", title, FAIL, value, threshold, sorted(run_ids),
                     detail=f"off by more than the bound: {', '.join(off)}")
    if short:
        return Check("fault_rates", title, NOT_EVALUATED, value, threshold, sorted(run_ids),
                     detail=f"too few requests to measure: {', '.join(short)}")
    return Check("fault_rates", title, PASS, value, threshold, sorted(run_ids))


def _roko_ingest(evidence: Evidence, limit: dict) -> Check:
    title, threshold = f"Every {limit['arm']} record ingested", "all planned runs recorded; no model mismatch kept"
    runs = [run for run in evidence.runs if (run.manifest or {}).get("arm") == limit["arm"]
            or any(record["arm"] == limit["arm"] for _, record in run.records)]
    if not runs:
        return Check("roko_ingest", title, NOT_EVALUATED, None, threshold, detail=f"no {limit['arm']} run")
    planned = sum(int((run.manifest or {}).get("runs") or 0) for run in runs)
    records = [record for run in runs for _, record in run.records]
    kept_mismatch = [record for record in records if record["execution"]["status"] != "infra_error"
                     and any("model_mismatch" in (attempt.get("checks") or [])
                             for attempt in record["execution"]["attempts"])]
    problems = [problem for run in runs for problem in run.problems]
    value = {"recorded": f"{len(records)}/{planned}", "mismatch_outside_infra_error": len(kept_mismatch)}
    ok = planned > 0 and len(records) == planned and not kept_mismatch and not problems
    detail = "; ".join(problems[:3]) if problems else f"{len(runs)} run(s)"
    return Check("roko_ingest", title, PASS if ok else FAIL, value, threshold, [run.path.name for run in runs],
                 detail=detail)


def _direct_loop(evidence: Evidence, limit: dict) -> Check:
    title, threshold = "Direct-loop choice (S08 decision 3)", f"one of {', '.join(limit['choices'])}, recorded"
    table = (evidence.sc2 or {}).get("direct_loop")
    choice = table.get("choice") if isinstance(table, dict) else None
    if choice is None:
        return Check("direct_loop_choice", title, NOT_EVALUATED, None, threshold,
                     detail="not recorded: [direct_loop] choice in the --sc2 file")
    ok = choice in limit["choices"]
    return Check("direct_loop_choice", title, PASS if ok else FAIL, choice, threshold,
                 detail=str(table.get("reason", "")))


# ---------------------------------------------------------------- helpers


def _model(record: dict) -> str | None:
    requested = {attempt["model_requested"] for attempt in record["execution"]["attempts"]}
    return requested.pop() if len(requested) == 1 else None


def _run_ids(records: list[dict]) -> list[str]:
    return sorted({record["run_id"] for record in records})


def _json_file(path: Path, evidence: Evidence, schema: str | None = None) -> dict | None:
    try:
        doc = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as err:
        evidence.problems.append(f"{path}: {err}")
        return None
    if not isinstance(doc, dict) or (schema and doc.get("schema_version") != schema):
        evidence.problems.append(f"{path}: not a {schema or 'JSON object'} document")
        return None
    return doc


def _jsonl(path: Path) -> list[dict]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return []
    rows = []
    for line in lines:
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if isinstance(row, dict):
            rows.append(row)
    return rows


def _show(value: object) -> str:
    if value is None:
        return "-"
    if isinstance(value, dict):
        return ", ".join(f"{key} {_show(item)}" for key, item in value.items())
    if isinstance(value, list):
        return "; ".join(_show(item) for item in value) or "-"
    return str(value)


if __name__ == "__main__":
    sys.exit(main())
