"""What the paper's figure and table scripts share (paper/FIGURES-TABLES.md, "Conventions"; W9 PW10).

Each `fig_<id>_<name>.py` and `tab_<id>_<name>.py` beside this file is thin: it names the MetricRecords it reads and
lays them out with this module, which loads and checks the records, draws stdlib SVG (as `tools/status_matrix.py`
draws the whitepaper's Figure 3) or writes a Markdown table, and writes the sidecar. Every script takes

    <script>.py INPUT [INPUT ...] --out DIR [--dry-run]

**Input.** `vb.metric_record/1` rows (S08 §5.5) and nothing else: a `metrics.json` from `report.py` (`vb.metrics/1`,
its `records`), a JSON list of records, or a JSONL file with one record per line. A `vb.metrics/1` document's
`plan_slice` section is kept for T11's per-feature rows, which its MetricRecords' `run_ids` name. The schema gives a
record no id, so its id is the `sha256:` digest of its canonical JSON (sorted keys, no whitespace, the digest
`driver/records.py::canonical_hash` gives run records): anyone holding the record can recompute it.

**Refusals.** A script writes nothing and exits 1 when
- a record or document says `simulated` is anything but false (S10 §4.5), outside a dry run;
- a record fails `schema/validate.py`, so it is not a `vb.metric_record/1`;
- a record has no `run_ids`;
- a P1 script (F2-F5, F9; T3, T7, T9, T11) reads a record whose `label_source` is not `vs_census`;
- a figure would draw a mark without a CI (CIs are always drawn), or an arm with no fixed colour (`ARMS`);
- it finds nothing to draw.
Exit status 2 is a usage error or an unreadable input.

**Dry run** (`--dry-run`). Every input record and document must say `simulated: true`, so synthetic numbers never
mix with results; apart from that flag each record must still validate. Every output is stamped "DRY RUN: synthetic
records, not results" and its sidecar says `"dry_run": true`. Nothing a dry run writes belongs in the paper: its
`[[RESULT …]]` slots are filled only from real bundles. `synthetic_record` builds such records from the schema's
example.

**Output**, in `--out`: `<stem>.svg` for a figure or `<stem>.md` for a table (pipe tables with a pandoc caption), and
the sidecar `<stem>.data.json` (`vb.figure_data/1`):

    {"schema_version", "id", "kind", "title", "script", "dry_run", "inputs": [{"name", "sha256"}], "caption",
     "values": [{"panel", "series", "role", "metric", "value", "ci", "ci_method", "n", "record_id"}],
     "rows": [{"panel", "series", "role", "value", "run_id", "run_record_id", "named_by": [record_id, ...]}],
     "derived": [{"panel", "what", "value", "from": [record_id, ...]}],
     "design": [{"panel", "what", "value", "source"}], "not_drawn": [{"panel", "why"}]}

`values` lists every plotted or tabulated number with the MetricRecord it came from; `rows` the per-run values T11
reads from the bundle's per-feature table, each with the MetricRecords whose `run_ids` name its run; `derived` what a
script computes from those (a frontier, a reference line at 0.90 times a value, a verdict), with the ids it used;
`design` holds spec constants such as S08 §4.4's design bands. The PDF comes from the paper build
(`paper/tools/build.sh`, task 9511, through rsvg-convert), so no script writes one.

**Encoding.** Rates on fixed [0, 1] axes (a zoomed axis says so); dollars on log axes with labelled ticks; a CI on
every mark, its method in the caption; no dual axes. One colour per arm in every figure (`ARMS`, from the Okabe-Ito
colour-blind-safe palette); the shape is the harness (circle direct, diamond Roko) and the fill the tier (hollow
cheap, filled frontier); the secondary comparators are grey. Marks carry direct labels rather than a legend. The
caption states n, the CI methods, `price_snapshot_id`, the harness commits and `prereg_id`.

**Metric names.** The scripts read the names `report.py` emits (`REPORT_METRICS`). A name no producer emits yet is
declared in the reading script's `READS`, with the cut it is read at, so S09 E3/E9, `econ.py` (6123) and `replay.py`
(3340) can emit exactly those names (FIGURES-TABLES reconciliation item 2). A cut is read off the record's
`record_filter`, as `metrics.Clause` writes it: `task.ladder` (or the `ladder` field) for the level, `task.family`,
`model` for an arm's model, `stream.id`, `task.spec_variant`, `task.instance_id`, and the clauses a `READS` names.

API:
    FigureError; Spec(id, stem, title, script, kind="figure", p1=True)
    run(spec, build, argv=None) -> int                 # the CLI each script's main() calls
    load(paths, *, dry_run, p1) -> Inputs              # read and check; raises FigureError
    Inputs.where(metric, ...) -> list[Rec]; Inputs.one(metric, ...) -> Rec | None
    Rec: doc, id, clauses; metric, value, ci, ci_method, n, arms, arm, series, level, family, model, eq(field)
    Figure(spec, dry_run, width, height); Figure.grid(...); Figure.panel(...) -> Panel
    Panel: point, whisker_x, whisker_y, line, band, ref, shade, text, hbar, end_labels, note
    Table(spec, dry_run); Table.part(...); Table.cell(...)
    Output.take / take_row / derive / design / skip; rate_axis, linear_axis, log_axis, category_axis; arm_style
    canonical_id(record) -> str; parse_filter(text) -> clauses; synthetic_record(metric, value, ...) -> dict
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import math
import re
import sys
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path

import metrics

ANALYSIS_DIR = Path(__file__).resolve().parent
VB_ROOT = ANALYSIS_DIR.parent
sys.path.insert(0, str(VB_ROOT / "schema"))
import validate  # noqa: E402

SIDECAR_VERSION = "vb.figure_data/1"
METRICS_VERSION = "vb.metrics/1"
RECORD_VERSION = "vb.metric_record/1"
EXAMPLE = VB_ROOT / "schema" / "examples" / "metric-record.json"
DRY_RUN_STAMP = "DRY RUN: synthetic records, not results"
P1_LABEL = "vs_census"
X_BAR, Y_BAR = 0.90, 0.30  # H1's bars on R and C (D3; S09 §4.1)
P1_CORE = ("LOG1", "E-P1-live")  # S09's experiments on the 120 P1-core tasks
P1_EXT = ("E-P1-ext",)  # the external SWE-bench Verified slice (60 tasks)
REFERENCE_ARM = "fd_claude"  # H1's ratios R and C are arm / fd_claude
H3_STREAM = "p1_h3"  # task 3328's stream ids
PASS5_STREAM = "p1_pass5"  # the 30-task pass^5 subset's stream (FIGURES-TABLES F4: stream.id separates it)
# The filter fields that place a record in a report.py cell. Any other `==` or `in` clause narrows the cell further
# (a stream, a spec variant, one task, a rung ...), so `Inputs.where` returns such a record only when asked for that
# field. `policy` only names the policy a live arm ran, and the P1-core stream is the default stream.
BASE_FIELDS = frozenset({"experiment_id", "arm", "model", "task.family", "task.ladder", "task.is_honeypot",
                         "execution.status", "policy"})
BASE_STREAMS = (None, "p1_core")
# What report.py emits today (metrics.py), read under these names: `pass_hat_<k>` and the `_unknown_as_1` bounds
# follow the same pattern.
REPORT_METRICS = frozenset({
    "vs_rate", "vs_rate_unknown_as_1", "honest_conflict_rate", "usd_per_vs", "usd_per_vs_vendor", "cost_gap_ur",
    "spend_usd", "false_greens", "false_green_rate", "unverified_runs", "already_satisfied_runs",
    "cap_censored_runs", "infra_error_runs", "leak_suspected_runs", "estimated_cost_runs", "unknown_cost_runs",
    "pl_verified_features", "pl_vf_rate", "pl_vf_rate_unknown_as_1", "pl_cpf_usd", "pl_cpf_vendor_usd",
    "pl_makespan_median_s", "pl_makespan_min_s", "pl_makespan_max_s", "pl_queue_wait_median_s", "pl_planner_share",
    "pl_realized_parallelism_median", "pl_tasks_escalated_share", "pl_integrations_rejected", "pl_cpf_ratio",
    "pl_makespan_median_ratio",
})


class FigureError(ValueError):
    """The figure or table cannot be drawn honestly; the message says why. Nothing is written."""


class _Any:
    def __repr__(self) -> str:
        return "ANY"


ANY = _Any()  # in `Inputs.where`: any value, or any clause


@dataclass(frozen=True)
class Spec:
    id: str  # "F2"
    stem: str  # output file stem, "f2-pareto"
    title: str
    script: str  # "fig_f2_pareto.py"
    kind: str = "figure"  # or "table"
    p1: bool = True  # P1 numbers: label_source must be vs_census


# ---------------------------------------------------------------------------------------------------------- records

def canonical_id(record: dict) -> str:
    """A MetricRecord's id: the sha256 of its canonical JSON (sorted keys, no whitespace)."""
    text = json.dumps(record, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False)
    return "sha256:" + hashlib.sha256(text.encode("utf-8")).hexdigest()


_CLAUSE = re.compile(r"([A-Za-z_][\w.]*) (==|!=|not in|in) ")
_DECODER = json.JSONDecoder()


def parse_filter(text: str | None) -> tuple[tuple[str, str, object], ...]:
    """The clauses of a `record_filter` as `metrics.Cut.filter` writes them: `field op <json>`, joined by " and "."""
    clauses, pos, text = [], 0, text or ""
    while pos < len(text):
        match = _CLAUSE.match(text, pos)
        try:
            if not match:
                raise ValueError("expected `field op value`")
            value, pos = _DECODER.raw_decode(text, match.end())
        except ValueError as err:
            raise FigureError(f"cannot read the record_filter {text!r} at character {pos}: {err}") from None
        clauses.append((match[1], match[2], value))
        if text.startswith(" and ", pos):
            pos += len(" and ")
        elif pos != len(text):
            raise FigureError(f"cannot read the record_filter {text!r} at character {pos}: expected ' and '")
    return tuple(clauses)


@dataclass(frozen=True)
class Rec:
    """One loaded MetricRecord: the row as read, its id and its filter's clauses."""

    doc: dict
    id: str
    clauses: tuple[tuple[str, str, object], ...]

    @property
    def metric(self) -> str:
        return self.doc["metric"]

    @property
    def value(self) -> float | None:
        return self.doc["value"]

    @property
    def ci(self) -> tuple[float, float] | None:
        ci = self.doc.get("ci")
        return (ci[0], ci[1]) if ci else None

    @property
    def ci_method(self) -> str:
        return self.doc.get("ci_method", "none")

    @property
    def n(self) -> int:
        return self.doc["n"]

    @property
    def arms(self) -> tuple[str, ...]:
        return tuple(self.doc["arms"])

    @property
    def arm(self) -> str | None:
        return self.arms[0] if len(self.arms) == 1 else None

    @property
    def experiment(self) -> str:
        return self.doc["experiment_id"]

    def eq(self, field: str, default: object = None) -> object:
        """The value of the filter's first `field == value` clause, or `default`."""
        return next((value for name, op, value in self.clauses if name == field and op == "=="), default)

    @property
    def level(self) -> int | None:
        return self.doc.get("ladder") or self.eq("task.ladder")

    @property
    def family(self) -> str | None:
        return self.eq("task.family")

    @property
    def model(self) -> str | None:
        return self.eq("model")

    @property
    def series(self) -> str:
        """The cell as report.py names it: the arm, with its model when it has one; joined arms for a contrast."""
        return metrics.cell_name(self.arms[0], self.model) if len(self.arms) == 1 else " / ".join(self.arms)


@dataclass
class Inputs:
    records: list[Rec]
    sources: list[dict]  # [{"name", "sha256"}] per input file
    sections: dict[str, list]  # e.g. {"plan_slice": [(path, section), ...]} from vb.metrics/1 documents
    dry_run: bool
    paths: list[Path]

    def where(self, metric: str, *, experiment: object = ANY, arm: object = ANY, against: Sequence[str] = (),
              level: object = None, family: object = None, model: object = ANY, cost_basis: object = ANY,
              clauses: dict[str, object] | None = None) -> list[Rec]:
        """The records of `metric` in one cut. `level` and `family` default to None, the pooled cell: no level (no
        `ladder`, no `task.ladder` clause) and no `task.family ==` clause; pass ANY for any. `arm` with `against`
        matches a record whose `arms` are exactly {arm} and `against`. `experiment` is an id or a tuple of ids.
        `clauses` maps a field to the value its `==` clause must have: ANY needs the clause, None forbids it, a tuple
        allows its values. A record whose filter narrows the cell on a field outside `BASE_FIELDS` that `clauses`
        does not name is left out, so a pooled query never returns a per-task or per-variant record."""
        found = []
        named = clauses or {}
        for rec in self.records:
            if rec.metric != metric or not _matches(rec.experiment, experiment) or _narrows(rec, named):
                continue
            if arm is not ANY and set(rec.arms) != {arm, *against}:
                continue
            if not (_matches(rec.level, level) and _matches(rec.family, family) and _matches(rec.model, model)):
                continue
            if not _matches(rec.doc.get("cost_basis"), cost_basis):
                continue
            if all(_matches(rec.eq(name, None), want) and (want is not ANY or rec.eq(name, ANY) is not ANY)
                   for name, want in (clauses or {}).items()):
                found.append(rec)
        return found

    def one(self, metric: str, **cut: object) -> Rec | None:
        """The one record of `metric` in the cut, or None; two records in one cut are an error."""
        found = self.where(metric, **cut)
        if len(found) > 1:
            raise FigureError(f"{len(found)} {metric} records fit one cut ({_cut_text(cut)}): "
                              + ", ".join(rec.id for rec in found) + "; a cut must name one record")
        return found[0] if found else None

    def series_of(self, metric: str, **cut: object) -> list[str]:
        """The single-arm series that have `metric` in the cut, in `ARMS` order then by name."""
        names = {rec.series: rec.arm for rec in self.where(metric, **cut) if rec.arm is not None}
        return sorted(names, key=lambda name: (arm_order(names[name]), name))


def _narrows(rec: Rec, named: dict) -> bool:
    """Whether the record's filter narrows its cell on a field the query does not name."""
    for name, op, value in rec.clauses:
        if name in BASE_FIELDS or name in named or op not in ("==", "in"):
            continue
        if not (name == "stream.id" and op == "==" and value in BASE_STREAMS):
            return True
    return False


def _matches(actual: object, want: object) -> bool:
    if want is ANY:
        return True
    if isinstance(want, tuple):
        return actual in want
    return actual == want


def _cut_text(cut: dict) -> str:
    return ", ".join(f"{key} {value!r}" for key, value in cut.items() if value is not ANY) or "no cut"


def load(paths: Iterable[Path | str], *, dry_run: bool, p1: bool) -> Inputs:
    """Read and check every input (the module docstring has the rules); raises FigureError with every problem."""
    records: dict[str, Rec] = {}
    sources, sections, problems = [], {}, []
    paths = list(paths)
    for path in map(Path, paths):
        raw = path.read_bytes()
        sources.append({"name": path.name, "sha256": "sha256:" + hashlib.sha256(raw).hexdigest()})
        try:
            rows, document = _rows(path, raw.decode("utf-8"))
        except ValueError as err:
            raise FigureError(f"{path.name}: {err}") from None
        if document is not None:
            problems += _flag_problem(path.name, document, dry_run)
            for key, section in document.items():
                if key == "plan_slice" and section:
                    sections.setdefault(key, []).append((path, section))
        for where, row in rows:
            found = _record_problems(where, row, dry_run=dry_run, p1=p1)
            problems += found
            if not found:
                rec_id = canonical_id(row)
                records.setdefault(rec_id, Rec(row, rec_id, parse_filter(row.get("record_filter"))))
    if problems:
        shown = problems[:25] + ([f"... and {len(problems) - 25} more"] if len(problems) > 25 else [])
        raise FigureError(f"{len(problems)} problem(s) in the inputs:\n  " + "\n  ".join(shown))
    if not records:
        raise FigureError("the inputs hold no MetricRecord")
    return Inputs(list(records.values()), sources, sections, dry_run, [Path(path) for path in paths])


def _rows(path: Path, text: str) -> tuple[list[tuple[str, object]], dict | None]:
    """(each record with its location, the vb.metrics/1 document or None)."""
    if path.suffix == ".jsonl":
        return [(f"{path.name}:{number}", json.loads(line))
                for number, line in enumerate(text.splitlines(), 1) if line.strip()], None
    doc = json.loads(text)
    if isinstance(doc, list):
        return [(f"{path.name}[{index}]", row) for index, row in enumerate(doc)], None
    if isinstance(doc, dict) and doc.get("schema_version") == RECORD_VERSION:
        return [(path.name, doc)], None
    if isinstance(doc, dict) and doc.get("schema_version") == METRICS_VERSION and isinstance(doc.get("records"), list):
        return [(f"{path.name}: records[{index}]", row) for index, row in enumerate(doc["records"])], doc
    raise ValueError(f"not a {METRICS_VERSION} document, a list of {RECORD_VERSION} rows or a JSONL file of them")


def _flag_problem(where: str, doc: dict, dry_run: bool) -> list[str]:
    flag = doc.get("simulated", False)
    if dry_run and flag is not True:
        return [f"{where}: a dry run draws only synthetic records, and this one does not say simulated: true"]
    if not dry_run and flag is not False:
        return [f"{where}: simulated is {json.dumps(flag)}; the paper draws only real records (S10 §4.5)"]
    return []


def _record_problems(where: str, row: object, *, dry_run: bool, p1: bool) -> list[str]:
    if not isinstance(row, dict):
        return [f"{where}: not a JSON object"]
    try:
        canonical_id(row)
    except ValueError:
        return [f"{where}: holds a NaN or an infinite number, which JSON cannot carry"]
    problems = _flag_problem(where, row, dry_run)
    body = {key: value for key, value in row.items() if key != "simulated"}
    problems += [f"{where}: {error}" for error in validate.validate("metric-record", body)]
    if not row.get("run_ids"):
        problems.append(f"{where}: no run_ids; a number without the runs behind it is never drawn")
    if p1 and row.get("label_source") != P1_LABEL:
        problems.append(f"{where}: label_source is {json.dumps(row.get('label_source'))}; P1 numbers use "
                        f"{P1_LABEL} labels only")
    return problems


def synthetic_record(metric: str, value: float | None, *, arms: Sequence[str], experiment_id: str = "LOG1",
                     ci: Sequence[float] | None = None, ci_method: str | None = None, n: int = 360,
                     record_filter: str | None = None, ladder: int | None = None, cost_basis: str | None = None,
                     label_source: str = P1_LABEL, seeds: Sequence[int] = (1, 2, 3), estimator: str | None = None,
                     prereg_id: str | None = "dry-run") -> dict:
    """A `simulated: true` MetricRecord built from the schema's example, for dry runs and their tests only."""
    record = json.loads(EXAMPLE.read_text(encoding="utf-8"))
    for key in ("ci", "ladder"):
        record.pop(key, None)
    record.update(metric=metric, value=value, n=n, experiment_id=experiment_id, arms=list(arms),
                  run_ids=[f"dry-run-{experiment_id}-{'-'.join(arms)}"], seeds=list(seeds), commits=["dry-run"],
                  estimator=estimator or f"synthetic {metric} for a dry run; not a result",
                  ci_method=ci_method or ("dry_run_bootstrap" if ci is not None else "none"),
                  label_source=label_source, cost_basis=cost_basis, prereg_id=prereg_id,
                  preregistered=prereg_id is not None)
    if ci is not None:
        record["ci"] = [float(ci[0]), float(ci[1])]
    if record_filter:
        record["record_filter"] = record_filter
    if ladder is not None:
        record["ladder"] = ladder
    record["simulated"] = True
    return record


# ----------------------------------------------------------------------------------------------------------- output

class Output:
    """The sidecar's bookkeeping, shared by figures and tables: each number drawn, derived or taken from a spec."""

    def __init__(self, spec: Spec, dry_run: bool):
        self.spec, self.dry_run = spec, dry_run
        self.values: list[dict] = []
        self.derived: list[dict] = []
        self.designs: list[dict] = []
        self.skipped: list[dict] = []
        self.rows: list[dict] = []
        self.used: dict[str, Rec] = {}

    def take(self, rec: Rec, *, panel: str, role: str, series: str | None = None,
             need_ci: bool = False) -> float | None:
        """The record's value, listed in the sidecar. A null value is listed as not drawn, and None comes back."""
        series = series or rec.series
        if need_ci and rec.ci is None:
            raise FigureError(f"{rec.metric} of {series} has no ci (ci_method {rec.ci_method}; record {rec.id}); "
                              f"{self.spec.id} draws a CI for every mark")
        if rec.value is None:
            why = rec.doc["estimator"].split("null: ", 1)[1] if "null: " in rec.doc["estimator"] else "no value"
            self.skip(panel, f"{rec.metric} of {series} is null ({why}); record {rec.id}")
            return None
        self.used[rec.id] = rec
        self.values.append({"panel": panel, "series": series, "role": role, "metric": rec.metric,
                            "value": rec.value, "ci": list(rec.ci) if rec.ci else None,
                            "ci_method": rec.ci_method, "n": rec.n, "record_id": rec.id})
        return rec.value

    def take_row(self, panel: str, series: str, role: str, value: object, row: dict,
                 named_by: Sequence[Rec]) -> object:
        """A value from a per-run row that MetricRecords name by run id (T11's per-feature table), never a number
        the records do not cover: the run must be in the `run_ids` of a record already taken."""
        ids = [rec.id for rec in named_by if row["run_id"] in rec.doc["run_ids"] and rec.id in self.used]
        if not ids:
            raise FigureError(f"{self.spec.id}: run {row['run_id']} is named by no MetricRecord it drew")
        self.rows.append({"panel": panel, "series": series, "role": role, "value": value, "run_id": row["run_id"],
                          "run_record_id": row.get("record_id"), "named_by": ids})
        return value

    def derive(self, panel: str, what: str, value: object, sources: Iterable[Rec]) -> object:
        """A number computed from records already taken (a frontier, a bar times a value, a verdict)."""
        ids = [rec.id for rec in sources]
        missing = [rec_id for rec_id in ids if rec_id not in self.used]
        if missing:
            raise FigureError(f"{self.spec.id} derives {what!r} from records it did not take: {', '.join(missing)}")
        self.derived.append({"panel": panel, "what": what, "value": value, "from": ids})
        return value

    def design(self, panel: str, what: str, value: object, source: str) -> object:
        """A constant from a spec, such as a design band or a decided bar; never a result."""
        self.designs.append({"panel": panel, "what": what, "value": value, "source": source})
        return value

    def skip(self, panel: str, why: str) -> None:
        self.skipped.append({"panel": panel, "why": why})

    def caption(self) -> str:
        """n, the CI methods, the price snapshot, the harness commits and prereg_id of every record taken."""
        recs = list(self.used.values())
        ns = sorted({rec.n for rec in recs})
        seeds = sorted({seed for rec in recs for seed in rec.doc["seeds"]})
        methods = sorted({rec.ci_method for rec in recs if rec.ci is not None})
        listed = {key: sorted({str(item) for rec in recs for item in (rec.doc[key] if isinstance(rec.doc[key], list)
                                                                        else [rec.doc[key]])})
                  for key in ("price_snapshot_id", "commits", "prereg_id")}
        prereg = [item if item != "None" else "none (descriptive)" for item in listed["prereg_id"]]
        n_text = f"n = {ns[0]}" if len(ns) == 1 else f"n = {ns[0]} to {ns[-1]}"
        text = (f"{self.spec.id}: {self.spec.title}. {n_text} per number (runs or tasks, as each record's estimator "
                f"says; seeds {', '.join(map(str, seeds)) or 'none'}); intervals: {', '.join(methods) or 'none'} "
                f"(95% unless the method says otherwise); prices {', '.join(listed['price_snapshot_id'])}; harness "
                f"{', '.join(listed['commits'])}; prereg {', '.join(prereg)}.")
        return f"{DRY_RUN_STAMP}. {text}" if self.dry_run else text

    def sidecar(self, inputs: Inputs) -> dict:
        return {"schema_version": SIDECAR_VERSION, "id": self.spec.id, "kind": self.spec.kind,
                "title": self.spec.title, "script": self.spec.script, "dry_run": self.dry_run,
                "inputs": inputs.sources, "caption": self.caption(), "values": self.values, "rows": self.rows,
                "derived": self.derived, "design": self.designs, "not_drawn": self.skipped}

    def body(self) -> tuple[str, str]:
        raise NotImplementedError

    def files(self, inputs: Inputs) -> dict[str, str]:
        if not self.values:
            raise FigureError(f"{self.spec.id} found nothing to draw in the inputs: "
                              + ("; ".join(item["why"] for item in self.skipped) or "no record it reads"))
        suffix, text = self.body()
        sidecar = json.dumps(self.sidecar(inputs), indent=1, ensure_ascii=False) + "\n"
        return {f"{self.spec.stem}{suffix}": text, f"{self.spec.stem}.data.json": sidecar}


def run(spec: Spec, build: Callable[[Inputs], Output], argv: Sequence[str] | None = None) -> int:
    """The command line every script's main() hands to: read, check, build, then write everything or nothing."""
    parser = argparse.ArgumentParser(prog=spec.script, description=f"{spec.id}: {spec.title} (paper/FIGURES-"
                                     "TABLES.md).", allow_abbrev=False)
    parser.add_argument("inputs", nargs="+", type=Path, help="metrics.json documents, or JSON or JSONL files of "
                        f"{RECORD_VERSION} rows")
    parser.add_argument("--out", type=Path, required=True, help="the directory for the output and its sidecar")
    parser.add_argument("--dry-run", action="store_true", help="draw synthetic records (each simulated: true), "
                        "stamped as a dry run; real records are refused")
    args = parser.parse_args(argv)
    try:
        inputs = load(args.inputs, dry_run=args.dry_run, p1=spec.p1)
        output = build(inputs)
        files = output.files(inputs)
    except FigureError as err:
        print(f"{spec.script}: refused: {err}", file=sys.stderr)
        return 1
    except (OSError, UnicodeDecodeError) as err:
        print(f"{spec.script}: {err}", file=sys.stderr)
        return 2
    args.out.mkdir(parents=True, exist_ok=True)
    for name, text in files.items():
        (args.out / name).write_text(text, encoding="utf-8")
    print(f"{spec.script}: wrote {', '.join(files)} in {args.out}: {len(output.values)} values from "
          f"{len(output.used)} records" + (f" ({DRY_RUN_STAMP})" if args.dry_run else ""), file=sys.stderr)
    return 0


# ----------------------------------------------------------------------------------------------------- arm encoding

@dataclass(frozen=True)
class Style:
    colour: str
    shape: str = "circle"  # circle: a direct harness (or none); diamond: Roko
    filled: bool = False  # filled: frontier tier; hollow: cheap tier


GREY = "#8c8c8c"
# One fixed arm -> colour mapping for every figure (Okabe-Ito; yellow is left out, too faint on white). Secondary
# comparators are grey (FIGURES-TABLES "Encoding").
ARMS = {
    "roko_full": Style("#0072B2", "diamond"),
    "roko_fixed": Style("#56B4E9", "diamond"),
    "roko_plan": Style("#009E73", "diamond"),
    "roko_ladder": Style("#000000", "diamond"),
    "cheap_direct": Style("#E69F00", "circle"),
    "fd_claude": Style("#D55E00", "circle", True),
    "fr_claude": Style("#CC79A7", "diamond", True),
    "fd_claude_lite": Style(GREY, "circle", True),
    "fd_codex": Style(GREY, "circle", True),
    "fd_api": Style(GREY, "circle", True),
    "hybrid": Style(GREY, "circle"),
}


def arm_style(arm: str | None) -> Style:
    if arm not in ARMS:
        raise FigureError(f"arm {arm!r} has no fixed colour in figlib.ARMS; add it there, so that every figure draws "
                          "it alike")
    return ARMS[arm]


def arm_order(arm: str | None) -> int:
    """An arm's place in `ARMS`, the order figures and tables list arms in; unknown arms come last."""
    return list(ARMS).index(arm) if arm in ARMS else len(ARMS)


def ink(colour: str) -> str:
    """A label colour that reads on white: grey marks get darker grey text."""
    return "#5c5c5c" if colour == GREY else colour


# ------------------------------------------------------------------------------------------------------------- axes

@dataclass(frozen=True)
class Axis:
    lo: float
    hi: float
    title: str
    log: bool = False
    ticks: tuple[tuple[float, str], ...] = ()
    note: str = ""  # printed by the axis, e.g. "zoomed axis"

    def frac(self, value: float) -> float:
        """0 at `lo`, 1 at `hi`; values outside are clamped (the sidecar keeps the true value)."""
        value = min(max(value, self.lo), self.hi)
        if self.log:
            return (math.log10(value) - math.log10(self.lo)) / (math.log10(self.hi) - math.log10(self.lo))
        return (value - self.lo) / (self.hi - self.lo)


def rate_axis(title: str, lo: float = 0.0, hi: float = 1.0, step: float = 0.2) -> Axis:
    """A rate on its fixed [0, 1] axis; any other range is labelled as a zoomed axis (F8a's [0.80, 1.00])."""
    count = round((hi - lo) / step)
    ticks = tuple((lo + i * step, _num(lo + i * step)) for i in range(count + 1))
    zoomed = (lo, hi) != (0.0, 1.0)
    return Axis(lo, hi, title, ticks=ticks, note=f"zoomed axis [{_num(lo)}, {_num(hi)}]" if zoomed else "")


def linear_axis(values: Iterable[float | None], title: str, *, lo: float = 0.0, ticks: int = 5,
                minimum: float | None = None) -> Axis:
    """A linear axis from `lo` to a round number at or above the largest value (and `minimum`)."""
    top = max([value for value in values if value is not None] + [minimum if minimum is not None else lo + 1e-9])
    raw = (top - lo) / ticks
    magnitude = 10.0 ** math.floor(math.log10(raw)) if raw > 0 else 1.0
    step = next(m * magnitude for m in (1, 2, 2.5, 5, 10) if m * magnitude >= raw)
    count = math.ceil(round((top - lo) / step, 9))
    return Axis(lo, lo + count * step, title, ticks=tuple((lo + i * step, _num(lo + i * step))
                                                          for i in range(count + 1)))


def log_axis(values: Iterable[float | None], title: str, *, money: bool = True) -> Axis:
    """A log10 axis from the decade below the smallest positive value to the decade above the largest, with
    labelled ticks at each decade (and at 2 and 5 when it spans two decades or fewer)."""
    positive = [value for value in values if value is not None and value > 0]
    if not positive:
        raise FigureError(f"no positive value to place on the log axis {title!r}")
    low, high = math.floor(math.log10(min(positive))), math.ceil(math.log10(max(positive)))
    high = max(high, low + 1)
    steps = (1, 2, 5) if high - low <= 2 else (1,)
    ticks = tuple((step * 10.0 ** exponent, (_usd_tick if money else _num)(step * 10.0 ** exponent))
                  for exponent in range(low, high + 1) for step in steps
                  if step * 10.0 ** exponent <= 10.0 ** high * (1 + 1e-9))
    return Axis(10.0 ** low, 10.0 ** high, title, log=True, ticks=ticks)


def category_axis(names: Sequence[str], title: str) -> Axis:
    """Categories at 1, 2, ... (levels, variants, arms)."""
    return Axis(0.5, len(names) + 0.5, title, ticks=tuple((index + 1.0, name) for index, name in enumerate(names)))


def _num(value: float) -> str:
    return f"{round(value, 6):g}"


def _usd_tick(value: float) -> str:
    return f"${round(value, 9):g}"


def esc(text: object) -> str:
    return html.escape(str(text), quote=True)


# ------------------------------------------------------------------------------------------------------------ figure

SVG_STYLE = """\
  <style>
    text { font-family: 'Helvetica Neue', Helvetica, Arial, 'Liberation Sans', Arimo, sans-serif; }
  </style>"""


class Figure(Output):
    """A stdlib SVG figure: panels on a canvas, each with its own axes."""

    def __init__(self, spec: Spec, dry_run: bool, width: int, height: int):
        super().__init__(spec, dry_run)
        self.width, self.height = width, height
        self.panels: list[Panel] = []

    def grid(self, rows: int, cols: int, *, top: int = 64, left: int = 72, right: int = 28, bottom: int = 56,
             hgap: int = 84, vgap: int = 84) -> list[tuple[float, float, float, float]]:
        """Plot-area boxes (x, y, w, h), row by row, leaving room for the stamp, titles, ticks and axis titles."""
        width = (self.width - left - right - hgap * (cols - 1)) / cols
        height = (self.height - top - bottom - vgap * (rows - 1)) / rows
        return [(left + col * (width + hgap), top + row * (height + vgap), width, height)
                for row in range(rows) for col in range(cols)]

    def panel(self, letter: str, title: str, box: tuple[float, float, float, float], x: Axis, y: Axis) -> Panel:
        panel = Panel(letter, title, box, x, y)
        self.panels.append(panel)
        return panel

    def body(self) -> tuple[str, str]:
        width, height = self.width, self.height
        out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} '
               f'{height}" role="img" aria-labelledby="title desc" data-figure="{esc(self.spec.id)}" '
               f'data-dry-run="{str(self.dry_run).lower()}">',
               f'  <title id="title">{esc(self.spec.id)}: {esc(self.spec.title)}</title>',
               f'  <desc id="desc">{esc(self.caption())}</desc>',
               SVG_STYLE,
               f'  <rect width="{width}" height="{height}" fill="#ffffff"/>']
        for panel in self.panels:
            out += panel.render()
        if self.dry_run:
            cx, cy = width / 2, height / 2
            out += ['  <g class="dry-run">',
                    f'    <rect x="0" y="0" width="{width}" height="22" fill="#d03b3b"/>',
                    f'    <text x="{cx:.1f}" y="15.5" font-size="12.5" font-weight="bold" text-anchor="middle" '
                    f'fill="#ffffff">{esc(DRY_RUN_STAMP)}</text>',
                    f'    <text x="{cx:.1f}" y="{cy:.1f}" font-size="44" font-weight="bold" text-anchor="middle" '
                    f'fill="#d03b3b" fill-opacity="0.13" transform="rotate(-20 {cx:.1f} {cy:.1f})">DRY RUN · '
                    'SYNTHETIC</text>',
                    '  </g>']
        return ".svg", "\n".join(out) + "\n</svg>\n"


class Panel:
    """One set of axes. Marks take data coordinates; `point` draws an arm's shape and fill."""

    def __init__(self, letter: str, title: str, box: tuple[float, float, float, float], x: Axis, y: Axis):
        self.letter, self.title, self.box, self.x, self.y = letter, title, box, x, y
        self.marks: list[str] = []
        self.notes: list[str] = []

    def px(self, value: float) -> float:
        return self.box[0] + self.x.frac(value) * self.box[2]

    def py(self, value: float) -> float:
        return self.box[1] + (1 - self.y.frac(value)) * self.box[3]

    def point(self, x: float, y: float, style: Style, *, label: str | None = None, tip: str | None = None,
              filled: bool | None = None, dx: float = 8, dy: float = -7) -> None:
        cx, cy = self.px(x), self.py(y)
        fill = style.colour if (style.filled if filled is None else filled) else "#ffffff"
        if style.shape == "diamond":
            shape = (f'<path d="M{cx:.1f},{cy - 6.5:.1f}L{cx + 6.5:.1f},{cy:.1f}L{cx:.1f},{cy + 6.5:.1f}'
                     f'L{cx - 6.5:.1f},{cy:.1f}Z"')
            tag = "path"
        else:
            shape, tag = f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="4.8"', "circle"
        title = f"<title>{esc(tip)}</title></{tag}>" if tip else "/>"
        self.marks.append(f'    {shape} fill="{fill}" stroke="{style.colour}" stroke-width="1.7"{">" if tip else ""}'
                          f'{title}')
        if label:
            self.text_px(cx + dx, cy + dy, label, colour=ink(style.colour))

    def whisker_y(self, x: float, lo: float, hi: float, colour: str, *, cap: float = 3.5) -> None:
        px, a, b = self.px(x), self.py(lo), self.py(hi)
        self.marks.append(f'    <path d="M{px:.1f},{a:.1f}V{b:.1f}M{px - cap:.1f},{a:.1f}H{px + cap:.1f}'
                          f'M{px - cap:.1f},{b:.1f}H{px + cap:.1f}" stroke="{colour}" stroke-width="1.2" fill="none"/>')

    def whisker_x(self, y: float, lo: float, hi: float, colour: str, *, cap: float = 3.5) -> None:
        py, a, b = self.py(y), self.px(lo), self.px(hi)
        self.marks.append(f'    <path d="M{a:.1f},{py:.1f}H{b:.1f}M{a:.1f},{py - cap:.1f}V{py + cap:.1f}M{b:.1f},'
                          f'{py - cap:.1f}V{py + cap:.1f}" stroke="{colour}" stroke-width="1.2" fill="none"/>')

    def line(self, points: Sequence[tuple[float, float]], colour: str, *, width: float = 1.6,
             dash: str | None = None, opacity: float = 1.0) -> None:
        if len(points) < 2:
            return
        coords = " ".join(f"{self.px(x):.1f},{self.py(y):.1f}" for x, y in points)
        extra = (f' stroke-dasharray="{dash}"' if dash else "")
        extra += f' stroke-opacity="{opacity}"' if opacity < 1 else ""
        self.marks.append(f'    <polyline points="{coords}" fill="none" stroke="{colour}" stroke-width="{width}"'
                          f'{extra}/>')

    def band(self, points: Sequence[tuple[float, float, float]], colour: str, *, opacity: float = 0.16) -> None:
        """A filled band through (x, lo, hi) points."""
        if len(points) < 2:
            return
        upper = [f"{self.px(x):.1f},{self.py(hi):.1f}" for x, _, hi in points]
        lower = [f"{self.px(x):.1f},{self.py(lo):.1f}" for x, lo, _ in reversed(points)]
        self.marks.append(f'    <polygon points="{" ".join(upper + lower)}" fill="{colour}" fill-opacity="{opacity}" '
                          'stroke="none"/>')

    def ref(self, *, x: float | None = None, y: float | None = None, label: str = "", colour: str = "#52514e") -> None:
        """A dashed reference line across the panel, labelled at its end."""
        x0, y0, w, h = self.box
        if y is not None:
            py = self.py(y)
            self.marks.append(f'    <path d="M{x0:.1f},{py:.1f}H{x0 + w:.1f}" stroke="{colour}" stroke-width="1" '
                              'stroke-dasharray="5 3" fill="none"/>')
            if label:
                self.text_px(x0 + w - 3, py - 4, label, colour=colour, anchor="end", size=10)
        if x is not None:
            px = self.px(x)
            self.marks.append(f'    <path d="M{px:.1f},{y0:.1f}V{y0 + h:.1f}" stroke="{colour}" stroke-width="1" '
                              'stroke-dasharray="5 3" fill="none"/>')
            if label:
                self.text_px(px + 4, y0 + 11, label, colour=colour, size=10)

    def shade(self, x0: float, x1: float, y0: float, y1: float, colour: str, *, opacity: float = 0.1,
              stroke: str | None = None) -> None:
        a, b = sorted((self.px(x0), self.px(x1)))
        c, d = sorted((self.py(y0), self.py(y1)))
        edge = f' stroke="{stroke}" stroke-width="0.8"' if stroke else ""
        self.marks.append(f'    <rect x="{a:.1f}" y="{c:.1f}" width="{b - a:.1f}" height="{d - c:.1f}" fill="{colour}" '
                          f'fill-opacity="{opacity}"{edge}/>')

    def hbar(self, y: float, x0: float, x1: float, colour: str, *, height: float = 14, label: str = "",
             hatch: bool = False) -> None:
        """A horizontal bar segment from x0 to x1 (stacked bars), centred on y."""
        a, b, py = self.px(x0), self.px(x1), self.py(y)
        fill = f'fill="{colour}"' if not hatch else f'fill="#ffffff" stroke="{colour}" stroke-dasharray="2 2"'
        self.marks.append(f'    <rect x="{a:.1f}" y="{py - height / 2:.1f}" width="{max(b - a, 0):.1f}" '
                          f'height="{height:.1f}" {fill}/>')
        if label and b - a > 24:
            self.text_px((a + b) / 2, py + 3.5, label, colour="#ffffff" if not hatch else "#0b0b0b",
                         anchor="middle", size=9.5)

    def text(self, x: float, y: float, text: str, *, colour: str = "#0b0b0b", anchor: str = "start",
             size: float = 10.5, dx: float = 0, dy: float = 0) -> None:
        self.text_px(self.px(x) + dx, self.py(y) + dy, text, colour=colour, anchor=anchor, size=size)

    def text_px(self, x: float, y: float, text: str, *, colour: str = "#0b0b0b", anchor: str = "start",
                size: float = 10.5) -> None:
        self.marks.append(f'    <text x="{x:.1f}" y="{y:.1f}" font-size="{size}" text-anchor="{anchor}" '
                          f'fill="{colour}">{esc(text)}</text>')

    def end_labels(self, items: Sequence[tuple[float, float, str, str]], *, gap: float = 11.5) -> None:
        """Direct labels at the ends of lines, (x, y, text, colour) each, moved apart vertically so none overlap."""
        placed = sorted(([self.px(x) + 9, self.py(y) + 3.5, text, colour] for x, y, text, colour in items),
                        key=lambda item: item[1])
        for previous, item in zip(placed, placed[1:]):
            item[1] = max(item[1], previous[1] + gap)
        overflow = placed[-1][1] - (self.box[1] + self.box[3]) if placed else 0
        for item in placed:
            item[1] -= max(overflow, 0)
        for x, y, text, colour in placed:
            self.text_px(x, y, text, colour=ink(colour), size=9.5)

    def note(self, text: str) -> None:
        """A line under the panel's x axis title, for what the panel leaves out."""
        self.notes.append(text)

    def render(self) -> list[str]:
        x0, y0, w, h = self.box
        out = [f'  <g class="panel" data-panel="{esc(self.letter)}">',
               f'    <text x="{x0:.1f}" y="{y0 - 12:.1f}" font-size="13" font-weight="bold">{esc(self.letter)}</text>'
               f'<text x="{x0 + 15:.1f}" y="{y0 - 12:.1f}" font-size="12">{esc(self.title)}</text>',
               f'    <rect x="{x0:.1f}" y="{y0:.1f}" width="{w:.1f}" height="{h:.1f}" fill="none" stroke="#bdbdbd"/>']
        for value, label in self.x.ticks:
            px = self.px(value)
            out.append(f'    <path d="M{px:.1f},{y0:.1f}V{y0 + h:.1f}" stroke="#ededed" stroke-width="1"/>'
                       f'<text x="{px:.1f}" y="{y0 + h + 14:.1f}" font-size="10" text-anchor="middle" '
                       f'fill="#52514e">{esc(label)}</text>')
        for value, label in self.y.ticks:
            py = self.py(value)
            out.append(f'    <path d="M{x0:.1f},{py:.1f}H{x0 + w:.1f}" stroke="#ededed" stroke-width="1"/>'
                       f'<text x="{x0 - 6:.1f}" y="{py + 3.5:.1f}" font-size="10" text-anchor="end" '
                       f'fill="#52514e">{esc(label)}</text>')
        x_title = self.x.title + (f" ({self.x.note})" if self.x.note else "")
        y_title = self.y.title + (f" ({self.y.note})" if self.y.note else "")
        out.append(f'    <text x="{x0 + w / 2:.1f}" y="{y0 + h + 32:.1f}" font-size="11" text-anchor="middle">'
                   f'{esc(x_title)}</text>')
        out.append(f'    <text x="{x0 - 48:.1f}" y="{y0 + h / 2:.1f}" font-size="11" text-anchor="middle" '
                   f'transform="rotate(-90 {x0 - 48:.1f} {y0 + h / 2:.1f})">{esc(y_title)}</text>')
        out += [f'    <text x="{x0:.1f}" y="{y0 + h + 46 + 12 * i:.1f}" font-size="9.5" fill="#52514e">{esc(note)}'
                '</text>' for i, note in enumerate(self.notes)]
        return out + self.marks + ["  </g>"]


# ------------------------------------------------------------------------------------------------------------- table

class Table(Output):
    """Markdown pipe tables, one per part, with a pandoc caption and footer lines."""

    def __init__(self, spec: Spec, dry_run: bool):
        super().__init__(spec, dry_run)
        self.parts: list[tuple[str, list[str], list[list[str]]]] = []
        self.footer: list[str] = []
        self.no_ci = False

    def part(self, heading: str, columns: Sequence[str]) -> list[list[str]]:
        """A new part; append its rows (lists of cell strings) to the list returned."""
        rows: list[list[str]] = []
        self.parts.append((heading, list(columns), rows))
        return rows

    def cell(self, rec: Rec | None, *, part: str, row: str, column: str, kind: str = "rate",
             with_ci: bool = True) -> str:
        """One cell from one record: "–" without a record, "n/a" for a null value, else the value with its CI."""
        if rec is None:
            return "–"
        value = self.take(rec, panel=part, series=row, role=column)
        if value is None:
            return "n/a"
        text = fmt(value, kind)
        if with_ci and rec.ci is not None:
            text += f" [{fmt(rec.ci[0], kind)}, {fmt(rec.ci[1], kind)}]"
        elif with_ci:
            text += " †"
            self.no_ci = True
        return text

    def body(self) -> tuple[str, str]:
        out = [f"> **{DRY_RUN_STAMP}.**", ""] if self.dry_run else []
        for heading, columns, rows in self.parts:
            if heading:
                out += [f"**{heading}**", ""]
            out.append("| " + " | ".join(map(_cell, columns)) + " |")
            out.append("|" + "---|" * len(columns))
            out += ["| " + " | ".join(map(_cell, row)) + " |" for row in rows]
            out.append("")
        out += [f": {self.caption()}", ""]
        notes = self.footer + (["† No interval: the record's `ci_method` is `none`."] if self.no_ci else [])
        out += [f"- {line}" for line in notes]
        return ".md", "\n".join(out).rstrip() + "\n"


def _cell(text: object) -> str:
    return str(text).replace("|", "\\|").replace("\n", " ")


def fmt(value: float, kind: str = "rate") -> str:
    """A number as the tables print it: rate and ratio to 3 places, usd with a dollar sign, count, pct, minutes."""
    if kind == "usd":
        return f"${value:.4f}" if abs(value) < 1 else f"${value:,.2f}"
    if kind == "count":
        return f"{value:g}" if isinstance(value, float) and not value.is_integer() else str(int(value))
    if kind == "pct":
        return f"{100 * value:.1f}%"
    if kind == "minutes":
        return f"{value / 60:.1f} min"
    return f"{value:.3f}"
