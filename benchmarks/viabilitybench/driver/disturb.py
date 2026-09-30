"""Disturbances for H6: which of S08 §4.6's hooks a `vb run` applies, and where in the stream (gap-15bb83).

`vb run --disturbance SPEC.toml` reads a `vb.disturbance/1` file, a list of S06 §4.9's `DisturbanceSpec`:

    schema_version = "vb.disturbance/1"

    [[disturbance]]
    kind = "provider_fault"                   # an S08 §4.6 hook, by S06's kind name
    start_at = 11                             # the first stream position it covers (from 1; default 1)
    end_at = 40                               # the last one (default: the stream's end)
    seed = 7                                  # default 0
    params = { name = "http_5xx", p = 0.2 }   # the kind's parameters

The tasks at the positions a disturbance covers get it, and each run record lists the kinds that covered its task
in `stream.perturbations_active`. The run manifest, and the config that `config_hash` hashes, carry the spec. Two
disturbances of one kind may not cover the same position. The hooks:

- `provider_fault`: the metering proxy injects the fault profile `params` names (`faultproxy.Profile`: `name` and
  that profile's parameters; the spec's seed is the profile's) into the calls of the tasks it covers, and every
  other task runs `clean`. It needs the run to go through the proxy: a billed network run always does, and any
  other needs `--proxy`. Ground truth: the proxy's log, and each attempt's `fault_injected`.
- `budget_cut`: the tasks it covers run under the arm's caps with `usd_per_task` and both input-token caps scaled by
  `params.factor` (default 0.5), in the runner and in the proxy's caps.
- `harder_mix`: from `start_at` on, the stream serves its instances at `params.levels` (default ℓ4–ℓ5) first, in
  their seeded order, then the rest. It permutes the seed's order, so each (task, seed) still runs once, and
  `order-<seed>.json` holds the disturbed order.
- `convention_flip`: the tasks it covers are rendered with `gen.py --latent <params.latent>` (default v2), which F1
  and F4 build (gap-98516b). `vb run` refuses a spec whose latent some family of the run cannot render. Ground
  truth: each record's `task.latent_version`.
- `flaky_verify` (S08's `VB_FLAKE_P`, gap-4e8795): each run of a covered task's visible check through the
  visible-verify wrapper (`vb_verify.py`) fails with probability `params.p` (default 0.25, S06 §4.9), drawn from the
  spec's seed. A run that names it gives every task the wrapper, with p = 0 outside the positions it covers, so the
  arms see the same thing throughout. Ground truth: each record's `visible.flake_injected` and `visible.flakes`.
- `model_swap` (gap-8bdf5e): for the tasks it covers, the metering proxy sends `params.to` upstream in place of the
  pinned model, so the provider serves that model and says so: S06's silent swap. The spec declares it, so the model
  checks (`records.final_status`, and `run_roko`'s) accept exactly that served model on those tasks and mark each
  attempt it served `model_swapped`; any other model is still a mismatch, and so is the swap outside its positions.
  It needs the proxy, as `provider_fault` does, so the Claude Code arm, whose CLI signs in by itself and cannot go
  through the proxy, cannot run it; and `params.to` needs a row in the run's price snapshot. Ground truth: the proxy
  log's `model_swap`, and each attempt's `model_reported` and `model_swapped`.

API:
    Disturbance(kind, start_at=1, end_at=None, seed=0, params={}); .covers(position) -> bool; .as_json() -> dict
    load(path) -> list[Disturbance]                                   # raises DisturbanceError
    kinds(disturbances, position) -> list[str]                        # a record's perturbations_active
    reorder(order, disturbances, level_of) -> list[str]
    scaled(limits: caps.Caps, disturbances, position) -> caps.Caps
    profile(disturbances, position) -> faultproxy.Profile
    latent(disturbances, position) -> str | None                      # None: the family's default
    flake(disturbances, position) -> tuple[float, int]                # flaky_verify's (p, seed); (0.0, 0) if none
    swap(disturbances, position) -> str | None                        # model_swap's model; None if none
"""

from __future__ import annotations

import dataclasses
import math
import tomllib
from collections.abc import Callable, Sequence
from dataclasses import dataclass, field
from pathlib import Path

import caps
import faultproxy

SCHEMA = "vb.disturbance/1"
KINDS = ("provider_fault", "model_swap", "harder_mix", "budget_cut", "convention_flip", "flaky_verify")
DEFAULTS = {"budget_cut": {"factor": 0.5}, "harder_mix": {"levels": [4, 5]}, "convention_flip": {"latent": "v2"},
            "flaky_verify": {"p": 0.25}, "model_swap": {"to": None}}  # model_swap's `to` has no default
LEVELS = range(1, 6)


class DisturbanceError(ValueError):
    """A disturbance spec cannot be read, or names a kind or a value that `vb run` cannot apply."""


@dataclass(frozen=True)
class Disturbance:
    kind: str
    start_at: int = 1
    end_at: int | None = None  # None: to the stream's end
    seed: int = 0
    params: dict = field(default_factory=dict)

    def covers(self, position: int) -> bool:
        return self.start_at <= position and (self.end_at is None or position <= self.end_at)

    def as_json(self) -> dict:
        return dataclasses.asdict(self)


def load(path: Path | str) -> list[Disturbance]:
    try:
        with Path(path).open("rb") as handle:
            doc = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as err:
        raise DisturbanceError(f"{path}: {err}") from None
    tables = doc.get("disturbance")
    if doc.get("schema_version") != SCHEMA or not isinstance(tables, list) or not tables:
        raise DisturbanceError(f"{path}: not a {SCHEMA} file with one or more [[disturbance]] tables")
    found = [_parse(table, f"{path}: disturbance {number}") for number, table in enumerate(tables, 1)]
    for index, one in enumerate(found):
        for other in found[index + 1:]:
            ends = [math.inf if end is None else end for end in (one.end_at, other.end_at)]
            if one.kind == other.kind and max(one.start_at, other.start_at) <= min(ends):
                raise DisturbanceError(f"{path}: two {one.kind} disturbances cover the same positions")
    return found


def kinds(disturbances: Sequence[Disturbance], position: int) -> list[str]:
    return sorted({one.kind for one in disturbances if one.covers(position)})


def reorder(order: list[str], disturbances: Sequence[Disturbance], level_of: Callable[[str], int]) -> list[str]:
    """The seed's order with each `harder_mix` applied (module docstring)."""
    for one in disturbances:
        if one.kind == "harder_mix":
            head, tail = order[:one.start_at - 1], order[one.start_at - 1:]
            levels = set(one.params["levels"])
            order = head + [i for i in tail if level_of(i) in levels] + [i for i in tail if level_of(i) not in levels]
    return order


def scaled(limits: caps.Caps, disturbances: Sequence[Disturbance], position: int) -> caps.Caps:
    """The caps of the task at `position`: the arm's, cut by the `budget_cut` that covers it."""
    cut = [one for one in disturbances if one.kind == "budget_cut" and one.covers(position)]
    if not cut:
        return limits
    factor = cut[0].params["factor"]
    return dataclasses.replace(limits, usd_per_task=limits.usd_per_task * factor,
                               input_tokens_per_task=max(int(limits.input_tokens_per_task * factor), 1),
                               input_tokens_per_attempt=max(int(limits.input_tokens_per_attempt * factor), 1))


def profile(disturbances: Sequence[Disturbance], position: int) -> faultproxy.Profile:
    """The proxy's fault profile for the task at `position`: the covering `provider_fault`'s, else clean."""
    fault = [one for one in disturbances if one.kind == "provider_fault" and one.covers(position)]
    return faultproxy.Profile.parse({**fault[0].params, "seed": fault[0].seed}) if fault else faultproxy.Profile()


def latent(disturbances: Sequence[Disturbance], position: int) -> str | None:
    flip = [one for one in disturbances if one.kind == "convention_flip" and one.covers(position)]
    return flip[0].params["latent"] if flip else None


def flake(disturbances: Sequence[Disturbance], position: int) -> tuple[float, int]:
    """The flake probability and seed of the task at `position`: the covering `flaky_verify`'s, else (0.0, 0)."""
    flaky = [one for one in disturbances if one.kind == "flaky_verify" and one.covers(position)]
    return (float(flaky[0].params["p"]), flaky[0].seed) if flaky else (0.0, 0)


def swap(disturbances: Sequence[Disturbance], position: int) -> str | None:
    """The model the proxy serves in place of the pin for the task at `position`: the covering `model_swap`'s."""
    swapped = [one for one in disturbances if one.kind == "model_swap" and one.covers(position)]
    return swapped[0].params["to"] if swapped else None


def _parse(table: object, where: str) -> Disturbance:
    if not isinstance(table, dict) or set(table) - {"kind", "start_at", "end_at", "seed", "params"}:
        raise DisturbanceError(f"{where}: a disturbance has only kind, start_at, end_at, seed and params")
    kind = table.get("kind")
    if kind not in KINDS:
        raise DisturbanceError(f"{where}: kind must be one of {', '.join(KINDS)}, not {kind!r}")
    start, end, seed = table.get("start_at", 1), table.get("end_at"), table.get("seed", 0)
    if not (_whole(start) and start >= 1 and (end is None or (_whole(end) and end >= start)) and _whole(seed)):
        raise DisturbanceError(f"{where}: start_at must be a position from 1, end_at none before it, and seed an "
                               "integer")
    if not isinstance(table.get("params", {}), dict):
        raise DisturbanceError(f"{where}: params must be a table")
    params = {**DEFAULTS.get(kind, {}), **table.get("params", {})}
    if kind == "provider_fault":
        try:
            faultproxy.Profile.parse({**params, "seed": seed})
        except faultproxy.ProxyError as err:
            raise DisturbanceError(f"{where}: {err}") from None
    elif set(params) != set(DEFAULTS[kind]):
        raise DisturbanceError(f"{where}: {kind} takes only {', '.join(DEFAULTS[kind])}")
    elif kind == "budget_cut" and not (isinstance(params["factor"], (int, float)) and not isinstance(
            params["factor"], bool) and 0 < params["factor"] <= 1):
        raise DisturbanceError(f"{where}: budget_cut's factor must be a number above 0 and at most 1")
    elif kind == "harder_mix" and not (isinstance(params["levels"], list) and params["levels"] and all(
            _whole(level) and level in LEVELS for level in params["levels"])):
        raise DisturbanceError(f"{where}: harder_mix's levels must be ladder levels from 1 to 5")
    elif kind == "convention_flip" and not (isinstance(params["latent"], str) and params["latent"]):
        raise DisturbanceError(f"{where}: convention_flip's latent must name a latent version, such as v2")
    elif kind == "flaky_verify" and not (isinstance(params["p"], (int, float)) and not isinstance(
            params["p"], bool) and 0 <= params["p"] <= 1):
        raise DisturbanceError(f"{where}: flaky_verify's p must be a probability from 0 to 1")
    elif kind == "model_swap" and not (isinstance(params["to"], str) and params["to"]):
        raise DisturbanceError(f"{where}: model_swap's to must name the model the proxy serves instead")
    return Disturbance(kind=kind, start_at=start, end_at=end, seed=seed, params=params)


def _whole(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)
