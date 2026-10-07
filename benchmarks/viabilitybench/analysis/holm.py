"""Graphical Holm over S09's seven primaries, with H1's level chain inside it (S09 §4.1 "Multiplicity", §5; D28;
task 3338).

**One p-value per primary** (§4.1; each hypothesis's *Holm p* line in §4.4):
- a directional claim uses p = 2 × the one-sided p, capped at 1 (`directional`), so an unadjusted claim matches the
  95% CI and a claim at local level a matches the (1 − a) bound;
- a conjunction (every part must hold) is an intersection-union test: p is the largest part's (`iut`);
- a disjunction (any part suffices) is Bonferroni: k × the smallest of k parts, capped at 1 (`bonferroni`; S09's
  disjunctions have two parts, "2·min");
- a failed pass/fail gate (a validity or safety check) sets p = 1 (`gated`).

**The procedure** is Bretz et al.'s (2009) sequentially rejective graphical test with Bonferroni local tests, over
eleven elementary hypotheses: H1's levels H1:l1 … H1:l5, and H2 … H7 (`s09_graph`).
- Initial weights: 1/7 on H1:l1 and on each of H2 … H7; 0 on H1:l2 … H1:l5.
- Edges: H1:lj → H1:l(j+1) with weight 1, so a claimed level hands H1's whole weight to the next level, and H1's
  weight reaches the other primaries only after H1:l5 is claimed (H1:l5 → each of H2 … H7, 1/6). Every other primary
  sends 1/6 to each of the six others, H1 entering at H1:l1: a rejected primary's weight is split equally among the
  unrejected ones ("equal recycling").
- While some unrejected node i has p_i ≤ w_i·α, it is rejected and the graph updated (Bretz et al., Algorithm 1):
  w_l ← w_l + w_i·g_il and g_lk ← (g_lk + g_li·g_ik) / (1 − g_li·g_il), or 0 when g_li·g_il = 1. The result does
  not depend on the order of rejection. As levels are claimed, the edges into H1:l1 move on to the next unclaimed
  level, so weight that another primary releases later can carry H1's chain further.
- H1's node p-values are the chain's p̃_j = max(p_ℓ1 … p_ℓj) (`chain`), so a level is never claimed past an
  unclaimed one. H1 is supported when H1:l1 is rejected, and E* under Holm is the last level rejected.

**Adjusted p-values** (`adjust`) come from the same algorithm (Bretz et al. 2011): reject the node with the smallest
p_i / w_i, with the running maximum of min(1, p_i / w_i) as its adjusted p, and repeat; a node is rejected at α
exactly when its adjusted p is at most α. Weights are exact fractions, so a p at a boundary never flips on rounding.

`MULTIPLICITY` is S09 §5's `multiplicity` block, the settings the lock records (task 3341); `test_envelope.py` checks
the graph against it.

**Verdict records** (`verdict_records`, `write_verdicts`; gap-2da8ec) persist a `decide()` result as one
`vb.verdict/1` record per primary, in a `vb.verdicts/1` document next to `report.py`'s `metrics.json`, so that
`showcase/build_bundle.py` has something to read instead of its `NOT_YET_MEASURED` default. `claim_state` picks
S09's claim state (`SUPPORTED`, `NOT_SUPPORTED` or `INCONCLUSIVE`; `NOT_YET_MEASURED` is a hypothesis with no
verdict record at all, never this module's to say) for one primary. A directional p-value (above) is already
one-sided, so a Holm rejection only ever happens in the hypothesized direction; failing to reject is silent on
which way the true effect lies, and nothing here can tell "the effect runs the other way" (`NOT_SUPPORTED`) from
"not enough signal either way" (`INCONCLUSIVE`) without a side fact Holm's own p-value cannot supply. `adverse`
is that fact, supplied per primary by whoever ran the analysis and read the point estimate's sign; the
conservative default with no `adverse` entry is `INCONCLUSIVE` (an ambiguous rule, picked conservatively,
gap-2da8ec).

API:
    MULTIPLICITY; PRIMARIES; H1_LEVELS; CLAIM_STATES; VERDICT_SCHEMA; VERDICTS_SCHEMA; VERDICTS_FILE
    directional(one_sided) -> float; iut(*parts) -> float; bonferroni(*parts) -> float; gated(p, passed) -> float
    chain(level_p) -> list[float]
    Graph(weights, edges); s09_graph() -> Graph
    node_p(h1_level_p, others) -> dict[node, float]
    adjust(p, graph=None) -> dict[node, float]
    HolmResult(adjusted, rejected, alpha, e_star, supported); decide(p, alpha=0.05, graph=None) -> HolmResult
    claim_state(primary, result, adverse=False) -> str
    verdict_records(result, experiment_id, metrics=None, run_ids=None, adverse=None) -> list[dict]
    write_verdicts(out_dir, experiment_id, result, metrics=None, run_ids=None, adverse=None, created_at=None) -> Path
"""

from __future__ import annotations

import datetime as dt
import json
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path

PRIMARIES = ("H1", "H2", "H3", "H4", "H5", "H6", "H7")
H1_LEVELS = tuple(f"H1:l{level}" for level in range(1, 6))
# S09 §5's `multiplicity` block, as the lock records it.
MULTIPLICITY = {"method": "graphical_holm", "weights": "1/7 each", "recycle": "equal", "directional_p": "2*one_sided",
                "conjunction": "iut_max", "disjunction": "bonferroni_2min", "failed_gate_p": 1,
                "h1_chain": ["l1", "l2", "l3", "l4", "l5"], "h1_release_after": "l5"}
# contracts.ts's ClaimState, minus NOT_YET_MEASURED: a hypothesis with no verdict record at all is that one, never
# one this module writes (gap-2da8ec's module docstring).
CLAIM_STATES = ("SUPPORTED", "NOT_SUPPORTED", "INCONCLUSIVE")
VERDICT_SCHEMA = "vb.verdict/1"  # one hypothesis's decision
VERDICTS_SCHEMA = "vb.verdicts/1"  # the document of all of them, next to report.py's vb.metrics/1
VERDICTS_FILE = "verdicts.json"


def directional(one_sided: float) -> float:
    """S09 §4.1: a directional claim's p is twice the one-sided p, capped at 1."""
    return min(1.0, 2.0 * _p(one_sided))


def iut(*parts: float) -> float:
    """A conjunction's p: the intersection-union test's maximum of its parts (`berger1982multiparameter`)."""
    if not parts:
        raise ValueError("a conjunction needs at least one part")
    return max(map(_p, parts))


def bonferroni(*parts: float) -> float:
    """A disjunction's p: k × the smallest of its k parts, capped at 1."""
    if not parts:
        raise ValueError("a disjunction needs at least one part")
    return min(1.0, len(parts) * min(map(_p, parts)))


def gated(p: float, passed: bool) -> float:
    """p when the gate passed; 1 when it failed (S09 §4.1)."""
    return _p(p) if passed else 1.0


def chain(level_p: Sequence[float]) -> list[float]:
    """The fixed sequence's p̃_j = max(p_1 … p_j) for H1's levels in order."""
    out, peak = [], 0.0
    for p in map(_p, level_p):
        peak = max(peak, p)
        out.append(peak)
    return out


@dataclass(frozen=True)
class Graph:
    weights: dict[str, Fraction]  # node -> initial weight; they sum to 1
    edges: dict[str, dict[str, Fraction]]  # node -> {target: share of its weight}; each row sums to at most 1


def s09_graph() -> Graph:
    """S09 §4.1's graph: equal weights and equal recycling over the primaries, H1 as a five-level chain."""
    others = PRIMARIES[1:]
    weights = {node: Fraction(0) for node in H1_LEVELS} | {primary: Fraction(1, 7) for primary in others}
    weights[H1_LEVELS[0]] = Fraction(1, 7)
    edges: dict[str, dict[str, Fraction]] = {node: {} for node in weights}
    for current, following in zip(H1_LEVELS, H1_LEVELS[1:]):
        edges[current][following] = Fraction(1)
    edges[H1_LEVELS[-1]] = {primary: Fraction(1, 6) for primary in others}
    for primary in others:
        edges[primary] = {target: Fraction(1, 6) for target in (H1_LEVELS[0], *others) if target != primary}
    return Graph(weights=weights, edges=edges)


def node_p(h1_level_p: Sequence[float], others: Mapping[str, float]) -> dict[str, float]:
    """The graph's p-values: H1's five levels through `chain`, and H2 … H7 as given (each already one p, §4.4)."""
    if len(h1_level_p) != len(H1_LEVELS):
        raise ValueError(f"H1 has {len(H1_LEVELS)} levels, not {len(h1_level_p)} p-values")
    missing = sorted(set(PRIMARIES[1:]) - set(others))
    if missing or set(others) - set(PRIMARIES[1:]):
        raise ValueError(f"the other primaries are {', '.join(PRIMARIES[1:])}; missing {', '.join(missing) or '-'}")
    return dict(zip(H1_LEVELS, chain(h1_level_p))) | {name: _p(others[name]) for name in PRIMARIES[1:]}


def adjust(p: Mapping[str, float], graph: Graph | None = None) -> dict[str, float]:
    """Each node's adjusted p-value under the graph (module docstring)."""
    graph = graph or s09_graph()
    if set(p) != set(graph.weights):
        raise ValueError(f"p-values for {sorted(p)}, but the graph's nodes are {sorted(graph.weights)}")
    weights = dict(graph.weights)
    edges = {node: dict(targets) for node, targets in graph.edges.items()}
    adjusted: dict[str, float] = {}
    running = Fraction(0)
    while weights:
        live = [node for node in sorted(weights) if weights[node] > 0]
        if not live:
            adjusted.update(dict.fromkeys(weights, 1.0))  # no weight is left to test the rest with
            break
        node = min(live, key=lambda name: (Fraction(p[name]) / weights[name], name))
        running = max(running, min(Fraction(1), Fraction(p[node]) / weights[node]))
        adjusted[node] = float(running)
        _reject(node, weights, edges)
    return {node: adjusted[node] for node in graph.weights}


@dataclass(frozen=True)
class HolmResult:
    adjusted: dict[str, float]
    rejected: tuple[str, ...]  # the nodes rejected at alpha, in the graph's order
    alpha: float
    e_star: int  # the last H1 level rejected under Holm (0 when H1:l1 is not)
    supported: dict[str, bool]  # primary -> rejected at alpha (H1: its first level)


def decide(p: Mapping[str, float], alpha: float = 0.05, graph: Graph | None = None) -> HolmResult:
    """The graphical Holm decisions at family-wise level `alpha`."""
    if not 0 < alpha < 1:
        raise ValueError(f"alpha must lie strictly between 0 and 1, not {alpha}")
    adjusted = adjust(p, graph)
    rejected = tuple(node for node in adjusted if adjusted[node] <= alpha)
    e_star = 0
    for node in H1_LEVELS:
        if node not in rejected:
            break
        e_star += 1
    supported = {primary: (H1_LEVELS[0] if primary == "H1" else primary) in rejected for primary in PRIMARIES}
    return HolmResult(adjusted=adjusted, rejected=rejected, alpha=alpha, e_star=e_star, supported=supported)


def claim_state(primary: str, result: HolmResult, adverse: bool = False) -> str:
    """S09's claim state for `primary` under `result` (module docstring; gap-2da8ec): ``SUPPORTED`` when Holm
    rejects `primary`'s null (its first level, for H1), else ``INCONCLUSIVE``, or ``NOT_SUPPORTED`` when `adverse`
    says the point estimate ran against the hypothesis rather than merely falling short of significance."""
    if primary not in result.supported:
        raise ValueError(f"{primary!r} is not one of {', '.join(PRIMARIES)}")
    if result.supported[primary]:
        return "SUPPORTED"
    return "NOT_SUPPORTED" if adverse else "INCONCLUSIVE"


def verdict_records(result: HolmResult, experiment_id: str, *, metrics: Mapping[str, Sequence[str]] | None = None,
                    run_ids: Mapping[str, Sequence[str]] | None = None,
                    adverse: Mapping[str, bool] | None = None) -> list[dict]:
    """One `vb.verdict/1` record per primary (module docstring; gap-2da8ec): `result`'s own decision and adjusted
    p, `claim_state`'s reading of it, and whichever of `metrics` (the S08 metric names) and `run_ids` name what it
    rests on for that primary -- both default to empty per primary, since a decision can exist before its inputs
    are catalogued this way."""
    metrics, run_ids, adverse = metrics or {}, run_ids or {}, adverse or {}
    records = []
    for primary in PRIMARIES:
        node = H1_LEVELS[0] if primary == "H1" else primary
        records.append({
            "schema_version": VERDICT_SCHEMA,
            "experiment_id": experiment_id,
            "hypothesis": primary,
            "alpha": result.alpha,
            "p_adjusted": result.adjusted[node],
            "rejected": result.supported[primary],
            "claim_state": claim_state(primary, result, bool(adverse.get(primary, False))),
            "test": "graphical_holm",
            "metrics": list(metrics.get(primary, ())),
            "run_ids": list(run_ids.get(primary, ())),
        })
    return records


def write_verdicts(out_dir: Path, experiment_id: str, result: HolmResult, *,
                   metrics: Mapping[str, Sequence[str]] | None = None,
                   run_ids: Mapping[str, Sequence[str]] | None = None,
                   adverse: Mapping[str, bool] | None = None, created_at: str | None = None) -> Path:
    """`verdict_records` written as `out_dir / VERDICTS_FILE` (module docstring; gap-2da8ec), the results
    directory's own per-experiment document, beside `report.py`'s `metrics.json`. Overwrites a file already
    there, like `report.py`'s own writer."""
    document = {
        "schema_version": VERDICTS_SCHEMA,
        "experiment_id": experiment_id,
        "created_at": created_at or dt.datetime.now(dt.UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "records": verdict_records(result, experiment_id, metrics=metrics, run_ids=run_ids, adverse=adverse),
    }
    path = Path(out_dir) / VERDICTS_FILE
    path.write_text(json.dumps(document, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return path


def _reject(node: str, weights: dict[str, Fraction], edges: dict[str, dict[str, Fraction]]) -> None:
    """Remove `node` and pass its weight along the graph (Bretz et al. 2009, Algorithm 1)."""
    released, outgoing = weights.pop(node), edges.pop(node)
    for other in weights:
        weights[other] += released * outgoing.get(other, Fraction(0))
    updated = {}
    for source in weights:
        into = edges[source].get(node, Fraction(0))
        back = outgoing.get(source, Fraction(0))
        row = {}
        for target in weights:
            if target == source:
                continue
            loop = into * back
            share = (edges[source].get(target, Fraction(0)) + into * outgoing.get(target, Fraction(0)))
            row[target] = share / (1 - loop) if loop < 1 else Fraction(0)
        updated[source] = {target: share for target, share in row.items() if share}
    edges.clear()
    edges.update(updated)


def _p(value: float) -> float:
    if isinstance(value, bool) or not 0.0 <= value <= 1.0:
        raise ValueError(f"a p-value lies in [0, 1], not {value!r}")
    return float(value)
