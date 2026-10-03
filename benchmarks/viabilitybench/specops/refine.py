"""The spec refiner R-v1 (S07 §4.3, §4.5; task 3235): ``refine(spec, model, ctx) -> RefineResult``.

S07's linter can answer "refine" for a weak spec (a C/D band): cheaper than escalating to a frontier model, a
cheap model gets at most two rounds to make the spec clearer, never to move its goalposts. Each round calls
``model(spec, round_number) -> Proposal``, a protocol any provider adapter can implement (tests use a stub; no
network call happens here). A proposal may only **add** to :data:`ADDITIVE_FIELDS` -- ``acceptance``, ``verify``,
``non_goals``, ``assumptions``, ``context.read_files`` and ``context.symbols`` -- each item with a non-empty
``source`` (a PRD line, a file or a symbol) so a reader can trace why the refiner added it.

A round is rejected, and refinement stops, when it:

- deletes, reorders or edits an existing ``verify`` step (the prefix must stay byte-for-byte; a round may only
  append new ones);
- changes ``files``;
- names a path under a hidden-suite convention (``.vb/...``, anything with a ``hidden`` path component, or
  ``task.json``) in a new ``context.read_files``/``context.symbols`` entry -- the refiner never hands the agent a
  path meant for the truth suite alone;
- carries an addition with no ``source``, or to a field :data:`ADDITIVE_FIELDS` does not list;
- would not raise the spec's own `speclint.score_task` score (monotonicity): a refiner that could lower the
  score could be gamed into laundering a worse spec through "refinement".

A proposal with no additions also stops the loop (the model has nothing left to add). ``red_on_base`` is the
caller's own dynamic-mode verdict (``speclint/dynamic.py``), passed through unchanged to every round's rescoring,
since no round here ever touches an existing verify step; a round's own new steps carry no red-on-base verdict of
their own until the caller dynamically checks them afresh. Every accepted round appends a :class:`RefineRound`,
the ``spec.refined`` record (S07 §4.3): the before/after spec hashes, what was added by field, its sources, the
model's name and its ``cost_usd``.

API:
    ADDITIVE_FIELDS = ("acceptance", "verify", "non_goals", "assumptions", "context.read_files", "context.symbols")
    Addition(field, item, source)
    Proposal(model, additions, cost_usd=0.0)
    Model = Callable[[dict, int], Proposal]
    RefineRound(round, before_hash, after_hash, before_score, after_score, added, model, cost_usd); .record() -> dict
    RefineResult(spec, rounds); .cost_usd; .refined -> bool
    refine(spec, model, ctx, *, max_rounds=2, red_on_base="unknown", linter=speclint.LINTER) -> RefineResult
    spec_hash(spec) -> str
"""

from __future__ import annotations

import copy
import hashlib
import json
import sys
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

_SPECLINT_DIR = Path(__file__).resolve().parents[1] / "speclint"
if str(_SPECLINT_DIR) not in sys.path:
    sys.path.insert(0, str(_SPECLINT_DIR))
import speclint  # noqa: E402

MAX_ROUNDS = 2
ADDITIVE_FIELDS = ("acceptance", "verify", "non_goals", "assumptions", "context.read_files", "context.symbols")
# Path components no addition may name: the private manifest directory, anything explicitly "hidden", and the
# manifest file itself (common/sandbox.py's `denied()` protects the same two things for a running agent).
_HIDDEN_COMPONENTS = (".vb", "hidden")
_HIDDEN_FILES = ("task.json",)


@dataclass(frozen=True)
class Addition:
    field: str  # one of ADDITIVE_FIELDS
    item: object  # a string for acceptance/non_goals/assumptions; a dict for verify steps and context entries
    source: str  # a PRD line, a file path or a symbol; never empty


@dataclass(frozen=True)
class Proposal:
    model: str
    additions: list[Addition] = field(default_factory=list)
    cost_usd: float = 0.0


Model = Callable[[dict, int], Proposal]


@dataclass(frozen=True)
class RefineRound:
    round: int
    before_hash: str
    after_hash: str
    before_score: float
    after_score: float
    added: dict[str, int]
    sources: list[str]
    model: str
    cost_usd: float

    def record(self) -> dict:
        return {
            "ev": "spec.refined", "round": self.round, "from_hash": self.before_hash, "to_hash": self.after_hash,
            "before_score": self.before_score, "after_score": self.after_score, "added": dict(self.added),
            "sources": list(self.sources), "model": self.model, "cost_usd": self.cost_usd,
        }


@dataclass(frozen=True)
class RefineResult:
    spec: dict
    rounds: list[RefineRound]
    stopped: str  # "no_proposal", "rounds_exhausted", or why a round was rejected

    @property
    def cost_usd(self) -> float:
        return sum(round_.cost_usd for round_ in self.rounds)

    @property
    def refined(self) -> bool:
        return bool(self.rounds)


def spec_hash(spec: dict) -> str:
    canonical = json.dumps(spec, sort_keys=True, ensure_ascii=False, separators=(",", ":"), default=str)
    return "sha256:" + hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def refine(spec: dict, model: Model, ctx: speclint.PlanContext, *, max_rounds: int = MAX_ROUNDS,
          red_on_base: str = "unknown", linter: str = speclint.LINTER) -> RefineResult:
    """Run at most `max_rounds` of `model` over a copy of `spec`; `spec` itself is never modified."""
    current = copy.deepcopy(spec)
    rounds: list[RefineRound] = []
    before_score = speclint.score_task(current, ctx, red_on_base=red_on_base, linter=linter)["score"]
    stopped = "rounds_exhausted"
    for round_number in range(1, max_rounds + 1):
        proposal = model(current, round_number)
        if not proposal.additions:
            stopped = "no_proposal"
            break
        candidate = copy.deepcopy(current)
        for addition in proposal.additions:
            _append(candidate, addition.field, copy.deepcopy(addition.item))
        problem = _rejection(current, candidate, proposal.additions)
        if problem is not None:
            stopped = problem
            break
        after_score = speclint.score_task(candidate, ctx, red_on_base=red_on_base, linter=linter)["score"]
        if after_score < before_score:
            stopped = f"round {round_number} would lower the score ({before_score} -> {after_score})"
            break
        added: dict[str, int] = {}
        for addition in proposal.additions:
            added[addition.field] = added.get(addition.field, 0) + 1
        rounds.append(RefineRound(
            round=round_number, before_hash=spec_hash(current), after_hash=spec_hash(candidate),
            before_score=before_score, after_score=after_score, added=added,
            sources=[addition.source for addition in proposal.additions], model=proposal.model,
            cost_usd=proposal.cost_usd))
        current, before_score = candidate, after_score
    return RefineResult(spec=current, rounds=rounds, stopped=stopped)


def _append(spec: dict, field_name: str, item: object) -> None:
    if "." in field_name:
        parent, key = field_name.split(".", 1)
        spec.setdefault(parent, {}).setdefault(key, []).append(item)
    else:
        spec.setdefault(field_name, []).append(item)


def _rejection(before: dict, after: dict, additions: list[Addition]) -> str | None:
    """Why this round is rejected (the module docstring's guards), or None when it holds."""
    if after.get("files") != before.get("files"):
        return "changed files"
    before_verify = before.get("verify") or []
    after_verify = after.get("verify") or []
    if after_verify[:len(before_verify)] != before_verify:
        return "deleted, reordered or edited an existing verify step"
    for addition in additions:
        if addition.field not in ADDITIVE_FIELDS:
            return f"{addition.field} is not an additive field"
        if not str(addition.source or "").strip():
            return f"{addition.field}: an addition with no source"
        if addition.field in ("context.read_files", "context.symbols") and _names_hidden_path(addition.item):
            return f"{addition.field}: names a hidden-suite path"
    return None


def _names_hidden_path(item: object) -> bool:
    path = str(item.get("path", "")) if isinstance(item, dict) else str(item)
    path = path.split("::")[0]
    parts = Path(path).parts
    return path in _HIDDEN_FILES or any(part in _HIDDEN_COMPONENTS for part in parts)
