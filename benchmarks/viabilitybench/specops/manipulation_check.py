"""The manipulation check (S07.3, §7.3; SC2; task 3234): a vague twin must read like a much weaker spec.

S07.3's acceptance, on H3's 48 instances (stream P1-H3: F1-F5 and F7, 8 each): every vague variant (D-v1's full
``VAGUE`` composition, L1 through L5) scores at least :data:`MIN_GAP` SQS points below its precise twin, scored
by the same linter in the same workspace. Without this, H3 would be comparing cheap and frontier models on a
"vague" spec that is not actually weaker -- H3 measures nothing (S07 SC2).

``check_pair`` degrades one precise spec and scores both halves; ``check_many`` runs it over a named collection
(the 48 H3 instances, or -- in :func:`test_manipulation_check.py` -- the four 3233 fixtures) and is false
(``Report.ok``) the moment one pair's gap is below the bar or its two halves were scored under different suites
or workspaces, which would make the gap meaningless to compare.

Building the 48 live H3 instances from the S08 family generators (3234's Plan item 1: F1-F5 and F7's own
``gen.py``/``render_spec``) needs a converter from each family's own manifest and markdown spec into this
module's TSS v1 ``[[task]]`` shape (S08.T8 and S08.T9 landed the families themselves, not a tasks.toml
projection of them). ``specops.from_generator`` (gap-c71dbb) is that converter for the one family whose
``generate()`` output it currently reads this way, F1; its own module docstring says which of H3's other five
families it does not cover yet, and why. ``check_many`` takes whatever precise specs its caller already has in
that shape -- the fixtures here, or such a converter's output.

API:
    MIN_GAP = 30
    PairReport(instance_id, precise_score, vague_score, linter, manifest, precise_suite_hash, vague_suite_hash,
              precise_env_hash, vague_env_hash); .gap; .hashes_match; .ok; .as_json() -> dict
    Report(pairs); .ok; .failures -> list[str]; .as_json() -> dict
    check_pair(instance_id, precise: dict, *, seed: int, workspace_root: Path, linter=speclint.LINTER) -> PairReport
    check_many(specs: Mapping[str, dict], *, seed: int, workspace_root: Path, linter=speclint.LINTER) -> Report
    suite_hash(linter: str) -> str
    env_hash(workspace_root: Path) -> str
"""

from __future__ import annotations

import hashlib
import json
import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

_SPECOPS_DIR = Path(__file__).resolve().parent
_VB_DIR = _SPECOPS_DIR.parent
for _path in (_VB_DIR, _VB_DIR / "speclint"):
    if str(_path) not in sys.path:
        sys.path.insert(0, str(_path))
import speclint  # noqa: E402
from specops import VAGUE, degrade  # noqa: E402

MIN_GAP = 30


def suite_hash(linter: str) -> str:
    """What scored the spec: the linter id, its rule weights and names. Two pairs scored under the same linter
    version always get the same hash; a rule-weight change (a new linter id) never silently keeps the old one."""
    payload = json.dumps({"linter": linter, "weights": speclint.WEIGHTS, "rules": speclint.RULE_NAMES},
                         sort_keys=True)
    return "sha256:" + hashlib.sha256(payload.encode("utf-8")).hexdigest()


def env_hash(workspace_root: Path) -> str:
    """Where it was scored: the interpreter version and the resolved workspace root (SQ07/HF4 read files there)."""
    payload = f"{sys.version}|{Path(workspace_root).resolve()}"
    return "sha256:" + hashlib.sha256(payload.encode("utf-8")).hexdigest()


@dataclass(frozen=True)
class PairReport:
    instance_id: str
    precise_score: float
    vague_score: float
    linter: str
    manifest: dict
    precise_suite_hash: str
    vague_suite_hash: str
    precise_env_hash: str
    vague_env_hash: str

    @property
    def gap(self) -> float:
        return round(self.precise_score - self.vague_score, 2)

    @property
    def hashes_match(self) -> bool:
        return (self.precise_suite_hash == self.vague_suite_hash
                and self.precise_env_hash == self.vague_env_hash)

    @property
    def ok(self) -> bool:
        return self.gap >= MIN_GAP and self.hashes_match

    def as_json(self) -> dict:
        return {"instance_id": self.instance_id, "precise_score": self.precise_score,
                "vague_score": self.vague_score, "gap": self.gap, "linter": self.linter, "ok": self.ok,
                "hashes_match": self.hashes_match, "manifest": self.manifest}


def check_pair(instance_id: str, precise: dict, *, seed: int, workspace_root: Path,
               linter: str = speclint.LINTER) -> PairReport:
    """Degrade `precise` with D-v1's full VAGUE composition at `seed`, score both under `linter`, and report the
    gap and each half's scoring suite and environment hash. Never modifies `precise`."""
    vague, manifest = degrade(precise, VAGUE, seed)
    workspace = speclint.Workspace(Path(workspace_root))
    precise_ctx = speclint.PlanContext(workspace=workspace, plan_id=instance_id, plan_path=f"{instance_id}.toml",
                                       archived=False, tasks_by_id={instance_id: precise}, plan_outputs={})
    vague_ctx = speclint.PlanContext(workspace=workspace, plan_id=instance_id, plan_path=f"{instance_id}.toml",
                                     archived=False, tasks_by_id={instance_id: vague}, plan_outputs={})
    precise_record = speclint.score_task(precise, precise_ctx, linter=linter)
    vague_record = speclint.score_task(vague, vague_ctx, linter=linter)
    env = env_hash(workspace_root)
    return PairReport(instance_id=instance_id, precise_score=precise_record["score"],
                      vague_score=vague_record["score"], linter=linter, manifest=manifest.record(),
                      precise_suite_hash=suite_hash(precise_record["linter"]),
                      vague_suite_hash=suite_hash(vague_record["linter"]), precise_env_hash=env, vague_env_hash=env)


@dataclass(frozen=True)
class Report:
    pairs: list[PairReport]

    @property
    def ok(self) -> bool:
        return bool(self.pairs) and not self.failures

    @property
    def failures(self) -> list[str]:
        return [f"{pair.instance_id}: gap {pair.gap} < {MIN_GAP}" if pair.gap < MIN_GAP else
                f"{pair.instance_id}: precise and vague were scored under different suites or workspaces"
                for pair in self.pairs if not pair.ok]

    def as_json(self) -> dict:
        return {"ev": "manipulation_check", "min_gap": MIN_GAP, "n": len(self.pairs), "ok": self.ok,
               "failures": self.failures, "pairs": [pair.as_json() for pair in self.pairs]}


def check_many(specs: Mapping[str, dict], *, seed: int, workspace_root: Path,
               linter: str = speclint.LINTER) -> Report:
    """`check_pair` over every (instance_id, precise spec) of `specs` (the 48 H3 instances, or a fixture set)."""
    return Report([check_pair(instance_id, spec, seed=seed, workspace_root=workspace_root, linter=linter)
                  for instance_id, spec in specs.items()])
