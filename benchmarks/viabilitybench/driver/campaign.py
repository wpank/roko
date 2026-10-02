"""`vb campaign`: validate an experiment manifest, estimate it, and run its blocks in order (S09 E1a, E5, E8; 3306).

    vb campaign --manifest experiments/pilot_a.toml --dry-run          # validate and estimate; nothing runs
    vb campaign --manifest experiments/pilot_a.toml --allow-network    # run it: one `vb run` per unit
    vb campaign --manifest M --provider-url http://127.0.0.1:P/v1 --results R --work W   # an offline rehearsal

**The manifest** (`vb.experiment/1`, `schema/experiment.schema.json`) is a TOML file in `experiments/`: the
experiment id every run gets (`vb run --experiment`), an optional `prereg_id`, `requires_lock` and `requires_live`
(which 3341 and 3359 enforce; until then a manifest that sets either is refused before it runs), an `order`, and its
blocks. A block is one cell of the experiment: a stream, an arm, a model, seeds (`vb run`'s `--seeds` form), a budget
line and, optionally, a disturbance spec, an instance subset of the stream, the secret file's fingerprint its
instances are audited under, the `--max-cost-usd` a network run needs, its planned billed spend (S09 §3's figures,
required for a billed block), `optional` (run only with `--include`) and `off_hours` (a note: the subscription is
shared).

**The order.** `as_listed` runs each block as one unit, in the manifest's order. `daily_interleave` (S09 §4.1, "arms
are interleaved in randomized daily blocks") makes one unit per (block, seed), puts a block's k-th seed on day k, and
orders each day's units by a keyed shuffle of the manifest id, the order's seed and the day. Each unit is one
`vb run`, whose run id is `<unit>-<attempt>`.

**Validation** (`--dry-run`, and before anything runs) refuses, before any call or directory:
- an arm, a stream, a disturbance spec or a line that does not exist; a line held back (`[reserved]`) or outside
  the experiment cap's lines; an instance outside its stream; a model outside its arm's `models_allow` or without a
  row in the price snapshot;
- a network block without `max_cost_usd`, or whose one task can cost more than it (`vb.admit`), and a billed block
  without `planned_usd`;
- a block whose planned spend would take its line, its experiment cap or the programme stop past the cap, counting
  what the ledger already holds (`ledger.read_books` over the results root) and the planned spend of the blocks
  before it that have not run; and a block whose first task could not start (`vb run`'s own admission: what is held,
  plus those earlier blocks' plans, plus the most one task can cost). The whole block's worst case is shown and not
  enforced: `vb run` enforces it task by task, and Pilot A's cheap_direct worst case alone ($17.40) passes BL0;
- a secret file whose fingerprint differs from the one a block names, and an instance that a unit of any campaign
  in the results root already ran under another secret's fingerprint (S09 §4.1: each instance is audited under one
  secret file for the whole campaign). Without a readable secret file the dry run says the secret was not checked.
The dry run prints the experiment, its units in order and, per block, its runs, planned spend and worst case, and
per line and cap what is held, planned and left, as JSON. It exits 2 when anything is refused.

**Running** (without `--dry-run`). Each unit runs `vb.py run` as a process of its own, with the operator's
environment, so its own checks and its restart into an allowlisted environment work as when typed by hand. A network
unit needs `--allow-network` on the campaign and gets `--allow-network --max-cost-usd <the block's>`. An offline
rehearsal passes `--provider-url` (a loopback URL) to every unit, and may swap an arm's file for a rehearsal one
(`--arm-file ID=PATH`, refused without a loopback `--provider-url`). The campaign appends a `start` and a `finish`
event per unit to `$VB_RESULTS/<experiment>/campaign.jsonl`, with the run id, the secret's fingerprint and the
instances. A block's instance subset becomes a stream file under `<experiment>/.campaign/`, which `vb report` skips.

**Resuming.** A rerun skips every unit whose run exited 0 and starts at the first other one. An earlier attempt of
that unit that exited otherwise, or never finished, is retried under a new run id when its run directory holds no
records and no ledger rows. Otherwise the campaign stops: the attempt's records would count twice. Move its run
directory to `<results>/<experiment>.abandoned/` (inside the results root, so the ledger still counts its spend) and
run the campaign again.

API:
    load(path) -> Manifest                                   # raises CampaignError
    units(manifest, include=()) -> list[Unit]
    check(vb, manifest, *, budget, results_root, include=(), provider_url=None, arm_files=None,
          secret_fingerprint=None) -> Check
    cmd_campaign(vb, args) -> int; add_arguments(parser)
    CampaignError, SCHEMA, ORDERS
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
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
NEEDS_PROXY = ("provider_fault", "model_swap")  # disturbances the metering proxy applies (disturb.py)


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

    def summary(self) -> dict:
        return {"experiment": self.manifest.id, "manifest": str(self.manifest.path), "order": self.manifest.order,
                "requires_lock": self.manifest.requires_lock, "requires_live": list(self.manifest.requires_live),
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


def check(vb, manifest: Manifest, *, budget: ledger.Budget, results_root: Path, include: Iterable[str] = (),
          provider_url: str | None = None, arm_files: dict[str, str] | None = None,
          secret_fingerprint: str | None = None, finished: Iterable[str] = ()) -> Check:
    """Validate `manifest` against the arms, streams, snapshot, budget and ledger (module docstring). `vb` is the
    driver module; `finished` names the units already done, whose spend the ledger holds."""
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
    if manifest.requires_lock:
        found.notes.append("requires_lock: the pre-registration lock is not checked by this driver (3341), so the "
                           "campaign refuses to run until it is")
    if manifest.requires_live:
        found.notes.append(f"requires_live ({', '.join(manifest.requires_live)}): not checked by this driver "
                           "(3359), so the campaign refuses to run until it is")
    try:
        books = ledger.read_books(results_root)
    except ledger.BudgetError as err:
        found.problems.append(f"the ledger under {results_root} cannot be read: {err}")
        books = ledger.Books([], [])
    pending: dict[str, float] = {}  # scope -> planned spend of the included blocks before this one, not yet run
    for block in manifest.blocks:
        included = not block.optional or block.id in include
        entry = {"block": block.id, "included": included, "stream": block.stream, "arm": block.arm,
                 "model": block.model, "seeds": block.seeds_text, "line": block.line, "optional": block.optional,
                 "off_hours": block.off_hours}
        found.blocks[block.id] = entry
        mine = [unit.key for unit in plan_units if unit.block.id == block.id]
        left = sum(1 for key in mine if key not in set(finished)) / len(mine) if mine else 0.0
        try:
            _check_block(vb, found, block, entry, budget, books, pending, provider_url, arm_files or {},
                         secret_fingerprint, left=left)
        except _Unavailable as err:
            entry["unavailable"] = str(err)
            (found.notes if block.optional and not included else found.problems).append(f"block {block.id}: {err}")
    _check_secrets(found, results_root, secret_fingerprint)
    return found


def cmd_campaign(vb, args: argparse.Namespace) -> int:
    """`vb campaign` (module docstring). `vb` is the driver module, which this one does not import itself."""
    manifest = load(_manifest_path(args.manifest))
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
                  finished=finished)
    if fingerprint is None:
        found.notes.append(f"secret: not checked ({why})")
    if args.dry_run:
        print(json.dumps(found.summary(), indent=2, ensure_ascii=False))
        return 2 if found.problems else 0
    if found.problems:
        raise CampaignError("refused before any run: " + "; ".join(found.problems))
    if manifest.requires_lock or manifest.requires_live:
        raise CampaignError("the manifest requires the pre-registration lock or live loops, which this driver cannot "
                            "check yet (3341, 3359)")
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
        attempt = _next_attempt(log, events, results_root / manifest.id, unit)
        code = _run_unit(vb, args, manifest, found, unit, attempt, results_root, arm_files, fingerprint, log)
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
    parser.add_argument("--arm-file", action="append", metavar="ID=PATH",
                        help="a rehearsal's arm file for arm ID (needs a loopback --provider-url)")
    parser.add_argument("--secret-file", type=Path, help="passed to every unit's vb run")
    parser.add_argument("--key-file", type=Path, help="passed to every unit's vb run")
    parser.add_argument("--results", type=Path, help="passed to every unit's vb run")
    parser.add_argument("--work", type=Path, help="passed to every unit's vb run")
    parser.add_argument("--transcripts", action="store_true", help="passed to every unit's vb run")


class _Unavailable(Exception):
    """A block refers to something that does not exist, so nothing else of it can be checked."""


def _check_block(vb, found: Check, block: Block, entry: dict, budget: ledger.Budget, books: ledger.Books,
                 pending: dict[str, float], provider_url: str | None, arm_files: dict[str, str],
                 fingerprint: str | None, *, left: float) -> None:
    """One block's checks (module docstring). `left` is the share of its units that have not run yet."""
    where = f"block {block.id}"
    try:
        arm = vb.load_arm(arm_files.get(block.arm, block.arm))
        stream = vb.load_stream(block.stream)
    except vb.DriverError as err:
        raise _Unavailable(str(err)) from None
    if arm["arm"]["id"] != block.arm:
        found.problems.append(f"{where}: {arm['path']} is arm {arm['arm']['id']}, not {block.arm}")
    instances = block.instances or tuple(stream.instances)
    outside = [instance for instance in instances if instance not in stream.instances]
    if outside:
        found.problems.append(f"{where}: {', '.join(outside)} not in stream {stream.id}")
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
    entry.update(network=network, provider=plan.endpoint.provider, worst_task_usd=worst,
                 worst_case_usd=None if worst is None else round(worst * entry["runs"], 6),
                 planned_usd=block.planned_usd if arm["arm"]["billed"] else 0.0, max_cost_usd=block.max_cost_usd)
    if network:
        if block.max_cost_usd is None:
            found.problems.append(f"{where}: a network run needs max_cost_usd, its --max-cost-usd")
        elif row is not None:
            try:
                vb.admit(plan, allow_network=True, max_cost_usd=block.max_cost_usd)
            except vb.DriverError as err:
                found.problems.append(f"{where}: {err}")
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
    _check_budget(found, block, entry, budget, books, pending, worst if arm["arm"]["billed"] else 0.0, left=left)


def _check_budget(found: Check, block: Block, entry: dict, budget: ledger.Budget, books: ledger.Books,
                  pending: dict[str, float], worst: float | None, *, left: float) -> None:
    """The budget rules of the module docstring, for one block in run order: of its planned spend, only the share of
    its units still to run counts, since the ledger holds what the others spent."""
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
    if not entry["included"]:
        return
    for name, cap, member in scopes:
        spent, reserved = books.sums(member)
        held = round(float(spent + reserved), 6)
        before = pending.get(name, 0.0)
        scope = found.scopes.setdefault(name, {"cap_usd": cap, "held_usd": held, "planned_usd": 0.0,
                                               "left_usd": round(cap - held, 6)})
        if not left:  # every unit ran: the ledger holds its spend
            continue
        if round(held + before + planned, 6) > cap:
            found.problems.append(f"{where}: {name}: ${held:.4f} held + ${before:.4f} planned before it "
                                  f"+ ${planned:.4f} planned for it would pass ${cap:.2f}")
        elif worst is not None and round(held + before + worst, 6) > cap:
            found.problems.append(f"{where}: {name}: its first task could not start: ${held:.4f} held "
                                  f"+ ${before:.4f} planned before it + ${worst:.4f} for one task would pass "
                                  f"${cap:.2f}")
        pending[name] = round(before + planned, 6)
        scope["planned_usd"] = pending[name]
        scope["left_usd"] = round(cap - held - pending[name], 6)


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


def _run_unit(vb, args: argparse.Namespace, manifest: Manifest, found: Check, unit: Unit, run_id: str,
              results_root: Path, arm_files: dict[str, str], fingerprint: str, log: Path) -> int:
    """One unit as `vb.py run` in a process of its own; its start and finish go to the campaign log."""
    block = unit.block
    plan = found.blocks[block.id]
    stream = block.stream
    if block.instances is not None:
        stream = str(_derived_stream(vb, results_root / manifest.id / STREAMS, block))
    argv = [sys.executable, str(layout.DRIVER_DIR / "vb.py"), "run", "--experiment", manifest.id, "--run-id",
            run_id, "--stream", stream, "--arm", arm_files.get(block.arm, block.arm), "--model", block.model,
            "--seeds", unit.seeds_text, "--line", block.line]
    if plan["network"]:
        argv += ["--allow-network", "--max-cost-usd", f"{block.max_cost_usd:g}"]
    if args.provider_url:
        argv += ["--provider-url", args.provider_url]
    if block.disturbance:
        argv += ["--disturbance", str(_config(block.disturbance))]
        if set(plan.get("disturbances", ())) & set(NEEDS_PROXY) and not plan["network"]:
            argv.append("--proxy")
    for flag, value in (("--secret-file", args.secret_file), ("--key-file", args.key_file),
                        ("--results", results_root), ("--work", args.work)):
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
