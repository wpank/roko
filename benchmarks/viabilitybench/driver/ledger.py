"""Prices and the run ledger: usage priced from the one snapshot, and an append-only row per attempt (S08 §4.10, §5.6).

`load_snapshot("prices-2026-09-28")` reads `config/prices/2026-09-28.toml` (the id `prices-<date>` resolves to
`config/prices/<date>.toml`) and validates it against `schema/price-snapshot.schema.json`. `Snapshot.row(model)`
finds a model's row by its slug; a dated variant such as `gpt-5.4-2026-03-05` falls back to its undated slug. A
model without a row has an unknown cost: `price` returns null amounts with source "unknown", never a fallback rate
(S08 D6, §4.12).

Usage has the run-record shape (`vb_usage`): disjoint token classes, with `tokens_in` counting only uncached input.
A cost is each class times its rate. Reasoning tokens are added only for a row whose `reasoning_in_output` is false;
otherwise they are already inside `tokens_out`. `without_cache_usd` prices cache reads as input.

`Ledger` appends one row per dispatched attempt to `ledger.jsonl`, validated against `schema/ledger.schema.json`,
and never rewrites the file. Each row names its budget line (S09 §4.6) and the worst-case cost reserved before the
attempt was dispatched. Caps per budget line, and refusing a dispatch that would break one, are gap-33d54b's
(S09 E2); `Ledger.spent_bound_usd` is what a run-level guard can check today.

API:
    DEFAULT_SNAPSHOT = "prices-2026-09-28"
    load_snapshot(snapshot_id: str = DEFAULT_SNAPSHOT) -> Snapshot           # raises PriceError
    Snapshot(id, rows); Snapshot.row(model: str | None) -> dict | None
    vb_usage(prompt_tokens, completion_tokens, cached_tokens=0, reasoning_tokens=0) -> dict
    add_usage(a: dict, b: dict) -> dict
    price(usage: dict | None, row: dict | None) -> Cost
    Cost(api_equiv_usd, without_cache_usd, source)
    worst_case_usd(row: dict | None, *, input_tokens: int, output_tokens: int) -> float | None
    Ledger(path, *, line, experiment_id, run_id, price_snapshot_id)
    Ledger.append(*, attempt_key, provider, model_reported, usage, cost, billed, reserved_usd) -> dict
    Ledger.spent_bound_usd -> float     # known costs, plus the reservation of every row whose cost is unknown
"""

from __future__ import annotations

import datetime as dt
import json
import os
import re
import tomllib
from dataclasses import dataclass
from pathlib import Path

import layout
import validate  # schema/validate.py, on sys.path through layout

DEFAULT_SNAPSHOT = "prices-2026-09-28"
SNAPSHOT_ID = re.compile(r"prices-(\d{4}-\d{2}-\d{2})")
DATED_SLUG = re.compile(r"(.+?)-\d{4}-?\d{2}-?\d{2}")
PER_TOKENS = 1_000_000


class PriceError(ValueError):
    """A snapshot id does not resolve, or its file is invalid."""


class LedgerError(ValueError):
    """A ledger row failed validation; nothing was written."""


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
    def __init__(self, path: Path, *, line: str, experiment_id: str, run_id: str, price_snapshot_id: str) -> None:
        self.path = Path(path)
        self.line = line
        self.experiment_id = experiment_id
        self.run_id = run_id
        self.price_snapshot_id = price_snapshot_id
        self.spent_bound_usd = 0.0

    def append(self, *, attempt_key: str, provider: str, model_reported: str | None, usage: dict | None, cost: Cost,
               billed: bool, reserved_usd: float) -> dict:
        """Validate and append one row. `billed` is false for subscription runs, whose billed cost is $0."""
        billed_usd = cost.api_equiv_usd if billed else 0.0
        row = {"ts": dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%SZ"), "line": self.line,
               "experiment_id": self.experiment_id, "run_id": self.run_id, "attempt_key": attempt_key,
               "provider": provider, "model_reported": model_reported, "usage": usage,
               "api_equiv_usd": cost.api_equiv_usd, "billed_usd": billed_usd, "reserved_usd": round(reserved_usd, 6),
               "price_snapshot_id": self.price_snapshot_id, "source": cost.source}
        errors = validate.validate("ledger", row)
        if errors:
            raise LedgerError(f"refusing to write an invalid ledger row: {errors[0]}")
        fd = os.open(self.path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
        with os.fdopen(fd, "a", encoding="utf-8") as handle:
            handle.write(json.dumps(row, sort_keys=True) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
        self.spent_bound_usd += cost.api_equiv_usd if cost.api_equiv_usd is not None else reserved_usd
        return row
