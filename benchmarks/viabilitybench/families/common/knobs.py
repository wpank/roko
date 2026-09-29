"""Difficulty knobs: a family's `ladder.toml` maps each level ℓ1–ℓ5 to knob values (S08 §4.4).

The ladder file (`families/<family>/ladder.toml`):

    schema_version = "vb.ladder/1"
    family = "F1"

    [levels.1]
    k_doc = "documented"
    k_ex = [4, 6]        # an inclusive range: gen.py draws one value from the instance's surface stream
    k_files = 1
    k_size = 6
    k_vis = "main_path"
    k_hid = "low"
    # [levels.2] … [levels.5] likewise; every level must be present

Knob names start with `k_`. A value is a bool, an int, a string, or an inclusive `[low, high]` int range. The S08
knobs, when a family uses them, have fixed types: `k_doc`, `k_vis` and `k_hid` take the strings in `S08_ENUMS`, and
`k_ex`, `k_files`, `k_quirks` and `k_size` (the module count) take non-negative ints or ranges. A family may add
knobs of its own, and uses only the S08 knobs that apply to it (`k_quirks` is F2's and F4's).

`s08_deviations(ladder)` lists every value outside S08 §4.4's design band for its level. It reports rather than
refuses, because the calibration rule (§4.4) may move a level off its band after the pilot; CI decides.

Instance ids name (family, level, seed), as in S08 §5.2: `instance_id("F4", 3, 17) == "F4-l3-0017"`.

API:
    load_ladder(path: Path) -> Ladder                          # raises LadderError
    Ladder.family, Ladder.levels: dict[int, dict[str, KnobValue]]
    Ladder.declared(level: int) -> dict[str, KnobValue]        # as written, ranges included
    Ladder.draw(level: int, stream: Stream) -> dict[str, bool | int | str]   # ranges resolved, in knob-name order
    s08_deviations(ladder: Ladder) -> list[str]
    instance_id(family: str, level: int, seed: int) -> str
    parse_instance_id(value: str) -> tuple[str, int, int]      # (family, level, seed)
"""

from __future__ import annotations

import re
import tomllib
from dataclasses import dataclass
from pathlib import Path

from .hmac_seed import Stream

LEVELS = (1, 2, 3, 4, 5)
SCHEMA_VERSION = "vb.ladder/1"
FAMILY_RE = re.compile(r"F[1-8]")
KNOB_RE = re.compile(r"k_[a-z0-9_]+")
INSTANCE_RE = re.compile(r"(F[1-8])-l([1-5])-(\d{4,})")

KnobValue = bool | int | str | list[int]

S08_ENUMS = {
    "k_doc": ("documented", "undocumented", "legacy_distractor", "stale_or_contradictory"),
    "k_vis": ("main_path", "weak", "weak_misleading", "happy_path_only"),
    "k_hid": ("low", "medium", "high", "high_differential"),
}
S08_COUNTS = ("k_ex", "k_files", "k_quirks", "k_size")
# S08 §4.4's table, level by level. A string knob must match; a count's value or range must lie inside the band.
S08_BANDS: dict[str, dict[int, str | tuple[int, int]]] = {
    "k_doc": {1: "documented", 2: "documented", 3: "undocumented", 4: "legacy_distractor",
              5: "stale_or_contradictory"},
    "k_ex": {1: (4, 6), 2: (2, 3), 3: (1, 1), 4: (1, 1), 5: (0, 1)},
    "k_files": {1: (1, 1), 2: (1, 2), 3: (2, 3), 4: (4, 6), 5: (5, 8)},
    "k_quirks": {1: (1, 1), 2: (1, 1), 3: (2, 2), 4: (3, 3), 5: (3, 3)},
    "k_size": {1: (6, 6), 2: (10, 10), 3: (16, 16), 4: (30, 30), 5: (60, 60)},
    "k_vis": {1: "main_path", 2: "main_path", 3: "weak", 4: "weak_misleading", 5: "happy_path_only"},
    "k_hid": {1: "low", 2: "low", 3: "medium", 4: "high", 5: "high_differential"},
}


class LadderError(ValueError):
    """A ladder file is missing, malformed, or has a knob of the wrong type."""


@dataclass(frozen=True)
class Ladder:
    family: str
    levels: dict[int, dict[str, KnobValue]]
    path: Path

    def declared(self, level: int) -> dict[str, KnobValue]:
        """The knobs of `level` as the file writes them; a range stays a [low, high] list."""
        if level not in self.levels:
            raise LadderError(f"{self.path}: no level {level!r}; levels are {', '.join(map(str, LEVELS))}")
        return {name: list(value) if isinstance(value, list) else value
                for name, value in self.levels[level].items()}

    def draw(self, level: int, stream: Stream) -> dict[str, bool | int | str]:
        """The knobs of `level` with every range resolved by `stream`, drawn in knob-name order."""
        drawn: dict[str, bool | int | str] = {}
        for name, value in sorted(self.declared(level).items()):
            drawn[name] = stream.randint(value[0], value[1]) if isinstance(value, list) else value
        return drawn


def load_ladder(path: Path) -> Ladder:
    """Read and check a family's ladder file."""
    path = Path(path)
    try:
        with path.open("rb") as handle:
            doc = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as err:
        raise LadderError(f"{path}: {err}") from None
    if doc.get("schema_version") != SCHEMA_VERSION:
        raise LadderError(f"{path}: schema_version must be {SCHEMA_VERSION!r}")
    family = doc.get("family")
    if not isinstance(family, str) or not FAMILY_RE.fullmatch(family):
        raise LadderError(f"{path}: family must be one of F1–F8, not {family!r}")
    unknown = set(doc) - {"schema_version", "family", "levels"}
    if unknown:
        raise LadderError(f"{path}: unexpected top-level key(s): {', '.join(sorted(unknown))}")
    raw = doc.get("levels")
    if not isinstance(raw, dict) or sorted(raw) != [str(level) for level in LEVELS]:
        raise LadderError(f"{path}: [levels] must define exactly the levels 1–5")
    levels = {}
    for level in LEVELS:
        knobs = raw[str(level)]
        if not isinstance(knobs, dict) or not knobs:
            raise LadderError(f"{path}: [levels.{level}] must be a non-empty table")
        for name, value in knobs.items():
            error = _knob_error(name, value)
            if error:
                raise LadderError(f"{path}: levels.{level}.{name}: {error}")
        levels[level] = dict(knobs)
    return Ladder(family=family, levels=levels, path=path)


def s08_deviations(ladder: Ladder) -> list[str]:
    """Each S08 knob value outside its S08 §4.4 band, as "ℓ<level> <knob> = <value>: S08 has <band>"."""
    deviations = []
    for level in LEVELS:
        for name, value in sorted(ladder.levels[level].items()):
            band = S08_BANDS.get(name, {}).get(level)
            if band is None:
                continue
            if isinstance(band, str):
                inside = value == band
                shown = band
            else:
                low, high = value if isinstance(value, list) else (value, value)
                inside = band[0] <= low and high <= band[1]
                shown = str(band[0]) if band[0] == band[1] else f"{band[0]}–{band[1]}"
            if not inside:
                deviations.append(f"ℓ{level} {name} = {value!r}: S08 §4.4 has {shown}")
    return deviations


def instance_id(family: str, level: int, seed: int) -> str:
    """The id of one instance, e.g. "F4-l3-0017"; the seed is zero-padded to at least four digits."""
    if not isinstance(family, str) or not FAMILY_RE.fullmatch(family):
        raise ValueError(f"family must be one of F1–F8, not {family!r}")
    if level not in LEVELS or isinstance(level, bool):
        raise ValueError(f"level must be one of 1–5, not {level!r}")
    if isinstance(seed, bool) or not isinstance(seed, int) or seed < 0:
        raise ValueError(f"seed must be a non-negative int, not {seed!r}")
    return f"{family}-l{level}-{seed:04d}"


def parse_instance_id(value: str) -> tuple[str, int, int]:
    """(family, level, seed) from an instance id; the inverse of `instance_id`."""
    match = INSTANCE_RE.fullmatch(value)
    if not match or instance_id(match[1], int(match[2]), int(match[3])) != value:
        raise ValueError(f"not an instance id: {value!r}")
    return match[1], int(match[2]), int(match[3])


def _knob_error(name: str, value: object) -> str | None:
    if not KNOB_RE.fullmatch(name):
        return "knob names are k_ followed by lowercase letters, digits or underscores"
    if isinstance(value, list):
        if len(value) != 2 or not all(type(bound) is int for bound in value) or value[0] > value[1]:
            return f"a range is [low, high] with int bounds and low <= high, not {value!r}"
    elif not isinstance(value, (bool, int, str)):
        return f"a knob is a bool, an int, a string or a [low, high] range, not {type(value).__name__}"
    if name in S08_ENUMS and value not in S08_ENUMS[name]:
        return f"must be one of {', '.join(S08_ENUMS[name])}, not {value!r}"
    if name in S08_COUNTS:
        bounds = value if isinstance(value, list) else [value]
        if not all(type(bound) is int and bound >= 0 for bound in bounds):
            return f"must be a non-negative int or range, not {value!r}"
    return None
