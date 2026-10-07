"""`vb campaign`: validate an experiment manifest, estimate it, and run its blocks in order (S09 E1a, E5, E8; 3306).

    vb campaign --manifest experiments/pilot_a.toml --dry-run          # validate and estimate; nothing runs
    vb campaign --manifest experiments/pilot_a.toml --allow-network    # run it: one `vb run` per unit
    vb campaign --manifest experiments/pilot_a.toml --allow-network --max-cost-usd 8   # under a hard cap in all
    vb campaign --manifest M --provider-url http://127.0.0.1:P/v1 --results R --work W   # an offline rehearsal

**The manifest** (`vb.experiment/1`, `schema/experiment.schema.json`) is a TOML file in `experiments/`: the
experiment id every run gets (`vb run --experiment`), an optional `prereg_id`, `requires_lock` and `requires_live`
(3359 enforces `requires_live`; until then a manifest that sets it is refused before it runs), an `order`, and its
blocks. A block is one cell of the experiment: a stream, an arm file's name in `arms/`, a model, seeds (`vb run`'s
`--seeds` form), a budget line and, optionally, a disturbance spec, an instance subset of the stream, the secret
file's fingerprint its instances are audited under, `max_cost_usd` (the most the block may spend, which a network
block needs), its planned billed spend (S09 §3's figures, required for a billed block), `optional` (run only with
`--include`) and `off_hours` (a note: the subscription is shared).

**The order.** `as_listed` runs each block as one unit, in the manifest's order. `daily_interleave` (S09 §4.1, "arms
are interleaved in randomized daily blocks") makes one unit per (block, seed), puts a block's k-th seed on day k, and
orders each day's units by a keyed shuffle of the manifest id, the order's seed and the day. Each unit is one
`vb run`, whose run id is `<unit>-<attempt>`, and whose `--max-cost-usd` is its share of the block's `max_cost_usd`
(its seeds over the block's).

**Validation** (`--dry-run`, and before anything runs) refuses, before any call or directory:
- an arm, a stream, a disturbance spec or a line that does not exist; a line held back (`[reserved]`) or outside
  the experiment cap's lines; an instance outside its stream; a model outside its arm's `models_allow` or without a
  row in the price snapshot;
- a network block without `max_cost_usd`, or one of whose units could not afford one task (`vb.admit`), and a
  billed block without `planned_usd`;
- a block whose worst case would take its line, its experiment cap or the programme stop past the cap, counting what
  the ledger already holds (`ledger.read_books` over the results root) and the worst cases of the blocks before it
  that have not run. A billed block's worst case is the most its runs can bill: each task's worst case under the
  arm's caps, but never more than `max_cost_usd`, since `vb run` stops a run before any task that could take it past
  its `--max-cost-usd`. A subscription block's is $0. A block whose planned spend alone would pass a cap is refused
  as well, with that reason. Of a block that has partly run, only the units still to run count, since the ledger
  holds what the others spent;
- a secret file whose fingerprint differs from the one a block names, and an instance that a unit of any campaign
  in the results root already ran under another secret's fingerprint (S09 §4.1: each instance is audited under one
  secret file for the whole campaign). Without a readable secret file the dry run says the secret was not checked.
The dry run prints the experiment, its units in order and, per block, its runs, planned spend and worst case, and
per line and cap what is held, planned and left, as JSON. It exits 2 when anything is refused.

**The pre-registration lock** (S09 SC1, 3341). An experiment runs only under the lock when its manifest says
`requires_lock`, when it is LOG1, or when it is a live experiment (S09 §5: an id starting `E-`); `requires_lock`
says which, from those rules and the manifests in `experiments/`. Such a campaign, and each `vb run` of such an
experiment, is refused unless the lock (`--lock`, default `experiments/prereg.lock.json`) exists, is committed and
checks clean against S09 (`--prereg-spec`) and the tree it pins (`analysis/lock.py`'s `require`).

**Running** (without `--dry-run`). Each unit runs `vb.py run` as a process of its own, with the operator's
environment, so its own checks and its restart into an allowlisted environment work as when typed by hand. A network
unit needs `--allow-network` on the campaign, and gets `--allow-network` and its `--max-cost-usd`. The campaign's own
`--max-cost-usd` is a hard cap on the experiment's billed spend: before each billed unit it counts what the
experiment's ledger rows and open reservations hold under the results root, and the unit's `--max-cost-usd` becomes
the smaller of its share and what is left, so no sequence of units can bill past the cap. A unit left less than one
task's worst case does not start, and the campaign stops there (exit 2); the dry run says what the cap leaves. An
offline rehearsal passes `--provider-url` (a loopback URL) to every unit, and `--proxy` to each unit the real run would
meter through the proxy (a billed arm whose provider names a key, or a disturbance the proxy applies). It may swap an
arm's file for a rehearsal one of the same arm id (`--arm-file NAME=PATH`) and run each unit on the first N instances
of its stream (`--limit N`); both are refused without a loopback `--provider-url`. The campaign appends a `start` and
a `finish` event per unit to `$VB_RESULTS/<experiment>/campaign.jsonl`, with the run id, the secret's fingerprint and
the instances. A block's instance subset becomes a stream file under `<experiment>/.campaign/`, which `vb report`
skips.

**Resuming.** A rerun skips every unit whose run exited 0 and starts at the first other one. An earlier attempt of
that unit that exited otherwise, or never finished, is retried under a new run id when its run directory holds no
records and no ledger rows. Otherwise the campaign stops: the attempt's records would count twice. Move its run
directory to `<results>/<experiment>.abandoned/` (inside the results root, so the ledger still counts its spend) and
run the campaign again.

**`requires_live`** (3359, S09 §5's example: `L-audit`, `L-gate-depth`, `L-route-trust`). Before a live manifest's
first dispatch, `check` runs the loop census (`roko learn loops --json`, S03.T15) against this repo's roko binary
and reads each required loop's status. A loop is LIVE when its census row's audit state is `live` (S03 §4.6's
`AuditState::Live`: "proven benefit: the learned policy runs, at the live holdout rate") — probation, flagged and
demoted loops, and a loop missing from the census, are not; `census` (default `census_report`, which shells out
to the `roko` binary) is injectable so a test can fake the report without one. Any loop not LIVE refuses the
campaign (`check`'s `problems`, so `--dry-run` exits 2 too) and, for a real run, `cmd_campaign` writes a NOT RUN
stub naming the loops and the harness sha (S09 §5: "a hypothesis whose loops are not LIVE … is postponed and
reported NOT RUN") before it raises.

API:
    load(path) -> Manifest                                   # raises CampaignError
    units(manifest, include=()) -> list[Unit]
    check(vb, manifest, *, budget, results_root, include=(), provider_url=None, arm_files=None,
          secret_fingerprint=None, lock=DEFAULT_LOCK, spec=DEFAULT_SPEC, census=None) -> Check
    requires_lock(experiment_id, manifests=EXPERIMENTS_DIR) -> bool
    lock_refusal(lock=DEFAULT_LOCK, spec=DEFAULT_SPEC) -> str | None      # why the lock does not let a run start
    census_report(repo=None, roko_bin=None) -> dict                      # roko.loop_census/1, via `roko learn loops`
    is_loop_live(row: dict) -> bool
    requires_live_refusal(manifest, *, repo=None, census=None) -> tuple[str | None, str | None, tuple[str, ...]]
    write_not_run(results_root, manifest, loops, sha) -> Path            # (why, harness_sha, not_live loops)
    cmd_campaign(vb, args) -> int; add_arguments(parser)
    CampaignError, SCHEMA, ORDERS, LOOPS_SCHEMA, NOT_RUN_SCHEMA, NOT_RUN_FILE
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib
import json
import math
import os
import re
import subprocess
import sys
import tomllib
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from pathlib import Path

import disturb
import layout
import ledger
import provider
import secret
import validate  # schema/validate.py, on sys.path through layout

SCHEMA = "vb.experiment/1"
ORDERS = ("as_listed", "daily_interleave")
BLOCK_ID = re.compile(r"[a-z0-9][a-z0-9-]{0,39}")
LOG = "campaign.jsonl"  # in <results>/<experiment>/
STREAMS = ".campaign"  # <results>/<experiment>/.campaign/: derived stream files, which `vb report` skips
ABANDONED = ".abandoned"  # <results>/<experiment>.abandoned/: where a stopped unit's run directory goes
EXPERIMENTS_DIR = layout.VB_ROOT / "experiments"
DEFAULT_LOCK = EXPERIMENTS_DIR / "prereg.lock.json"  # S09 E4
DEFAULT_SPEC = layout.REPO_ROOT / "tmp" / "cybernetic-harness" / "specs" / "S09-experiments.md"  # untracked, MAIN only
LOCKED_EXPERIMENTS = ("LOG1",)  # S09 SC1: LOG1 never starts without the lock
LIVE_PREFIX = "E-"  # S09 §5: live experiments are E-<name>(-live), and each runs under the lock
NEEDS_PROXY = ("provider_fault", "model_swap")  # disturbances the metering proxy applies (disturb.py)
LOOPS_SCHEMA = "roko.loop_census/1"  # crates/roko-learn/src/loop_audit/census.rs CENSUS_SCHEMA (`roko learn loops`)
NOT_RUN_SCHEMA = "vb.not_run/1"  # this driver's own stub (3359): vb.metric_record/1 needs run_ids, seeds, commits,
                                 # price_snapshot_id, ... which do not exist for a hypothesis that never ran
NOT_RUN_FILE = "not_run.json"  # in <results>/<experiment>/, next to LOG; written by write_not_run (3359)


class CampaignError(RuntimeError):
    """The manifest, or the campaign's state, refuses the campaign; `vb` exits 2."""


@dataclass(frozen=True)
class Block:
    id: str
    stream: str
    arm: str
    model: str
    seeds: tuple[int, ...]
    seeds_text: str
    line: str
    planned_usd: float | None = None
    max_cost_usd: float | None = None
    disturbance: str | None = None
    instances: tuple[str, ...] | None = None
    secret_fingerprint: str | None = None
    optional: bool = False
    off_hours: bool = False
    note: str = ""


@dataclass(frozen=True)
class Manifest:
    path: Path
    id: str
    order: str
    order_seed: int
    blocks: tuple[Block, ...]
    requires_lock: bool = False
    requires_live: tuple[str, ...] = ()
    prereg_id: str | None = None
    title: str = ""


@dataclass(frozen=True)
class Unit:
    """One `vb run` of a campaign: a block, or under `daily_interleave` one seed of it."""

    key: str
    block: Block
    seeds: tuple[int, ...]
    day: int

    @property
    def seeds_text(self) -> str:
        return self.block.seeds_text if self.seeds == self.block.seeds else ",".join(map(str, self.seeds))


@dataclass
class Check:
    """What validation found: the plan of each block, the units in order, the budget per scope, and the refusals."""

    manifest: Manifest
    units: list[Unit]
    blocks: dict[str, dict] = field(default_factory=dict)  # block id -> its plan summary
    scopes: dict[str, dict] = field(default_factory=dict)  # "line BL0" etc. -> cap, held, planned, left
    instances: dict[str, tuple[str, ...]] = field(default_factory=dict)  # block id -> the instances it runs
    problems: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    not_run: tuple[str, ...] = ()  # requires_live loops that are not LIVE (3359); cmd_campaign writes the stub
    harness_sha: str | None = None  # the loop census's harness sha, once requires_live has been checked

    def summary(self) -> dict:
        return {"experiment": self.manifest.id, "manifest": str(self.manifest.path), "order": self.manifest.order,
                "requires_lock": self.manifest.requires_lock, "requires_live": list(self.manifest.requires_live),
                "not_run": list(self.not_run), "harness_sha": self.harness_sha,
                "prereg_id": self.manifest.prereg_id,
                "runs": sum(self.blocks[unit.block.id].get("runs_per_seed", 0) * len(unit.seeds)
                            for unit in self.units),
                "planned_usd": round(sum(block.get("planned_usd") or 0.0 for block in self.blocks.values()
                                         if block.get("included")), 6),
                "units": [{"unit": unit.key, "day": unit.day, "block": unit.block.id, "seeds": unit.seeds_text}
                          for unit in self.units],
                "blocks": list(self.blocks.values()), "budget": self.scopes, "notes": self.notes,
                "problems": self.problems, "ok": not self.problems}


def load(path: Path) -> Manifest:
    """A `vb.experiment/1` manifest, checked against its schema and the rules the schema cannot state."""
    path = Path(path)
    try:
        with path.open("rb") as handle:
            doc = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as err:
        raise CampaignError(f"{path}: {err}") from None
    errors = validate.validate("experiment", doc)
    if errors:
        raise CampaignError(f"{path}: {errors[0]}")
    order = doc["order"]
    if order["kind"] == "daily_interleave" and "seed" not in order:
        raise CampaignError(f"{path}: order daily_interleave needs a seed")
    blocks = []
    for number, table in enumerate(doc["block"], 1):
        where = f"{path}: block {number}"
        if not BLOCK_ID.fullmatch(table["id"]):
            raise CampaignError(f"{where}: id {table['id']!r} must match {BLOCK_ID.pattern}")
        for name in ("planned_usd", "max_cost_usd"):
            value = table.get(name)
            if value is not None and not (value >= 0 if name == "planned_usd" else value > 0):
                raise CampaignError(f"{where}: {name} must be {'at least 0' if name == 'planned_usd' else 'above 0'}")
        instances = table.get("instances")
        if instances is not None and len(set(instances)) != len(instances):
            raise CampaignError(f"{where}: instances repeat")
        blocks.append(Block(id=table["id"], stream=table["stream"], arm=table["arm"], model=table["model"],
                            seeds=tuple(_seeds(table["seeds"], where)), seeds_text=table["seeds"],
                            line=table["line"], planned_usd=table.get("planned_usd"),
                            max_cost_usd=table.get("max_cost_usd"), disturbance=table.get("disturbance"),
                            instances=tuple(instances) if instances is not None else None,
                            secret_fingerprint=table.get("secret_fingerprint"),
                            optional=table.get("optional", False), off_hours=table.get("off_hours", False),
                            note=table.get("note", "")))
    ids = [block.id for block in blocks]
    if len(set(ids)) != len(ids):
        raise CampaignError(f"{path}: block ids repeat")
    if any(not name for name in doc["requires_live"]):
        raise CampaignError(f"{path}: requires_live names loops, so no entry may be empty")
    return Manifest(path=path, id=doc["id"], order=order["kind"], order_seed=order.get("seed", 0),
                    blocks=tuple(blocks), requires_lock=doc["requires_lock"],
                    requires_live=tuple(doc["requires_live"]), prereg_id=doc.get("prereg_id") or None,
                    title=doc.get("title", ""))


def units(manifest: Manifest, include: Iterable[str] = ()) -> list[Unit]:
    """The campaign's units in run order (module docstring), optional blocks only when `include` names them."""
    chosen = [block for block in manifest.blocks if not block.optional or block.id in set(include)]
    if manifest.order == "as_listed":
        return [Unit(key=block.id, block=block, seeds=block.seeds, day=1) for block in chosen]
    days: dict[int, list[Unit]] = {}
    for block in chosen:
        for day, seed in enumerate(block.seeds, 1):
            days.setdefault(day, []).append(Unit(key=f"{block.id}-s{seed}", block=block, seeds=(seed,), day=day))
    return [unit for day in sorted(days) for unit in sorted(days[day], key=lambda unit: _shuffle_key(
        f"{manifest.id}/{manifest.order_seed}/{day}/{unit.key}"))]


def requires_lock(experiment_id: str, manifests: Path = EXPERIMENTS_DIR) -> bool:
    """Whether `experiment_id` runs only under the pre-registration lock (module docstring)."""
    if experiment_id in LOCKED_EXPERIMENTS or experiment_id.startswith(LIVE_PREFIX):
        return True
    for path in sorted(Path(manifests).glob("*.toml")):
        try:
            manifest = load(path)
        except CampaignError:
            continue  # budget.toml and the like are not manifests
        if manifest.id == experiment_id and manifest.requires_lock:
            return True
    return False


def lock_refusal(lock: Path = DEFAULT_LOCK, spec: Path = DEFAULT_SPEC) -> str | None:
    """Why the lock at `lock` does not let a locked experiment start (missing, uncommitted or drifted), or None."""
    analysis = str(layout.VB_ROOT / "analysis")
    if analysis not in sys.path:
        sys.path.append(analysis)  # after the driver's own modules: analysis/lock.py and what it imports
    prereg = importlib.import_module("lock")
    try:
        prereg.require(lock, spec)
    except prereg.LockError as err:
        return str(err)
    return None


def census_report(repo: Path | None = None, roko_bin: Path | str | None = None) -> dict:
    """The `roko.loop_census/1` report `roko learn loops --json` prints for the binary at `roko_bin` (default:
    `repo`'s own `target/debug/roko`, the build a live manifest runs under; S03.T15), run from `repo` (default
    layout.REPO_ROOT). Raises CampaignError if the binary is missing or the command does not print that schema."""
    repo = Path(repo) if repo else layout.REPO_ROOT
    binary = Path(roko_bin) if roko_bin else repo / "target" / "debug" / "roko"
    if not os.access(binary, os.X_OK):
        raise CampaignError(f"no executable roko binary at {binary} to run the loop census (requires_live); build "
                            "it, or pass a fake census to check()")
    try:
        result = subprocess.run([str(binary), "learn", "loops", "--json"], cwd=repo, capture_output=True,
                                text=True, timeout=60)
    except OSError as err:
        raise CampaignError(f"{binary} learn loops --json: {err}") from None
    if result.returncode != 0:
        raise CampaignError(f"{binary} learn loops --json exited {result.returncode}: {result.stderr.strip()}")
    try:
        report = json.loads(result.stdout)
    except ValueError as err:
        raise CampaignError(f"{binary} learn loops --json did not print JSON: {err}") from None
    if not isinstance(report, dict) or report.get("schema") != LOOPS_SCHEMA:
        got = report.get("schema") if isinstance(report, dict) else report
        raise CampaignError(f"{binary} learn loops --json printed schema {got!r}, not {LOOPS_SCHEMA!r}")
    return report


def is_loop_live(row: dict) -> bool:
    """Whether a `roko.loop_census/1` row is LIVE: its audit state (crates/roko-learn/src/loop_audit/spec.rs,
    `AuditState::Live`) is "proven benefit: the learned policy runs, at the live holdout rate". Probation, flagged
    and demoted loops are not LIVE, and neither is a loop missing from the census."""
    return bool(row) and row.get("state") == "live"


def _not_live_reason(loop_id: str, row: dict) -> str:
    if not row:
        return f"{loop_id} (not in the census)"
    state = row.get("state") or "no audit state yet"
    reason = row.get("reason")
    return f"{loop_id} ({state}{f', ' + reason if reason else ''})"


def requires_live_refusal(manifest: Manifest, *, repo: Path | None = None,
                          census: Callable[[], dict] | None = None) -> tuple[str | None, str | None, tuple[str, ...]]:
    """Why `manifest.requires_live` refuses the campaign, the harness sha the census was read at, and which
    required loops are not LIVE; all empty/None when the manifest needs no live loop. S09 §5: a live manifest's
    required loops must be LIVE at the run's harness sha before the first dispatch, or the hypothesis is postponed
    and reported NOT RUN (`write_not_run`). `census` (default: `census_report` at `repo`) is injectable so a test
    can fake the roko.loop_census/1 report without a binary."""
    if not manifest.requires_live:
        return None, None, ()
    report = (census or (lambda: census_report(repo)))()
    sha = report.get("harness_sha")
    rows = {row.get("loop"): row for row in report.get("rows", []) if isinstance(row, dict)}
    not_live = tuple(loop_id for loop_id in manifest.requires_live if not is_loop_live(rows.get(loop_id, {})))
    if not not_live:
        return None, sha, ()
    named = "; ".join(_not_live_reason(loop_id, rows.get(loop_id, {})) for loop_id in not_live)
    return f"requires_live: not LIVE at harness {sha or 'unknown'}: {named}", sha, not_live


def write_not_run(results_root: Path, manifest: Manifest, loops: Iterable[str], sha: str | None) -> Path:
    """Write the NOT RUN stub for a live manifest `requires_live` refused (S09 §5: "a hypothesis whose loops are
    not LIVE … is postponed and reported NOT RUN"), naming the loops and the harness sha. This is not a
    `vb.metric_record/1`: most of that schema's required fields (run_ids, seeds, commits, price_snapshot_id, …)
    describe a run that happened, which this one did not, so forcing placeholders into them would mislead whoever
    reads it; `NOT_RUN_SCHEMA` is a separate, clearly-labeled shape instead. Returns the path written,
    `<results_root>/<manifest.id>/not_run.json`."""
    path = results_root / manifest.id / NOT_RUN_FILE
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    stub = {"schema": NOT_RUN_SCHEMA, "status": "not_run", "experiment_id": manifest.id,
           "manifest": str(manifest.path), "prereg_id": manifest.prereg_id, "harness_sha": sha,
           "loops_not_live": list(loops), "reported_at": dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")}
    text = json.dumps(stub, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    try:
        os.write(fd, text.encode("utf-8"))
    finally:
        os.close(fd)
    return path


def check(vb, manifest: Manifest, *, budget: ledger.Budget, results_root: Path, include: Iterable[str] = (),
          provider_url: str | None = None, arm_files: dict[str, str] | None = None,
          secret_fingerprint: str | None = None, finished: Iterable[str] = (), limit: int | None = None,
          lock: Path = DEFAULT_LOCK, spec: Path = DEFAULT_SPEC, census: Callable[[], dict] | None = None) -> Check:
    """Validate `manifest` against the arms, streams, snapshot, budget and ledger (module docstring). `vb` is the
    driver module; `finished` names the units already done, whose spend the ledger holds. `census` overrides
    `requires_live_refusal`'s default loop-census call (module docstring, requires_live)."""
    include = set(include)
    unknown = include - {block.id for block in manifest.blocks if block.optional}
    plan_units = units(manifest, include)
    found = Check(manifest=manifest, units=plan_units)
    if unknown:
        found.problems.append(f"--include names no optional block: {', '.join(sorted(unknown))}")
    if provider_url and not provider.is_loopback(provider_url):
        found.problems.append(f"--provider-url {provider_url} is not a loopback URL; a campaign overrides the "
                              "provider only for an offline rehearsal")
    if arm_files and not provider_url:
        found.problems.append("--arm-file swaps an arm's file for a rehearsal, so it needs a loopback --provider-url")
    if limit is not None and (not provider_url or limit < 1):
        found.problems.append("--limit cuts a rehearsal's streams, so it needs a loopback --provider-url and N >= 1")
    if manifest.requires_lock or requires_lock(manifest.id):
        refusal = lock_refusal(lock, spec)
        if refusal:
            found.problems.append(f"requires_lock: {refusal}")
        else:
            found.notes.append(f"requires_lock: the lock at {lock} is committed and checks clean")
    if manifest.requires_live:
        refusal, sha, not_live = requires_live_refusal(manifest, census=census)
        found.harness_sha = sha
        if refusal:
            found.problems.append(refusal)
            found.not_run = not_live
        else:
            found.notes.append(f"requires_live ({', '.join(manifest.requires_live)}): LIVE at harness "
                               f"{sha or 'unknown'}")
    try:
        books = ledger.read_books(results_root)
    except ledger.BudgetError as err:
        found.problems.append(f"the ledger under {results_root} cannot be read: {err}")
        books = ledger.Books([], [])
    pending: dict[str, list[float]] = {}  # scope -> [planned, worst case] of the earlier blocks' units still to run
    for block in manifest.blocks:
        included = not block.optional or block.id in include
        entry = {"block": block.id, "included": included, "stream": block.stream, "arm": block.arm,
                 "model": block.model, "seeds": block.seeds_text, "line": block.line, "optional": block.optional,
                 "off_hours": block.off_hours, "planned_usd": block.planned_usd, "max_cost_usd": block.max_cost_usd}
        found.blocks[block.id] = entry
        mine = [unit for unit in plan_units if unit.block.id == block.id]
        left = sum(len(unit.seeds) for unit in mine if unit.key not in set(finished)) / len(block.seeds)
        share = min((len(unit.seeds) for unit in mine), default=len(block.seeds)) / len(block.seeds)
        try:
            _check_block(vb, found, block, entry, budget, books, pending, provider_url, arm_files or {},
                         secret_fingerprint, left=left, share=share, limit=limit)
        except _Unavailable as err:
            entry["unavailable"] = str(err)
            (found.notes if block.optional and not included else found.problems).append(f"block {block.id}: {err}")
    _check_secrets(found, results_root, secret_fingerprint)
    return found


def cmd_campaign(vb, args: argparse.Namespace) -> int:
    """`vb campaign` (module docstring). `vb` is the driver module, which this one does not import itself."""
    manifest = load(_manifest_path(args.manifest))
    if args.max_cost_usd is not None and not args.max_cost_usd > 0:
        raise CampaignError("--max-cost-usd must be above $0")
    results_root = vb._outside_repo(args.results or os.environ.get("VB_RESULTS") or vb.DEFAULT_RESULTS, "--results")
    arm_files = dict(_pair(item) for item in args.arm_file or [])
    budget = ledger.load_budget()
    secret_file = Path(args.secret_file or os.environ.get(secret.FILE_ENV) or vb.DEFAULT_SECRET_FILE).expanduser()
    fingerprint, why = _fingerprint(secret_file)
    log = results_root / manifest.id / LOG
    events = _read_log(log)
    finished = {event["unit"] for event in events if event["event"] == "finish" and event.get("exit") == 0}
    found = check(vb, manifest, budget=budget, results_root=results_root, include=args.include or (),
                  provider_url=args.provider_url, arm_files=arm_files, secret_fingerprint=fingerprint,
                  finished=finished, limit=args.limit, lock=args.lock, spec=args.prereg_spec)
    if fingerprint is None:
        found.notes.append(f"secret: not checked ({why})")
    if args.max_cost_usd is not None and not found.problems:
        held = _held_usd(results_root, manifest.id)
        worst = sum(entry.get("worst_case_usd") or 0.0 for entry in found.blocks.values() if entry["included"])
        found.notes.append(f"--max-cost-usd ${args.max_cost_usd:.2f}: {manifest.id}'s books hold ${held:.4f} and "
                           f"its units still to run could bill ${worst:.2f}, so it bills at most "
                           f"${min(worst, max(args.max_cost_usd - held, 0.0)):.2f} more")
    if args.dry_run:
        print(json.dumps(found.summary(), indent=2, ensure_ascii=False))
        return 2 if found.problems else 0
    if found.problems:
        if found.not_run:
            write_not_run(results_root, manifest, found.not_run, found.harness_sha)
        raise CampaignError("refused before any run: " + "; ".join(found.problems))
    if fingerprint is None:
        raise CampaignError(f"the secret file cannot be read ({why}), so the campaign cannot keep each instance "
                            "under one secret")
    network = [unit.block.id for unit in found.units if found.blocks[unit.block.id].get("network")]
    if network and not args.allow_network:
        raise CampaignError(f"block(s) {', '.join(dict.fromkeys(network))} call a network provider: inspect "
                            "`vb campaign --dry-run` and pass --allow-network explicitly")
    ran = 0
    for unit in found.units:
        if unit.key in finished:
            continue
        if args.units is not None and ran >= args.units:
            print(f"vb campaign: stopped after {ran} unit(s) (--units); run again to go on", file=sys.stderr)
            return 0
        cap = _unit_cap(args, manifest, found, unit, results_root)
        attempt = _next_attempt(log, events, results_root / manifest.id, unit)
        code = _run_unit(vb, args, manifest, found, unit, attempt, results_root, arm_files, fingerprint, log, cap)
        events = _read_log(log)
        ran += 1
        if code != 0:
            print(f"vb campaign: unit {unit.key} exited {code}; its run is {results_root / manifest.id / attempt}. "
                  "Fix the cause and run the campaign again (module docstring, Resuming)", file=sys.stderr)
            return 1
    print(f"vb campaign: {manifest.id}: every unit finished", file=sys.stderr)
    return 0


def add_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--manifest", required=True, help="an experiment manifest (vb.experiment/1), or its name in "
                                                          "experiments/")
    parser.add_argument("--dry-run", action="store_true", help="validate and estimate only; print JSON")
    parser.add_argument("--allow-network", action="store_true", help="allow units that call a network provider")
    parser.add_argument("--include", action="append", help="also run this optional block (repeatable)")
    parser.add_argument("--units", type=int, help="run at most this many units now (a rerun goes on)")
    parser.add_argument("--provider-url", help="an offline rehearsal: every unit's provider is this loopback URL")
    parser.add_argument("--arm-file", action="append", metavar="NAME=PATH",
                        help="a rehearsal's file for the arm NAME, of the same arm id (needs a loopback "
                             "--provider-url)")
    parser.add_argument("--limit", type=int, help="a rehearsal runs each unit on its stream's first N instances (needs "
                                                  "a loopback --provider-url)")
    parser.add_argument("--secret-file", type=Path, help="passed to every unit's vb run")
    parser.add_argument("--key-file", type=Path, help="passed to every unit's vb run")
    parser.add_argument("--results", type=Path, help="passed to every unit's vb run")
    parser.add_argument("--work", type=Path, help="passed to every unit's vb run")
    parser.add_argument("--transcripts", action="store_true", help="passed to every unit's vb run")
    parser.add_argument("--lock", type=Path, default=DEFAULT_LOCK, help="the pre-registration lock a locked "
                        "experiment needs (default: experiments/prereg.lock.json); passed to every unit's vb run")
    parser.add_argument("--prereg-spec", type=Path, default=DEFAULT_SPEC, help="S09, which the lock pins (default: "
                        "the untracked spec in tmp/); passed to every unit's vb run")
    parser.add_argument("--max-cost-usd", type=float, help="a hard cap on the experiment's billed spend: each billed "
                                                           "unit gets its share, or what is left under this cap")


class _Unavailable(Exception):
    """A block refers to something that does not exist, so nothing else of it can be checked."""


def _check_block(vb, found: Check, block: Block, entry: dict, budget: ledger.Budget, books: ledger.Books,
                 pending: dict[str, list[float]], provider_url: str | None, arm_files: dict[str, str],
                 fingerprint: str | None, *, left: float, share: float, limit: int | None) -> None:
    """One block's checks (module docstring). `left` is the share of its seeds still to run, `share` its smallest
    unit's share of them, and `limit` a rehearsal's cut of each unit's stream."""
    where = f"block {block.id}"
    try:
        arm = vb.load_arm(arm_files.get(block.arm, block.arm))
        stream = vb.load_stream(block.stream)
        if block.arm in arm_files and vb.load_arm(block.arm)["arm"]["id"] != arm["arm"]["id"]:
            found.problems.append(f"{where}: --arm-file {arm['path']} is arm {arm['arm']['id']}, not the arm of "
                                  f"{block.arm}")
    except vb.DriverError as err:
        raise _Unavailable(str(err)) from None
    instances = block.instances or tuple(stream.instances)
    outside = [instance for instance in instances if instance not in stream.instances]
    if outside:
        found.problems.append(f"{where}: {', '.join(outside)} not in stream {stream.id}")
    instances = instances[:limit] if limit else instances
    found.instances[block.id] = instances
    runs_per_seed = len(instances)
    entry.update(runs_per_seed=runs_per_seed, runs=runs_per_seed * len(block.seeds), billed=arm["arm"]["billed"])
    args = argparse.Namespace(arm=arm["path"], stream=block.stream, model=block.model, seeds=block.seeds_text,
                              limit=0, price_snapshot=ledger.DEFAULT_SNAPSHOT, provider_url=provider_url)
    try:
        plan = vb.make_plan(args)
    except (vb.DriverError, ledger.PriceError) as err:
        found.problems.append(f"{where}: {err}")
        return
    row = plan.snapshot.row(block.model)
    if row is None:
        found.problems.append(f"{where}: {block.model} has no row in {plan.snapshot.id}, so its cost is unknown")
    network = not plan.endpoint.offline
    worst = plan.worst_task_usd
    bound = None if worst is None else round(worst * entry["runs"], 6)  # every task at its worst
    if bound is not None and block.max_cost_usd is not None:
        bound = min(bound, block.max_cost_usd)  # where `vb run` stops the block's runs
    keyed = bool(arm.get("providers", {}).get(plan.endpoint.provider, {}).get("api_key_env"))
    entry.update(network=network, provider=plan.endpoint.provider, worst_task_usd=worst,
                 worst_case_usd=bound if arm["arm"]["billed"] else 0.0,
                 planned_usd=block.planned_usd if arm["arm"]["billed"] else 0.0,
                 proxied=arm["arm"]["billed"] and keyed)  # what `vb run` meters through its proxy on the network
    if network:
        if block.max_cost_usd is None:
            found.problems.append(f"{where}: a network run needs max_cost_usd, its --max-cost-usd")
        elif row is not None:
            try:
                vb.admit(plan, allow_network=True, max_cost_usd=round(block.max_cost_usd * share, 6))
            except vb.DriverError as err:
                found.problems.append(f"{where}: a unit's share of max_cost_usd is too small: {err}")
    if arm["arm"]["billed"] and block.planned_usd is None:
        found.problems.append(f"{where}: a billed block needs planned_usd, its planned billed spend (S09 §3)")
    if block.disturbance:
        try:
            kinds = {one.kind for one in disturb.load(_config(block.disturbance))}
        except (disturb.DisturbanceError, OSError) as err:
            found.problems.append(f"{where}: {err}")
        else:
            if kinds & set(NEEDS_PROXY) and not plan.endpoint.api_key_env and not plan.endpoint.offline:
                found.problems.append(f"{where}: {', '.join(sorted(kinds & set(NEEDS_PROXY)))} needs the metering "
                                      f"proxy, which arm {block.arm} cannot go through")
            entry["disturbances"] = sorted(kinds)
    if block.secret_fingerprint and fingerprint and block.secret_fingerprint != fingerprint:
        found.problems.append(f"{where}: runs under secret {block.secret_fingerprint}, but the secret file is "
                              f"{fingerprint}")
    _check_budget(found, block, entry, budget, books, pending, left=left)


def _check_budget(found: Check, block: Block, entry: dict, budget: ledger.Budget, books: ledger.Books,
                  pending: dict[str, list[float]], *, left: float) -> None:
    """The budget rules of the module docstring, for one block in run order: of its planned spend and its worst case,
    only the share of its seeds still to run counts, since the ledger holds what the others spent."""
    where = f"block {block.id}"
    line = budget.lines.get(block.line)
    if line is None:
        held = budget.reserved.get(block.line)
        found.problems.append(f"{where}: budget line {block.line} " + (f"is held back ({held}) and funds nothing"
                                                                       if held else f"is not in {budget.path}"))
        return
    group = budget.experiment(found.manifest.id)
    if group and block.line not in group.lines:
        found.problems.append(f"{where}: experiment {found.manifest.id} may book only to {', '.join(group.lines)} "
                              f"(experiment cap {group.id}), not to {block.line}")
        return
    scopes: list[tuple[str, float, Callable[[dict], bool]]] = [
        (f"line {line.id}", line.cap_usd, lambda item: item["line"] == line.id)]
    if group:
        scopes.append((f"experiment cap {group.id}", group.cap_usd,
                       lambda item: item["experiment_id"] in group.experiment_ids))
    scopes.append(("the programme stop", budget.stop_usd, lambda item: True))
    planned = round((entry["planned_usd"] or 0.0) * left, 6)
    worst = round((entry["worst_case_usd"] or 0.0) * left, 6)
    if not entry["included"]:
        return
    for name, cap, member in scopes:
        spent, reserved = books.sums(member)
        held = round(float(spent + reserved), 6)
        planned_before, worst_before = pending.get(name, [0.0, 0.0])
        scope = found.scopes.setdefault(name, {"cap_usd": cap, "held_usd": held, "planned_usd": 0.0,
                                               "worst_case_usd": 0.0, "left_usd": round(cap - held, 6)})
        if not left:  # every unit ran: the ledger holds its spend
            continue
        if planned and round(held + planned_before + planned, 6) > cap:  # a block that adds nothing passes no cap
            found.problems.append(f"{where}: {name}: ${held:.4f} held + ${planned_before:.4f} planned before it "
                                  f"+ ${planned:.4f} planned for it would pass ${cap:.2f}")
        elif worst and round(held + worst_before + worst, 6) > cap:
            found.problems.append(f"{where}: {name}: at its worst, ${held:.4f} held + ${worst_before:.4f} for the "
                                  f"blocks before it + ${worst:.4f} for it would pass ${cap:.2f}")
        pending[name] = [round(planned_before + planned, 6), round(worst_before + worst, 6)]
        scope["planned_usd"], scope["worst_case_usd"] = pending[name]
        scope["left_usd"] = round(cap - held - pending[name][0], 6)


def _check_secrets(found: Check, results_root: Path, fingerprint: str | None) -> None:
    """S09 §4.1: an instance that a unit of any campaign in the results root ran under another secret is refused."""
    if fingerprint is None:
        return
    seen: dict[str, tuple[str, str]] = {}  # instance -> (fingerprint, where)
    for log in sorted(results_root.glob(f"*/{LOG}")):
        for event in _read_log(log):
            if event["event"] == "start" and event.get("secret_fingerprint"):
                for instance in event.get("instances") or []:
                    seen.setdefault(instance, (event["secret_fingerprint"], f"{log.parent.name}/{event['unit']}"))
    for block_id, instances in found.instances.items():
        clash = [(instance, *seen[instance]) for instance in instances
                 if instance in seen and seen[instance][0] != fingerprint]
        if clash and found.blocks[block_id]["included"]:
            instance, other, where = clash[0]
            found.problems.append(f"block {block_id}: {instance} ran under secret {other} ({where}), not the secret "
                                  f"file's {fingerprint}; each instance keeps one secret for the whole campaign "
                                  f"({len(clash)} instance(s))")


def _held_usd(results_root: Path, experiment_id: str) -> float:
    """What the experiment's ledger rows and open reservations under the results root hold (`Books.sums`)."""
    spent, reserved = ledger.read_books(results_root).sums(lambda item: item.get("experiment_id") == experiment_id)
    return spent + reserved


def _unit_cap(args: argparse.Namespace, manifest: Manifest, found: Check, unit: Unit,
              results_root: Path) -> float | None:
    """The unit's `--max-cost-usd`: its share of its block's `max_cost_usd`, held under what is left of the
    campaign's `--max-cost-usd` for a billed block (module docstring, Running). Raises CampaignError when what is
    left cannot afford one task."""
    block = unit.block
    if block.max_cost_usd is None:
        return None
    share = block.max_cost_usd * len(unit.seeds) / len(block.seeds)
    plan = found.blocks[block.id]
    if args.max_cost_usd is None or not plan.get("billed"):
        return share
    held = _held_usd(results_root, manifest.id)
    left = math.floor((args.max_cost_usd - held) * 1e6) / 1e6  # rounded down, so the cap holds to the micro-dollar
    worst = plan.get("worst_task_usd") or 0.0
    if left < worst:
        raise CampaignError(f"unit {unit.key}: ${max(left, 0.0):.4f} is left under --max-cost-usd "
                            f"${args.max_cost_usd:.2f} ({manifest.id}'s books hold ${held:.4f}), less than one "
                            f"task's worst case ${worst:.4f}, so it does not start")
    return min(share, left)


def _run_unit(vb, args: argparse.Namespace, manifest: Manifest, found: Check, unit: Unit, run_id: str,
              results_root: Path, arm_files: dict[str, str], fingerprint: str, log: Path,
              max_cost_usd: float | None) -> int:
    """One unit as `vb.py run` in a process of its own, under `max_cost_usd` (`_unit_cap`); its start and finish go
    to the campaign log."""
    block = unit.block
    plan = found.blocks[block.id]
    stream = block.stream
    if block.instances is not None:
        stream = str(_derived_stream(vb, results_root / manifest.id / STREAMS, block))
    argv = [sys.executable, str(layout.DRIVER_DIR / "vb.py"), "run", "--experiment", manifest.id, "--run-id",
            run_id, "--stream", stream, "--arm", arm_files.get(block.arm, block.arm), "--model", block.model,
            "--seeds", unit.seeds_text, "--line", block.line]
    if plan["network"]:
        argv.append("--allow-network")
    if max_cost_usd is not None:  # its share of the block's ceiling, or what the campaign's cap leaves
        argv += ["--max-cost-usd", f"{max_cost_usd:.6f}".rstrip("0").rstrip(".")]  # `:g` could round a cap up
    if args.provider_url:
        argv += ["--provider-url", args.provider_url]
    if args.limit:
        argv += ["--limit", str(args.limit)]
    if block.disturbance:
        argv += ["--disturbance", str(_config(block.disturbance))]
    if not plan["network"] and (plan["proxied"] or set(plan.get("disturbances", ())) & set(NEEDS_PROXY)):
        argv.append("--proxy")  # a rehearsal meets the metering proxy wherever the real run would
    for flag, value in (("--secret-file", args.secret_file), ("--key-file", args.key_file),
                        ("--results", results_root), ("--work", args.work), ("--lock", args.lock),
                        ("--prereg-spec", args.prereg_spec)):
        if value is not None:
            argv += [flag, str(value)]
    if args.transcripts:
        argv.append("--transcripts")
    if block.off_hours:
        print(f"vb campaign: unit {unit.key} runs on the shared subscription: off-hours only", file=sys.stderr)
    _append(log, {"event": "start", "unit": unit.key, "block": block.id, "day": unit.day, "seeds": list(unit.seeds),
                  "run_id": run_id, "secret_fingerprint": fingerprint,
                  "instances": list(found.instances.get(block.id, ()))})
    code = subprocess.run(argv, check=False).returncode
    _append(log, {"event": "finish", "unit": unit.key, "run_id": run_id, "exit": code})
    return code


def _next_attempt(log: Path, events: list[dict], experiment_dir: Path, unit: Unit) -> str:
    """The run id of the unit's next attempt; refuses when an earlier attempt left records or ledger rows."""
    attempts = [event["run_id"] for event in events if event["event"] == "start" and event["unit"] == unit.key]
    for run_id in attempts:
        run_dir = experiment_dir / run_id
        if any(_lines(run_dir / name) for name in ("records.jsonl", "ledger.jsonl")):
            raise CampaignError(f"unit {unit.key}: its attempt {run_id} did not finish cleanly and left records or "
                                f"ledger rows in {run_dir}. Move that directory to "
                                f"{experiment_dir.parent / (experiment_dir.name + ABANDONED)}/ and run the campaign "
                                f"again (module docstring, Resuming); {log} keeps its history")
    return f"{unit.key}-{len(attempts) + 1}"


def _derived_stream(vb, directory: Path, block: Block) -> Path:
    """A stream file holding the block's instance subset of its stream, under the parent's id."""
    stream = vb.load_stream(block.stream)
    doc = tomllib.loads(stream.path.read_text(encoding="utf-8"))
    table = doc["stream"]
    lines = ['schema_version = "vb.stream/1"', "", "[stream]", f"id = {json.dumps(table['id'])}",
             f"spec_variant = {json.dumps(stream.spec_variant)}",
             "families = { " + ", ".join(f"{name} = {json.dumps(path)}" for name, path in
                                         table.get("families", {}).items()) + " }",
             f"instances = {json.dumps(list(block.instances))}"]
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    path = directory / f"{block.id}.toml"
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return path


def _fingerprint(path: Path) -> tuple[str | None, str]:
    """The secret file's fingerprint, or None and why it could not be read. The secret itself is not kept."""
    try:
        return secret.load(path.absolute()).fingerprint, ""
    except (secret.SecretError, OSError) as err:
        return None, str(err)


def _read_log(path: Path) -> list[dict]:
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return []
    except OSError as err:
        raise CampaignError(f"{path}: {err}") from None
    events = []
    for number, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except ValueError:
            raise CampaignError(f"{path}:{number}: not JSON") from None
        if not isinstance(event, dict) or event.get("event") not in ("start", "finish") or "unit" not in event:
            raise CampaignError(f"{path}:{number}: not a campaign event")
        events.append(event)
    return events


def _append(path: Path, event: dict) -> None:
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    line = json.dumps({"ts": dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%SZ"), **event}, sort_keys=True)
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        os.write(fd, (line + "\n").encode("utf-8"))
    finally:
        os.close(fd)


def _lines(path: Path) -> int:
    try:
        return sum(1 for line in path.read_text(encoding="utf-8").splitlines() if line.strip())
    except OSError:
        return 0


def _seeds(text: str, where: str) -> list[int]:
    """`vb run`'s --seeds form ("1-3", "1,3"), parsed as `vb.parse_seeds` does."""
    seeds: list[int] = []
    for part in text.split(","):
        low, _, high = part.strip().partition("-")
        if not low.isdigit() or (high and not high.isdigit()):
            raise CampaignError(f"{where}: bad seeds {text!r}: use forms like 1-3 or 1,3")
        seeds += range(int(low), int(high or low) + 1)
    if not seeds or len(set(seeds)) != len(seeds):
        raise CampaignError(f"{where}: bad seeds {text!r}: empty or repeated")
    return seeds


def _shuffle_key(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def _manifest_path(name: str) -> Path:
    path = Path(name) if name.endswith(".toml") or "/" in name else layout.VB_ROOT / "experiments" / f"{name}.toml"
    if not path.is_file():
        raise CampaignError(f"no such manifest: {path}")
    return path.resolve()


def _config(name: str) -> Path:
    """A path in a manifest: absolute, or relative to the benchmark's root."""
    path = Path(name)
    return path if path.is_absolute() else layout.VB_ROOT / path


def _pair(text: str) -> tuple[str, str]:
    name, sep, path = text.partition("=")
    if not sep or not name or not path:
        raise CampaignError(f"--arm-file takes ID=PATH, not {text!r}")
    return name, str(Path(path).absolute())
