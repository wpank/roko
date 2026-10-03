"""The ambiguity probe A-v1 (ClarifyGPT-style, S07 §4.2, S07.5; task 3237): how much a cheap model's own idea of
a task's interface inputs disagrees with itself, sample to sample.

``probe(spec, model, k=3, max_inputs=5) -> AmbiguityResult`` draws `k` cheap samples, each `model(spec,
sample_index) -> (inputs, cost_usd)` naming at most `max_inputs` example inputs the model thinks the spec's
interface takes (a short completion, never a solution). `ambiguity` is the mean pairwise Jaccard distance
between the samples' input sets: 0 when every sample names the same inputs (the spec reads the same way every
time), 1 when no two samples share a single one. A single sample (`k <= 1`) has no pair to compare, so
`ambiguity` is 0 -- one draw cannot show disagreement.

Advisory only (S07 §8), like `specops.critic`: never changes the spec, or speclint's `score` or `band`. No
network call happens in this module; `model` is the caller's own adapter (a stub in tests; a live provider is
S09 block C, not yet wired).

API:
    AmbiguityModel = Callable[[dict, int], tuple[Iterable[str], float]]  # (spec, sample_index) -> (inputs, cost_usd)
    AmbiguityResult(ambiguity, samples, cost_usd); .as_json() -> dict
    probe(spec: dict, model: AmbiguityModel, k: int = 3, max_inputs: int = 5) -> AmbiguityResult
    jaccard_distance(a: set, b: set) -> float
"""

from __future__ import annotations

import itertools
from collections.abc import Callable, Iterable
from dataclasses import dataclass

AmbiguityModel = Callable[[dict, int], tuple[Iterable[str], float]]
K = 3
MAX_INPUTS = 5


@dataclass(frozen=True)
class AmbiguityResult:
    ambiguity: float
    samples: list[list[str]]
    cost_usd: float

    def as_json(self) -> dict:
        return {"ambiguity": self.ambiguity}


def jaccard_distance(a: set, b: set) -> float:
    """1 - |a ∩ b| / |a ∪ b|; 0 when both are empty (nothing to disagree about)."""
    union = a | b
    return 0.0 if not union else 1.0 - len(a & b) / len(union)


def probe(spec: dict, model: AmbiguityModel, k: int = K, max_inputs: int = MAX_INPUTS) -> AmbiguityResult:
    """Draw `k` samples of `model`'s idea of `spec`'s interface inputs; never modifies `spec`."""
    if k < 1:
        raise ValueError(f"k is at least 1, not {k}")
    samples: list[list[str]] = []
    cost_usd = 0.0
    for sample_index in range(k):
        inputs, sample_cost = model(spec, sample_index)
        samples.append(list(inputs)[:max_inputs])
        cost_usd += sample_cost
    pairs = list(itertools.combinations(range(k), 2))
    distances = [jaccard_distance(set(samples[i]), set(samples[j])) for i, j in pairs]
    ambiguity = sum(distances) / len(distances) if distances else 0.0
    return AmbiguityResult(ambiguity=ambiguity, samples=samples, cost_usd=cost_usd)
