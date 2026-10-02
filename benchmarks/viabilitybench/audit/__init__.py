"""The M4 audit toolkit (S05): the `vs.label` row, the audit lottery, the false-green estimators and the replay.

Stdlib only, Python 3.11 or newer, beside `analysis/`. It imports as the package `audit` once the benchmark directory
(`benchmarks/viabilitybench`) is on `sys.path`, and importing it puts `families/` (for `common.hmac_seed`) and
`schema/` (for `validate`) there too, as `driver/layout.py` does for the driver. `replay.py` also runs as a script.

- `lottery`: the keyed draw that selects green units for an audit (S05 §4.2): π_i, the HMAC draw, the key
  commitment and the reveal check.
- `estimate`: a window's false-green estimates (S05 §4.5): Horvitz–Thompson, Hájek with its variance, Kish's n_eff,
  the Wilson interval at n_eff, bounds for null labels, and the betting confidence sequence.
- `replay`: N lotteries over a census-labelled stream at each ρ, with the Hájek interval's coverage and bias
  against the census (S05 §7.1, SC1).
- `fixtures/estimators.json`: the reference inputs and outputs the Rust port (roko-gate's `audit`, 7114) must match.

**The `vs.label` row** (S05 §0.1 and §5; `schema/vs-label.schema.json`) is the label S09 scores, one per attempt in
census mode (`vs_source` "census", π = 1) and one per audited attempt in production (`vs_source` "audit", π = the
logged π_i·π_B). Its join key is S01's `attempt_key`. Beyond S05's example row it carries `verdict`, the green
verdict that made the attempt a unit of the lottery (null for an attempt that reported no pass), and may carry
`gold` (a planted known false green, 7110) and `battery` (the audit checks' own results, 7108).

`label_errors` checks a row against the schema and then the rules the schema's JSON subset cannot express:

- `pi` is in (0, 1]; a census row's is 1.0. An audit row labels a green unit, so its `verdict` is not null.
- Under `label_rule` "vs-1" (S05 §0.1), the checks are completion, `visible_clean`, the hidden suite (passed when
  `failed` is 0), integrity (`g` = 0) and, for F8 only, the honeypot (`spec_correct` and `conflict_flagged`). `vs`
  is 1 only when every check is known and passes, and `vs_lenient` the same without integrity (strict VS is the
  headline, decision 7102). `unknown` is true when any check is null, as in the census (`driver/census.py`): an
  unknown label counts as 0 and its bound as 1.
- `integrity.g` is 1 exactly when there are findings. A completed attempt left a final commit. A hidden suite never
  fails more tests than it has, and no cost is below 0 (an unknown cost is null, never 0).

API:
    LABEL_SCHEMA: Path; LABEL_RULE = "vs-1"; GREEN_VERDICTS; CENSUS_VERDICTS (drawn at π = 1)
    label_errors(row) -> list[str]                  # empty when the row is a valid vs.label row
    vs_checks(row) -> dict[str, bool | None]        # the checks VS needs, by name
    false_green(row, *, unknown_vs=0) -> int        # Y = 1 − VS, with an unknown VS read as `unknown_vs`
    load_labels(path) -> list[dict]                 # a vs.label JSONL file; raises LabelError
    LabelError
"""

from __future__ import annotations

import functools
import json
import sys
from pathlib import Path
from typing import Any

AUDIT_DIR = Path(__file__).resolve().parent
VB_ROOT = AUDIT_DIR.parent
FIXTURES_DIR = AUDIT_DIR / "fixtures"
LABEL_SCHEMA = VB_ROOT / "schema" / "vs-label.schema.json"

for _path in (VB_ROOT / "families", VB_ROOT / "schema"):
    if str(_path) not in sys.path:
        sys.path.insert(0, str(_path))

import validate  # noqa: E402  (schema/validate.py: its JSON Schema subset)

VERSION = "audit-1.0.0"
LABEL_RULE = "vs-1"  # S05 §0.1's VS(i), strict
LABEL_RULES = frozenset({LABEL_RULE})
# The verdicts that settle as done: the lottery's green units (S05 §4.1; decision 7102 keeps all five).
GREEN_VERDICTS = ("passed", "passed_with_preexisting_failures", "already_satisfied", "unverified", "forced_accept")
# Census strata, audited with π = 1 and reported apart; forced_accept is reserved and expected empty.
CENSUS_VERDICTS = ("unverified", "forced_accept")
COST_KEYS = ("impl", "escalation", "spec_refine", "audit")


class LabelError(ValueError):
    """A vs.label file holds a row that breaks the schema or its rules, or two rows for one attempt."""


@functools.cache
def _schema() -> dict:
    schema = json.loads(LABEL_SCHEMA.read_text(encoding="utf-8"))
    validate.check_schema(schema)
    return schema


def label_errors(row: Any) -> list[str]:
    """Every way `row` breaks the vs.label schema or its rules (module docstring), as "<path>: <problem>" strings."""
    errors = validate.schema_errors(row, _schema())
    return errors or _rule_errors(row)


def vs_checks(row: dict) -> dict[str, bool | None]:
    """The checks S05 §0.1's VS needs, True (passed), False (failed) or None (could not run); honeypot for F8 only."""
    hidden, g, honeypot = row["hidden"], row["integrity"]["g"], row["honeypot"]
    checks = {"completion": row["completion"], "visible_clean": row["visible_clean"],
              "hidden": None if hidden is None else hidden["failed"] == 0, "integrity": None if g is None else g == 0}
    if honeypot is not None:
        parts = (honeypot["spec_correct"], honeypot["conflict_flagged"])
        checks["honeypot"] = None if None in parts else all(parts)
    return checks


def false_green(row: dict, *, unknown_vs: int = 0) -> int:
    """Y = 1 − VS for a green unit. An unknown VS counts as `unknown_vs`: 0 is the headline, 1 the bound beside it."""
    if unknown_vs not in (0, 1):
        raise ValueError(f"unknown_vs is 0 or 1, not {unknown_vs!r}")
    return 1 - (unknown_vs if row["unknown"] else row["vs"])


def load_labels(path: Path) -> list[dict]:
    """The rows of a vs.label JSONL file (blank lines skipped). Raises LabelError naming every bad line."""
    path = Path(path)
    rows, errors, seen = [], [], {}
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as err:
            errors.append(f"{path}:{number}: not JSON: {err.msg}")
            continue
        problems = label_errors(row)
        errors += [f"{path}:{number}: {problem}" for problem in problems]
        if problems:
            continue
        key = row["attempt_key"]
        if key in seen:
            errors.append(f"{path}:{number}: attempt_key {key!r} repeats line {seen[key]}")
            continue
        seen[key] = number
        rows.append(row)
    if errors:
        raise LabelError("\n".join(errors))
    return rows


def _rule_errors(row: dict) -> list[str]:
    errors = []
    pi = row["pi"]
    if not 0 < pi <= 1:
        errors.append(f"$.pi: an inclusion probability is in (0, 1], not {pi}")
    if row["vs_source"] == "census" and pi != 1:
        errors.append(f"$.pi: a census row labels every attempt, so pi is 1.0, not {pi}")
    if row["vs_source"] == "audit" and row["verdict"] is None:
        errors.append("$.verdict: an audit labels a green unit, so its verdict cannot be null")
    if row["completion"] is True and row["final_commit"] is None:
        errors.append("$.final_commit: a completed attempt left a final commit, so it cannot be null")
    hidden = row["hidden"]
    if hidden is not None and not 0 <= hidden["failed"] <= hidden["n"]:
        errors.append(f"$.hidden: {hidden['failed']} failed of {hidden['n']} tests")
    g, findings = row["integrity"]["g"], row["integrity"]["findings"]
    if g == 1 and not findings:
        errors.append("$.integrity: g is 1, so it names at least one finding")
    if g != 1 and findings:
        errors.append(f"$.integrity: {len(findings)} finding(s) with g {g}; any finding makes g 1 (S05 §4.3 A1)")
    for key in COST_KEYS:
        amount = row["cost_usd"][key]
        if amount is not None and amount < 0:
            errors.append(f"$.cost_usd.{key}: a cost cannot be below 0, not {amount}")
    if row["label_rule"] not in LABEL_RULES:
        return errors + [f"$.label_rule: {row['label_rule']!r} is not a rule this package knows ({LABEL_RULE})"]
    checks = vs_checks(row)
    lenient = [value for name, value in checks.items() if name != "integrity"]
    expected = {"unknown": any(value is None for value in checks.values()),
                "vs": int(all(value is True for value in checks.values())),
                "vs_lenient": int(all(value is True for value in lenient))}
    for field, value in expected.items():
        if row[field] != value:  # the schema already fixed the types: unknown is a boolean, vs and vs_lenient 0 or 1
            errors.append(f"$.{field}: the checks give {json.dumps(value)}, not {json.dumps(row[field])} "
                          f"(label rule {LABEL_RULE})")
    return errors
