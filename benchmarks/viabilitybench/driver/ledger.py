"""Prices, the run ledger and its budget: priced usage, an append-only row per attempt, caps (S08 §4.10, §5.6; S09 E2).

`load_snapshot("prices-2026-09-28")` reads `config/prices/2026-09-28.toml` (the id `prices-<date>` resolves to
`config/prices/<date>.toml`) and validates it against `schema/price-snapshot.schema.json`. `Snapshot.row(model)`
finds a model's row by its slug; a dated variant such as `gpt-5.4-2026-03-05` falls back to its undated slug. A
model without a row has an unknown cost: `price` returns null amounts with source "unknown", never a fallback rate
(S08 D6, §4.12).

Usage has the run-record shape (`vb_usage`): disjoint token classes, with `tokens_in` counting only uncached input.
A cost is each class times its rate. Reasoning tokens are added only for a row whose `reasoning_in_output` is false;
otherwise they are already inside `tokens_out`. `without_cache_usd` prices cache reads as input.

`Ledger` appends one row per dispatched attempt to `ledger.jsonl`, validated against `schema/ledger.schema.json`,
and never rewrites the file. Each row names its budget line (S09 §4.6), the worst-case cost reserved before the
attempt was dispatched, and whether the account is billed for it (`billed`, false on a subscription).

**The budget** (`experiments/budget.toml`, read by `load_budget`) holds S09's budget lines with their caps, experiment
caps (the pilot's two experiment ids share $15) and the programme stop. Amounts are billed USD: a subscription run
bills $0, and a row whose cost is unknown counts at the worst case it reserved. A line's spend is summed over every
run in the results root (`$VB_RESULTS`, which holds `<experiment>/<run_id>/ledger.jsonl`), so a cap binds across
runs. Rows of offline runs count too, so dry runs belong in a scratch `--results`.
- `vb run` asks `Ledger.refusal` about each task's worst case before the task starts, and stops the run when the task
  could take spend past its line's cap, its experiment's cap or the programme stop.
- A runner reserves each attempt's worst case with `Ledger.reserve` before the attempt's first model call. It raises
  `BudgetError`, reserving nothing, when spent + reserved + worst case would pass a cap. The attempt's ledger row
  releases the reservation, and `Ledger.release` releases one whose attempt made no call.
- Reservations are appended to `reservations.jsonl` beside the run's ledger, under an exclusive lock on
  `$VB_RESULTS/.ledger.lock`, so concurrent runs count each other's. A reservation with neither a row nor a release
  (a run that died mid-attempt) stays counted, because its calls may have been billed. Once the provider's export
  shows what they cost, appending `{"event": "release", "attempt_key": ...}` to that file releases it.
- It fails closed: a budget file that does not load, a line the budget does not fund (BL12 is held back for D43) and
  an unreadable ledger line each refuse every dispatch.

`vb ledger report` shows spent, reserved and cap for each line and experiment cap, and the programme's totals. It
exits 1 when anything is over its cap (S09 SC3).

`vb ledger reconcile --provider cerebras --csv usage.csv` compares the ledger with a provider's usage export, per UTC
day and in total, and flags each measure (cost, input, cached and output tokens) that differs from the export's value
by more than 5% (S08 SC5); it exits 1 when one does. The export is a CSV with a header row. Its columns are found by
common names (`EXPORT_COLUMNS`) or named with `--column FIELD=HEADER`, and amounts in a currency other than USD are
refused. Only the provider's billed rows are compared, so the export must hold benchmark traffic alone: use a
dedicated key or project. zai and Moonshot exports work like Cerebras's and OpenAI's. For a model whose snapshot row
only infers `reasoning_in_output` (glm-4.7, kimi-k2.6), the result also says whether the export agrees: reasoning
billed outside the output tokens would show up as output and cost that the ledger does not have.

API:
    DEFAULT_SNAPSHOT = "prices-2026-09-28"
    load_snapshot(snapshot_id: str = DEFAULT_SNAPSHOT) -> Snapshot           # raises PriceError
    Snapshot(id, rows); Snapshot.row(model: str | None) -> dict | None
    vb_usage(prompt_tokens, completion_tokens, cached_tokens=0, reasoning_tokens=0) -> dict
    add_usage(a: dict, b: dict) -> dict
    price(usage: dict | None, row: dict | None) -> Cost
    Cost(api_equiv_usd, without_cache_usd, source)
    worst_case_usd(row: dict | None, *, input_tokens: int, output_tokens: int) -> float | None
    Ledger(path, *, line, experiment_id, run_id, price_snapshot_id, root=None, budget=None)
    Ledger.append(*, attempt_key, provider, model_reported, usage, cost, billed, reserved_usd) -> dict
    Ledger.spent_bound_usd -> float     # known costs, plus the reservation of every row whose cost is unknown
    Ledger.refusal(worst_usd) -> str | None; .reserve(attempt_key, worst_usd); .release(attempt_key)
    load_budget(path=None) -> Budget                                         # raises BudgetError
    read_books(root) -> Books; report(budget, books) -> dict; format_report(data, root) -> str
    read_export(path, columns=None) -> (rows, measures)                      # raises ReconcileError
    reconcile(rows, export, measures, *, provider, snapshot, ...) -> dict; format_reconcile(result) -> str
    add_parser(commands, default_results)                                    # vb ledger report|reconcile
"""

from __future__ import annotations

import argparse
import contextlib
import csv
import datetime as dt
import fcntl
import json
import math
import os
import re
import sys
import tomllib
from collections.abc import Callable, Iterator
from dataclasses import dataclass
from pathlib import Path

import layout
import validate  # schema/validate.py, on sys.path through layout

DEFAULT_SNAPSHOT = "prices-2026-09-28"
SNAPSHOT_ID = re.compile(r"prices-(\d{4}-\d{2}-\d{2})")
DATED_SLUG = re.compile(r"(.+?)-\d{4}-?\d{2}-?\d{2}")
PER_TOKENS = 1_000_000
BUDGET_FILE = layout.VB_ROOT / "experiments" / "budget.toml"
LINE_ID = re.compile(r"BL\d+")
LOCK_NAME = ".ledger.lock"  # in the results root
RESERVATIONS = "reservations.jsonl"  # beside each run's ledger.jsonl
TOLERANCE = 0.05  # S08 SC5: the ledger is within ±5% of the provider's usage export
REASONING_INFERRED = "reasoning_in_output is inferred"  # in the note of a snapshot row no provider document confirms
MEASURES = ("cost_usd", "input_tokens", "cached_tokens", "output_tokens")
# An export's columns, by normalized header name, in the order tried. input_tokens counts every prompt token, cached
# ones included, as OpenAI-compatible usage does.
EXPORT_COLUMNS = {
    "day": ("date", "day", "usage_date", "billing_date", "start_time_iso", "start_time_utc", "start_time",
            "timestamp", "time", "created_at"),
    "model": ("model", "model_name", "model_id"),
    "currency": ("currency", "amount_currency", "cost_currency"),
    "cost_usd": ("cost_usd", "cost", "amount_usd", "amount_value", "amount", "total_cost", "spend", "usd"),
    "input_tokens": ("input_tokens", "prompt_tokens", "tokens_in", "total_input_tokens", "input"),
    "cached_tokens": ("input_cached_tokens", "cached_tokens", "cached_input_tokens", "cache_read_tokens",
                      "cache_hit_tokens"),
    "output_tokens": ("output_tokens", "completion_tokens", "tokens_out", "total_output_tokens", "output"),
}
BUDGET_FIELDS = {"schema_version": "text", "source": "text", "programme": "table", "line": "tables"}
BUDGET_OPTIONAL = {"reserved": "table", "experiment": "tables"}
PROGRAMME_FIELDS = {"total_usd": "amount", "never_allocated_min_usd": "amount", "stop_usd": "amount"}
LINE_FIELDS = {"id": "text", "gate": "text", "content": "text", "planned_usd": "amount", "cap_usd": "amount"}
LINE_OPTIONAL = {"proposed_cap_usd": "amount", "was_cap_usd": "amount", "confirm": "text"}
EXPERIMENT_FIELDS = {"id": "text", "experiment_ids": "names", "lines": "names", "cap_usd": "amount"}
KINDS = {"text": "a string", "amount": "a number of at least 0", "names": "a non-empty list of strings",
         "table": "a table", "tables": "an array of tables"}


class PriceError(ValueError):
    """A snapshot id does not resolve, or its file is invalid."""


class LedgerError(ValueError):
    """A ledger row failed validation; nothing was written."""


class BudgetError(RuntimeError):
    """A dispatch would take billed spend past a cap, or the budget or a ledger line cannot be read."""


class ReconcileError(ValueError):
    """A provider's usage export cannot be read or compared."""


@dataclass(frozen=True)
class Snapshot:
    id: str
    rows: dict[str, dict]

    def row(self, model: str | None) -> dict | None:
        if not model:
            return None
        if model in self.rows:
            return self.rows[model]
        undated = DATED_SLUG.fullmatch(model)
        return self.rows.get(undated[1]) if undated else None


@dataclass(frozen=True)
class Cost:
    api_equiv_usd: float | None
    without_cache_usd: float | None
    source: str  # provider_usage or unknown


@dataclass(frozen=True)
class Line:
    id: str
    gate: str
    content: str
    planned_usd: float
    cap_usd: float
    proposed_cap_usd: float | None = None  # a raise that is not in force until the author agrees
    was_cap_usd: float | None = None  # the cap a v1.2 default lowered
    confirm: str | None = None  # "lock": a default the author confirms at the pre-registration lock


@dataclass(frozen=True)
class ExperimentCap:
    id: str
    experiment_ids: tuple[str, ...]  # the `vb run --experiment` ids it binds
    lines: tuple[str, ...]  # the only lines those runs may book to
    cap_usd: float


@dataclass(frozen=True)
class Budget:
    path: Path
    source: str
    lines: dict[str, Line]
    reserved: dict[str, str]  # line ids held back, with the reason (BL12: D43)
    experiments: tuple[ExperimentCap, ...]
    total_usd: float
    floor_usd: float  # never allocated (S09 SC3)
    stop_usd: float  # the programme stop

    @property
    def caps_usd(self) -> float:
        return round(sum(line.cap_usd for line in self.lines.values()), 6)

    @property
    def planned_usd(self) -> float:
        return round(sum(line.planned_usd for line in self.lines.values()), 6)

    def experiment(self, experiment_id: str) -> ExperimentCap | None:
        return next((cap for cap in self.experiments if experiment_id in cap.experiment_ids), None)


@dataclass(frozen=True)
class Books:
    """A results root's ledger rows, and its open reservations: dispatched attempts with no row or release yet."""

    rows: list[dict]
    reservations: list[dict]

    def sums(self, member: Callable[[dict], bool]) -> tuple[float, float]:
        """(spent, reserved) over the rows and open reservations that `member` selects."""
        return (sum(_charged(row) for row in self.rows if member(row)),
                sum(item["reserved_usd"] for item in self.reservations if member(item)))


def load_snapshot(snapshot_id: str = DEFAULT_SNAPSHOT) -> Snapshot:
    match = SNAPSHOT_ID.fullmatch(snapshot_id)
    if not match:
        raise PriceError(f"not a price snapshot id: {snapshot_id!r} (expected prices-YYYY-MM-DD)")
    path = layout.PRICES_DIR / f"{match[1]}.toml"
    try:
        with path.open("rb") as handle:
            doc = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as err:
        raise PriceError(f"{path}: {err}") from None
    errors = validate.validate("price-snapshot", doc)
    if errors or doc["id"] != snapshot_id:
        raise PriceError(f"{path}: {errors[0] if errors else 'its id is ' + repr(doc['id'])}")
    return Snapshot(id=snapshot_id, rows={row["slug"]: row for row in doc["model"]})


def vb_usage(prompt_tokens: int, completion_tokens: int, cached_tokens: int = 0, reasoning_tokens: int = 0) -> dict:
    """Provider usage in the run-record shape; the input classes are disjoint."""
    return {"tokens_in": prompt_tokens - cached_tokens, "tokens_out": completion_tokens,
            "tokens_cache_read": cached_tokens, "tokens_reasoning": reasoning_tokens}


def add_usage(a: dict, b: dict) -> dict:
    return {key: a.get(key, 0) + b.get(key, 0) for key in dict.fromkeys([*a, *b])}


def price(usage: dict | None, row: dict | None) -> Cost:
    """The cost of `usage` at `row`'s rates; unknown (null) when either is missing."""
    if usage is None or row is None:
        return Cost(None, None, "unknown")
    extra_out = 0 if row["reasoning_in_output"] else usage.get("tokens_reasoning", 0)
    paid = (usage["tokens_out"] + extra_out) * row["output"] \
        + usage.get("tokens_cache_write_5m", 0) * row["cache_write_5m"] \
        + usage.get("tokens_cache_write_1h", 0) * row["cache_write_1h"]
    api_equiv = usage["tokens_in"] * row["input"] + usage["tokens_cache_read"] * row["cache_read"] + paid
    without_cache = (usage["tokens_in"] + usage["tokens_cache_read"]) * row["input"] + paid
    return Cost(api_equiv / PER_TOKENS, without_cache / PER_TOKENS, "provider_usage")


def worst_case_usd(row: dict | None, *, input_tokens: int, output_tokens: int) -> float | None:
    """The most `input_tokens` of uncached input and `output_tokens` of output can cost; None without a price row."""
    if row is None:
        return None
    out_rate = row["output"] * (1 if row["reasoning_in_output"] else 2)
    return (input_tokens * row["input"] + output_tokens * out_rate) / PER_TOKENS


class Ledger:
    def __init__(self, path: Path, *, line: str, experiment_id: str, run_id: str, price_snapshot_id: str,
                 root: Path | None = None, budget: Budget | None = None) -> None:
        self.path = Path(path)
        self.line = line
        self.experiment_id = experiment_id
        self.run_id = run_id
        self.price_snapshot_id = price_snapshot_id
        # The results root whose runs share the budget: `vb run` writes <root>/<experiment>/<run_id>/ledger.jsonl.
        self.root = Path(root) if root is not None else self.path.absolute().parents[2]
        self.budget = budget  # None: experiments/budget.toml, read at every check
        self.spent_bound_usd = 0.0
        self._held: set[str] = set()  # attempts this ledger reserved for that have no row or release yet

    def append(self, *, attempt_key: str, provider: str, model_reported: str | None, usage: dict | None, cost: Cost,
               billed: bool, reserved_usd: float) -> dict:
        """Validate and append one row, which releases the attempt's reservation. `billed` is false for subscription
        runs, whose billed cost is $0."""
        billed_usd = cost.api_equiv_usd if billed else 0.0
        row = {"ts": _now(), "line": self.line, "experiment_id": self.experiment_id, "run_id": self.run_id,
               "attempt_key": attempt_key, "provider": provider, "model_reported": model_reported, "usage": usage,
               "api_equiv_usd": cost.api_equiv_usd, "billed_usd": billed_usd, "reserved_usd": round(reserved_usd, 6),
               "price_snapshot_id": self.price_snapshot_id, "source": cost.source, "billed": billed}
        errors = validate.validate("ledger", row)
        if errors:
            raise LedgerError(f"refusing to write an invalid ledger row: {errors[0]}")
        _append_json(self.path, row)
        self.spent_bound_usd += cost.api_equiv_usd if cost.api_equiv_usd is not None else reserved_usd
        self._held.discard(attempt_key)
        return row

    def refusal(self, worst_usd: float | None) -> str | None:
        """Why dispatching `worst_usd` more on this ledger's line would pass a cap, or None when it fits.

        `vb run` asks with a task's worst case before the task starts. None (a model without a price row, which only
        an offline run may use) adds nothing here; each attempt still reserves its own worst case.
        """
        try:
            return self._refusal(0.0 if worst_usd is None else worst_usd, read_books(self.root))
        except BudgetError as err:
            return str(err)

    def reserve(self, attempt_key: str, worst_usd: float) -> None:
        """Reserve an attempt's worst case before its first call. Raises BudgetError, reserving nothing, if it does not
        fit under every cap."""
        with _locked(self.root):
            reason = self._refusal(worst_usd, read_books(self.root))
            if reason:
                raise BudgetError(reason)
            _append_json(self.path.with_name(RESERVATIONS), {
                "ts": _now(), "event": "reserve", "attempt_key": attempt_key, "line": self.line,
                "experiment_id": self.experiment_id, "run_id": self.run_id, "reserved_usd": round(worst_usd, 6)})
        self._held.add(attempt_key)

    def release(self, attempt_key: str) -> None:
        """Release the reservation of an attempt that ended without a ledger row, because it made no model call."""
        if attempt_key in self._held:
            _append_json(self.path.with_name(RESERVATIONS),
                         {"ts": _now(), "event": "release", "attempt_key": attempt_key})
            self._held.discard(attempt_key)

    def _refusal(self, worst_usd: float, books: Books) -> str | None:
        if not _is_amount(worst_usd):
            return f"a dispatch's worst case must be an amount of at least $0, not {worst_usd!r}"
        budget = self.budget or load_budget()
        line = budget.lines.get(self.line)
        if line is None:
            held = budget.reserved.get(self.line)
            return (f"budget line {self.line} is held back ({held}) and funds nothing" if held else
                    f"{self.line} is not a budget line in {budget.path}")
        group = budget.experiment(self.experiment_id)
        if group and self.line not in group.lines:
            return (f"experiment {self.experiment_id} may book only to {', '.join(group.lines)} (experiment cap "
                    f"{group.id}), not to {self.line}")
        scopes = [(f"line {line.id}", line.cap_usd, lambda item: item["line"] == line.id)]
        if group:
            scopes.append((f"experiment cap {group.id}", group.cap_usd,
                           lambda item: item["experiment_id"] in group.experiment_ids))
        scopes.append(("the programme stop", budget.stop_usd, lambda item: True))
        for name, cap, member in scopes:
            spent, reserved = books.sums(member)
            if round(spent + reserved + worst_usd, 6) > cap:
                return (f"{name}: ${spent:.4f} spent + ${reserved:.4f} reserved + ${worst_usd:.4f} for this dispatch "
                        f"would pass ${cap:.2f}")
        return None


def load_budget(path: Path | None = None) -> Budget:
    """`experiments/budget.toml`, checked: known keys, amounts ≥ 0, planned ≤ cap, Σ caps ≤ stop ≤ total − floor."""
    path = Path(path or BUDGET_FILE)
    try:
        with path.open("rb") as handle:
            return _budget(path, tomllib.load(handle))
    except (OSError, ValueError) as err:  # a TOML decode error is a ValueError
        raise BudgetError(f"{path}: {err}") from None


def read_books(root: Path) -> Books:
    """Every ledger row under `root` (`<experiment>/<run_id>/ledger.jsonl`) and every open reservation.

    Raises BudgetError for a line that cannot be read or fails its schema. An unterminated last line is still being
    written: it is skipped, which leaves its attempt's reservation counted.
    """
    rows: list[dict] = []
    reservations: list[dict] = []
    run_dirs = {path.parent for name in ("ledger.jsonl", RESERVATIONS) for path in Path(root).glob(f"*/*/{name}")}
    for run_dir in sorted(run_dirs):
        run_rows = _read_jsonl(run_dir / "ledger.jsonl", lambda row: not validate.validate("ledger", row), "ledger row")
        pending: dict[str, dict] = {}
        for event in _read_jsonl(run_dir / RESERVATIONS, _event_ok, "reservation"):
            if event["event"] == "reserve":
                pending[event["attempt_key"]] = event
            else:
                pending.pop(event["attempt_key"], None)
        for row in run_rows:
            pending.pop(row["attempt_key"], None)
        rows += run_rows
        reservations += pending.values()
    return Books(rows, reservations)


def report(budget: Budget, books: Books) -> dict:
    """Spent, reserved and cap for every line and experiment cap, and the programme's totals (S09 SC3)."""

    def tally(cap: float, member: Callable[[dict], bool]) -> dict:
        spent, reserved = books.sums(member)
        return {"cap_usd": cap, "spent_usd": round(spent, 6), "reserved_usd": round(reserved, 6),
                "left_usd": round(cap - spent - reserved, 6), "rows": sum(map(member, books.rows)),
                "over_cap": round(spent + reserved, 6) > cap}

    lines = [{"id": line.id, "gate": line.gate, "content": line.content, "planned_usd": line.planned_usd,
              **tally(line.cap_usd, lambda item, line_id=line.id: item["line"] == line_id),
              "proposed_cap_usd": line.proposed_cap_usd, "was_cap_usd": line.was_cap_usd, "confirm": line.confirm}
             for line in budget.lines.values()]
    experiments = [{"id": cap.id, "experiment_ids": list(cap.experiment_ids), "lines": list(cap.lines),
                    **tally(cap.cap_usd, lambda item, ids=cap.experiment_ids: item["experiment_id"] in ids)}
                   for cap in budget.experiments]
    strays = sorted({item["line"] for item in [*books.rows, *books.reservations]} - set(budget.lines))
    unfunded = [{"id": line_id, "held_back": budget.reserved.get(line_id),
                 **tally(0.0, lambda item, line_id=line_id: item["line"] == line_id)} for line_id in strays]
    proposed = sum((line.proposed_cap_usd or line.cap_usd) - line.cap_usd for line in budget.lines.values())
    programme = {"total_usd": budget.total_usd, "never_allocated_min_usd": budget.floor_usd,
                 "planned_usd": budget.planned_usd, "caps_usd": budget.caps_usd,
                 "unallocated_usd": round(budget.total_usd - budget.caps_usd, 6),
                 "caps_with_proposals_usd": round(budget.caps_usd + proposed, 6),
                 **tally(budget.stop_usd, lambda item: True)}
    programme["stop_usd"] = programme.pop("cap_usd")
    over = [entry["id"] for entry in [*lines, *experiments] if entry["over_cap"]] + [entry["id"] for entry in unfunded]
    over += ["the programme stop"] if programme["over_cap"] else []
    return {"source": budget.source, "budget_file": str(budget.path), "lines": lines, "held_back": budget.reserved,
            "experiments": experiments, "unfunded": unfunded, "programme": programme, "over": over, "ok": not over}


def format_report(data: dict, root: Path) -> str:
    """`report`'s result as text: a row per line, then the experiment caps and the programme."""

    def row(name: str, planned: float | None, entry: dict, cap: float, notes: list[str]) -> str:
        planned_text = "" if planned is None else f"{planned:.2f}"
        note = "; ".join((["OVER CAP"] if entry["over_cap"] else []) + notes)
        return (f"{name:<15}{planned_text:>8}{cap:>9.2f}{entry['spent_usd']:>11.4f}{entry['reserved_usd']:>11.4f}"
                f"{cap - entry['spent_usd'] - entry['reserved_usd']:>11.4f}{entry['rows']:>6}  {note}").rstrip()

    out = [f"Budget: {data['source']}, {data['budget_file']}", f"Results: {root}", "",
           f"{'line':<15}{'planned':>8}{'cap':>9}{'spent':>11}{'reserved':>11}{'left':>11}{'rows':>6}  note"]
    for line in data["lines"]:
        notes = []
        if line["proposed_cap_usd"] is not None:
            notes.append(f"proposed cap ${line['proposed_cap_usd']:.2f} awaits the author")
        if line["confirm"] == "lock":
            was = "new" if line["was_cap_usd"] is None else f"was ${line['was_cap_usd']:.2f}"
            notes.append(f"v1.2 default ({was}), to be confirmed at the lock")
        out.append(row(f"{line['id']:<6}{line['gate']}", line["planned_usd"], line, line["cap_usd"], notes))
    for line_id, why in data["held_back"].items():
        out.append(f"{line_id:<6}held back, funds nothing: {why}")
    for stray in data["unfunded"]:
        out.append(row(stray["id"], None, stray, 0.0, ["rows on a line the budget does not fund"]))
    programme = data["programme"]
    total = {**programme, "over_cap": False}
    out.append(row("all lines", programme["planned_usd"], total, programme["caps_usd"], []))
    if data["experiments"]:
        out += ["", f"{'experiment cap':<23}{'cap':>9}{'spent':>11}{'reserved':>11}{'left':>11}{'rows':>6}  runs"]
        for cap in data["experiments"]:
            out.append(row(cap["id"], None, cap, cap["cap_usd"],
                           [f"{', '.join(cap['experiment_ids'])} on {', '.join(cap['lines'])}"]))
    out += ["", f"Programme: ${programme['spent_usd']:.4f} spent and ${programme['reserved_usd']:.4f} reserved of "
                f"the ${programme['stop_usd']:.2f} stop. The caps sum to ${programme['caps_usd']:.2f} of "
                f"${programme['total_usd']:.2f}, leaving ${programme['unallocated_usd']:.2f} never allocated (at least "
                f"${programme['never_allocated_min_usd']:.2f}); with the proposed caps they would sum to "
                f"${programme['caps_with_proposals_usd']:.2f}.",
            "OK: nothing is over its cap" if data["ok"] else f"OVER CAP: {', '.join(data['over'])}"]
    return "\n".join(out)


def read_export(path: Path, columns: dict[str, str] | None = None) -> tuple[list[dict], tuple[str, ...]]:
    """A usage export's rows as {"day", "model", <measure>: number}, and the measures it has.

    `columns` maps a field of `EXPORT_COLUMNS` to the export's header for it; other fields are found by name. Rows
    with no date, or "total" in its place, are skipped. Raises ReconcileError.
    """
    rows = []
    try:
        with Path(path).open(newline="", encoding="utf-8-sig") as handle:
            reader = csv.DictReader(handle)
            headers = {_header(name): name for name in reader.fieldnames or ()}
            chosen: dict[str, str] = {}
            for name, aliases in EXPORT_COLUMNS.items():
                wanted = (columns or {}).get(name)
                if wanted is not None and _header(wanted) not in headers:
                    raise ReconcileError(f"{path}: no column {wanted!r} for {name}; it has "
                                         f"{', '.join(headers.values())}")
                found = _header(wanted) if wanted is not None else next((a for a in aliases if a in headers), None)
                if found:
                    chosen[name] = headers[found]
            measures = tuple(measure for measure in MEASURES if measure in chosen)
            if "day" not in chosen or not measures:
                raise ReconcileError(f"{path}: needs a date column and a cost or token column, and has "
                                     f"{', '.join(headers.values()) or 'no header'}; name them with --column")
            for number, record in enumerate(reader, 2):
                where = f"{path}:{number}"
                stamp = (record.get(chosen["day"]) or "").strip()
                if not stamp or stamp.lower() in ("total", "totals"):
                    continue
                currency = (record.get(chosen["currency"]) or "").strip() if "currency" in chosen else ""
                if currency and currency.upper() != "USD":
                    raise ReconcileError(f"{where}: amounts in {currency}; the price snapshot is in USD, so export USD")
                model = (record.get(chosen["model"]) or "").strip() if "model" in chosen else ""
                rows.append({"day": _utc_day(stamp, where), "model": model or None,
                             **{measure: _number(record.get(chosen[measure]), where) for measure in measures}})
    except OSError as err:
        raise ReconcileError(f"{path}: {err}") from None
    except (csv.Error, UnicodeDecodeError) as err:
        raise ReconcileError(f"{path}: not a readable CSV file: {err}") from None
    if not rows:
        raise ReconcileError(f"{path}: no usage rows")
    return rows, measures


def reconcile(rows: list[dict], export: list[dict], measures: tuple[str, ...], *, provider: str, snapshot: Snapshot,
              experiments: tuple[str, ...] = (), since: str | None = None, until: str | None = None,
              by_model: bool = False, tolerance: float = TOLERANCE) -> dict:
    """Compare a provider's billed ledger rows with its usage export, per UTC day (and model) and in total.

    The window is the export's days, narrowed by `since` and `until`; ledger rows outside it are left out, and a day
    only one side has is compared against zero. Subscription rows are never on a provider's bill, so they are left
    out too.
    """
    if by_model and not all(item["model"] for item in export):
        raise ReconcileError("--by-model needs a model on every row of the export")
    days = sorted(item["day"] for item in export)
    if not days:
        raise ReconcileError("the export has no usage rows")
    first, last = max(days[0], since or days[0]), min(days[-1], until or days[-1])
    if first > last:  # nothing left to compare must never read as agreement
        raise ReconcileError(f"the export's days, {days[0]} to {days[-1]}, all lie outside --since and --until")

    def key(day: str, model: str | None) -> tuple[str, str]:
        return day, _model_key(model, snapshot) if by_model else "*"

    theirs: dict[tuple[str, str], dict[str, float]] = {}
    for item in export:
        if first <= item["day"] <= last:
            bucket = theirs.setdefault(key(item["day"], item["model"]), dict.fromkeys(measures, 0.0))
            for measure in measures:
                bucket[measure] += item[measure]
    ours: dict[tuple[str, str], dict[str, float]] = {}
    counted = unknown = 0
    for row in rows:
        day = row["ts"][:10]
        if row["provider"] != provider or _subscription(row) or not first <= day <= last \
                or (experiments and row["experiment_id"] not in experiments):
            continue
        counted += 1
        unknown += row["usage"] is None or row["api_equiv_usd"] is None
        bucket = ours.setdefault(key(day, row["model_reported"]), dict.fromkeys(measures, 0.0))
        for measure, value in _ledger_measures(row, snapshot).items():
            if measure in bucket:
                bucket[measure] += value
    zero = dict.fromkeys(measures, 0.0)
    comparisons = [{"day": day, "model": model, "measure": measure,
                    **_compare(ours.get((day, model), zero)[measure], theirs.get((day, model), zero)[measure],
                               tolerance)}
                   for day, model in sorted(set(ours) | set(theirs)) for measure in measures]
    totals = [{"day": "total", "model": "*", "measure": measure,
               **_compare(sum(bucket[measure] for bucket in ours.values()),
                          sum(bucket[measure] for bucket in theirs.values()), tolerance)} for measure in measures]
    notes = [f"{unknown} ledger row(s) have no reported usage or cost, so the export exceeds the ledger by what they "
             "cost"] if unknown else []
    for slug, price_row in snapshot.rows.items():
        if price_row["provider"] == provider and REASONING_INFERRED in price_row.get("note", ""):
            notes.append(_reasoning_note(slug, totals, counted, snapshot.id, tolerance))
    flagged = sum(entry["flagged"] for entry in [*comparisons, *totals])
    return {"provider": provider, "window": [first, last], "tolerance": tolerance, "measures": list(measures),
            "by_model": by_model, "ledger_rows": counted, "unknown_rows": unknown, "comparisons": comparisons,
            "totals": totals, "notes": notes, "flagged": flagged, "ok": flagged == 0}


def format_reconcile(result: dict) -> str:
    """`reconcile`'s result as text: one row per day, model and measure, then the totals and the notes."""
    first, last = result["window"]
    out = [f"Reconcile {result['provider']}, {first} to {last}: {result['ledger_rows']} billed ledger row(s) against "
           f"the export; drift above ±{result['tolerance']:.0%} is flagged", "",
           f"{'day':<12}{'model':<18}{'measure':<15}{'ledger':>16}{'export':>16}{'drift':>9}"]
    for entry in [*result["comparisons"], *result["totals"]]:
        amount = "{:,.6f}" if entry["measure"] == "cost_usd" else "{:,.0f}"
        drift = "n/a" if entry["drift"] is None else f"{entry['drift']:+.1%}"
        out.append(f"{entry['day']:<12}{entry['model']:<18}{entry['measure']:<15}{amount.format(entry['ledger']):>16}"
                   f"{amount.format(entry['export']):>16}{drift:>9}{'  FLAG' if entry['flagged'] else ''}")
    out += ["", *(f"Note: {note}" for note in result["notes"]),
            f"FLAGGED: {result['flagged']} comparison(s) differ by more than {result['tolerance']:.0%}"
            if result["flagged"] else f"OK: every comparison is within ±{result['tolerance']:.0%}"]
    return "\n".join(out)


def add_parser(commands: argparse._SubParsersAction, default_results: Path) -> None:
    """Add `vb ledger report|reconcile` to vb's subcommands; `default_results` is vb's default results root."""
    parser = commands.add_parser("ledger", help="budget lines: spend against caps, and reconciliation",
                                 allow_abbrev=False)
    actions = parser.add_subparsers(dest="ledger_command", required=True)
    results = f"the results root (default: $VB_RESULTS, then {default_results})"

    shown = actions.add_parser("report", help="spent, reserved and cap for every budget line", allow_abbrev=False)
    shown.add_argument("--results", type=Path, help=results)
    shown.add_argument("--budget", type=Path, help="the budget file (default: experiments/budget.toml)")
    shown.add_argument("--json", action="store_true", help="print JSON")
    shown.set_defaults(handler=cmd_report, default_results=default_results)

    checked = actions.add_parser("reconcile", help="compare the ledger with a provider's usage export",
                                 allow_abbrev=False)
    checked.add_argument("--provider", required=True, help="a provider in the price snapshot (cerebras, openai, zai, "
                                                           "moonshot, ...)")
    checked.add_argument("--csv", required=True, type=Path, help="the provider's usage export, in USD")
    checked.add_argument("--results", type=Path, help=results)
    checked.add_argument("--experiment", action="append", default=[], help="count only this experiment (repeatable)")
    checked.add_argument("--since", type=_day_arg, help="the first UTC day, YYYY-MM-DD (default: the export's)")
    checked.add_argument("--until", type=_day_arg, help="the last UTC day, YYYY-MM-DD (default: the export's)")
    checked.add_argument("--by-model", action="store_true", help="compare each model as well as each day")
    checked.add_argument("--column", action="append", default=[], type=_column_arg, metavar="FIELD=HEADER",
                         help=f"the export's column for a field: {', '.join(EXPORT_COLUMNS)} (repeatable)")
    checked.add_argument("--measure", action="append", choices=MEASURES, help="compare only this measure (repeatable)")
    checked.add_argument("--tolerance", type=float, default=TOLERANCE, help="flag drift above this share (0.05)")
    checked.add_argument("--price-snapshot", default=DEFAULT_SNAPSHOT)
    checked.add_argument("--json", action="store_true", help="print JSON")
    checked.set_defaults(handler=cmd_reconcile, default_results=default_results)


def cmd_report(args: argparse.Namespace) -> int:
    root = _results_root(args)
    try:
        data = report(load_budget(args.budget), read_books(root))
    except BudgetError as err:
        print(f"vb: {err}", file=sys.stderr)
        return 2
    print(json.dumps(data, indent=2, ensure_ascii=False) if args.json else format_report(data, root))
    return 0 if data["ok"] else 1


def cmd_reconcile(args: argparse.Namespace) -> int:
    try:
        snapshot = load_snapshot(args.price_snapshot)
        providers = sorted({row["provider"] for row in snapshot.rows.values()})
        if args.provider not in providers:
            raise ReconcileError(f"--provider must be one of {', '.join(providers)}, the providers in {snapshot.id}")
        export, measures = read_export(args.csv, dict(args.column))
        missing = sorted(set(args.measure or ()) - set(measures))
        if missing:
            raise ReconcileError(f"{args.csv} has no column for {', '.join(missing)}")
        result = reconcile(read_books(_results_root(args)).rows, export,
                           tuple(m for m in measures if m in (args.measure or measures)), provider=args.provider,
                           snapshot=snapshot, experiments=tuple(args.experiment), since=args.since,
                           until=args.until, by_model=args.by_model, tolerance=args.tolerance)
    except (BudgetError, PriceError, ReconcileError) as err:
        print(f"vb: {err}", file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2, ensure_ascii=False) if args.json else format_reconcile(result))
    return 0 if result["ok"] else 1


def _budget(path: Path, doc: dict) -> Budget:
    _fields(doc, "the file", BUDGET_FIELDS, BUDGET_OPTIONAL)
    if doc["schema_version"] != "vb.budget/1":
        raise ValueError(f"schema_version must be \"vb.budget/1\", not {doc['schema_version']!r}")
    programme = _fields(doc["programme"], "[programme]", PROGRAMME_FIELDS)
    lines: dict[str, Line] = {}
    for number, table in enumerate(doc["line"], 1):
        line = Line(**_fields(table, f"[[line]] #{number}", LINE_FIELDS, LINE_OPTIONAL))
        if not LINE_ID.fullmatch(line.id) or line.id in lines:
            raise ValueError(f"line {line.id!r}: a line id is BL<n>, and each appears once")
        if line.planned_usd > line.cap_usd or (line.proposed_cap_usd or line.cap_usd) < line.cap_usd:
            raise ValueError(f"{line.id}: its planned amount must fit its cap, and a proposed cap must raise it")
        if line.confirm not in (None, "lock"):
            raise ValueError(f"{line.id}: confirm must be \"lock\", not {line.confirm!r}")
        lines[line.id] = line
    reserved = _fields(doc.get("reserved", {}), "[reserved]", dict.fromkeys(doc.get("reserved", {}), "text"))
    for line_id in reserved:
        if not LINE_ID.fullmatch(line_id) or line_id in lines:
            raise ValueError(f"[reserved] {line_id}: a held-back id is BL<n> and funds no line")
    experiments: list[ExperimentCap] = []
    for number, table in enumerate(doc.get("experiment", []), 1):
        values = _fields(table, f"[[experiment]] #{number}", EXPERIMENT_FIELDS)
        cap = ExperimentCap(values["id"], tuple(values["experiment_ids"]), tuple(values["lines"]), values["cap_usd"])
        if [line_id for line_id in cap.lines if line_id not in lines] or \
                [e for e in cap.experiment_ids for other in experiments if e in other.experiment_ids]:
            raise ValueError(f"experiment cap {cap.id}: its lines must be funded lines, and no other experiment cap "
                             "may list its experiment ids")
        experiments.append(cap)
    budget = Budget(path, doc["source"], lines, reserved, tuple(experiments), programme["total_usd"],
                    programme["never_allocated_min_usd"], programme["stop_usd"])
    if not lines or budget.caps_usd > budget.stop_usd or budget.stop_usd > budget.total_usd - budget.floor_usd:
        raise ValueError(f"the caps sum to ${budget.caps_usd:g}: they must fit the ${budget.stop_usd:g} stop, which "
                         f"must leave ${budget.floor_usd:g} of ${budget.total_usd:g} never allocated (S09 SC3)")
    return budget


def _fields(table: object, where: str, required: dict[str, str], optional: dict[str, str] | None = None) -> dict:
    """`table`'s values, with its keys and their kinds (`KINDS`) checked; amounts come back as floats."""
    if not isinstance(table, dict):
        raise ValueError(f"{where} must be a table")
    kinds = {**required, **(optional or {})}
    problems = [f"unknown key {key}" for key in table if key not in kinds]
    problems += [f"missing {key}" for key in required if key not in table]
    values = {}
    for key, value in table.items():
        kind = kinds.get(key)
        ok = {None: True, "text": isinstance(value, str), "amount": _is_amount(value),
              "names": isinstance(value, list) and bool(value) and all(isinstance(item, str) for item in value),
              "table": isinstance(value, dict),
              "tables": isinstance(value, list) and all(isinstance(item, dict) for item in value)}[kind]
        if not ok:
            problems.append(f"{key} must be {KINDS[kind]}, not {value!r}")
        values[key] = float(value) if kind == "amount" and ok else value
    if problems:
        raise ValueError(f"{where}: {'; '.join(problems)}")
    return values


def _is_amount(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value >= 0


def _charged(row: dict) -> float:
    """What a ledger row counts against a cap: its billed cost, or its reservation when that cost is unknown."""
    return row["billed_usd"] if row["billed_usd"] is not None else row["reserved_usd"]


def _event_ok(event: dict) -> bool:
    if event.get("event") == "release":
        return isinstance(event.get("attempt_key"), str)
    return event.get("event") == "reserve" and _is_amount(event.get("reserved_usd")) and all(
        isinstance(event.get(key), str) for key in ("attempt_key", "line", "experiment_id"))


def _read_jsonl(path: Path, check: Callable[[dict], bool], what: str) -> list[dict]:
    """The complete lines of `path` ([] when it does not exist); raises BudgetError for one that is not a `what`."""
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return []
    except (OSError, UnicodeDecodeError) as err:
        raise BudgetError(f"{path}: {err}") from None
    items = []
    for number, line in enumerate(text.split("\n")[:-1], 1):  # the text after the last newline is still being written
        if not line.strip():
            continue
        try:
            item = json.loads(line)
        except json.JSONDecodeError as err:
            raise BudgetError(f"{path}:{number}: an unreadable {what}: {err}") from None
        if not isinstance(item, dict) or not check(item):
            raise BudgetError(f"{path}:{number}: not a valid {what}")
        items.append(item)
    return items


def _append_json(path: Path, value: dict) -> None:
    """Append one JSON line and fsync it; the file is created with mode 0600 and never rewritten."""
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    with os.fdopen(fd, "a", encoding="utf-8") as handle:
        handle.write(json.dumps(value, sort_keys=True) + "\n")
        handle.flush()
        os.fsync(handle.fileno())


@contextlib.contextmanager
def _locked(root: Path) -> Iterator[None]:
    """Hold the results root's exclusive lock, so that a reservation is checked and written as one step."""
    fd = os.open(Path(root) / LOCK_NAME, os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX)
        yield
    finally:
        os.close(fd)  # closing the descriptor releases the lock


def _now() -> str:
    return dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")


def _subscription(row: dict) -> bool:
    """A row that a subscription paid for: marked `billed: false`, or, in a row from before the mark (bug-a49003),
    one that bills $0 while its API-equivalent cost is above $0 or unknown."""
    if "billed" in row:
        return row["billed"] is False
    return row["billed_usd"] == 0 and row["api_equiv_usd"] != 0


def _ledger_measures(row: dict, snapshot: Snapshot) -> dict[str, float]:
    """A ledger row's cost and token counts as a provider bills them; unknown usage or cost adds nothing."""
    usage = row["usage"] or {}
    price_row = snapshot.row(row["model_reported"])
    reasoning = usage.get("tokens_reasoning", 0) if price_row and not price_row["reasoning_in_output"] else 0
    cached = usage.get("tokens_cache_read", 0)
    prompt = usage.get("tokens_in", 0) + cached + usage.get("tokens_cache_write_5m", 0) \
        + usage.get("tokens_cache_write_1h", 0)
    return {"cost_usd": row["api_equiv_usd"] or 0.0, "input_tokens": prompt, "cached_tokens": cached,
            "output_tokens": usage.get("tokens_out", 0) + reasoning}


def _compare(ledger_value: float, export_value: float, tolerance: float) -> dict:
    """The drift (ledger − export) / export, flagged above `tolerance`; with nothing exported, any ledger value is."""
    if export_value:
        drift = (ledger_value - export_value) / export_value
        flagged = abs(drift) > tolerance
    else:
        drift, flagged = (None, True) if ledger_value else (0.0, False)
    return {"ledger": round(ledger_value, 9), "export": round(export_value, 9),
            "drift": None if drift is None else round(drift, 6), "flagged": flagged}


def _reasoning_note(slug: str, totals: list[dict], counted: int, snapshot_id: str, tolerance: float) -> str:
    """Whether the export agrees with a `reasoning_in_output = true` that the snapshot only infers."""
    total = next((entry for measure in ("output_tokens", "cost_usd") for entry in totals
                  if entry["measure"] == measure), None)
    claim = f"{snapshot_id} infers reasoning_in_output = true for {slug}"
    if total is None or not counted:
        return f"{claim}; not checked, since the window has no ledger rows or the export no output or cost column"
    if not total["flagged"]:
        return f"{claim}, and the export agrees: {total['measure']} is within ±{tolerance:.0%}"
    if total["export"] > total["ledger"]:
        return (f"{claim}, which may be wrong: the export bills more {total['measure']} than the ledger records "
                f"(ledger {total['drift']:+.1%}), as it would if reasoning were billed outside the output tokens")
    return f"{claim}; the ledger records more {total['measure']} than the export, which reasoning cannot explain"


def _header(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", name.strip().lower()).strip("_")


def _utc_day(text: str, where: str) -> str:
    """The UTC day of a date, an ISO 8601 timestamp (its offset applied; none means UTC) or unix seconds."""
    try:
        if re.fullmatch(r"\d{9,13}(\.\d+)?", text):
            seconds = float(text) / (1000 if len(text.split(".")[0]) > 11 else 1)  # 13 digits are milliseconds
            return dt.datetime.fromtimestamp(seconds, dt.UTC).date().isoformat()
        stamp = dt.datetime.fromisoformat(text.replace("/", "-"))
    except (ValueError, OverflowError, OSError):
        raise ReconcileError(f"{where}: cannot read the date {text!r}; use ISO 8601 or unix seconds") from None
    return (stamp.astimezone(dt.UTC) if stamp.tzinfo else stamp).date().isoformat()


def _number(text: str | None, where: str) -> float:
    cleaned = (text or "").strip().replace(",", "").replace("$", "")
    try:
        value = float(cleaned) if cleaned else 0.0
    except ValueError:
        raise ReconcileError(f"{where}: {text!r} is not a number") from None
    if not math.isfinite(value):
        raise ReconcileError(f"{where}: {text!r} is not a finite number")
    return value


def _model_key(model: str | None, snapshot: Snapshot) -> str:
    """A model name both sides can match: the snapshot's slug when it has a row, else lower case and undated."""
    name = (model or "?").strip().lower()
    row = snapshot.row(name)
    undated = DATED_SLUG.fullmatch(name)
    return row["slug"] if row else undated[1] if undated else name


def _results_root(args: argparse.Namespace) -> Path:
    return Path(args.results or os.environ.get("VB_RESULTS") or args.default_results).expanduser().absolute()


def _day_arg(text: str) -> str:
    if re.fullmatch(r"\d{4}-\d{2}-\d{2}", text):
        with contextlib.suppress(ValueError):
            return dt.date.fromisoformat(text).isoformat()
    raise argparse.ArgumentTypeError("use a day as YYYY-MM-DD")


def _column_arg(text: str) -> tuple[str, str]:
    name, sep, header = text.partition("=")
    if not sep or name not in EXPORT_COLUMNS or not header.strip():
        raise argparse.ArgumentTypeError(f"use FIELD=HEADER, with FIELD one of {', '.join(EXPORT_COLUMNS)}")
    return name, header
