#!/usr/bin/env python3
"""`vb`, the ViabilityBench driver (S08 §5.7). This item builds `run`, `estimate` and `materialize`.

    vb run --experiment PILOT-A --stream pilot --arm cheap_direct --model gpt-oss-120b --seeds 1-3 \
           --allow-network --max-cost-usd 10 [--line BL0] [--limit N] [--transcripts] [--keep-workdirs]
    vb estimate --stream pilot --arm cheap_direct --model gpt-oss-120b --seeds 1-3
    vb materialize --stream pilot --instance F1-l1-0001 --out DIR

`vb run` runs every (task, seed) of a stream on one arm and one model, in a fresh workdir under `$VB_WORK` (default
`~/vb-work/<run_id>/`), outside the repository. For each one it: materializes the task (`materialize`); runs the
arm's runner module (`harness`: the arm file's `runner`, or its `harness` id with "-" read as "_", so a new arm adds
files and never edits this one); commits the final tree as c_i without touching the agent's repo and archives it
(`archive`); labels c_i by VS-census (`census`); and appends a `vb.run_record/1` row (`records`). Ledger rows
(`ledger`) are appended by the runner as attempts end. Everything lands in `$VB_RESULTS/<experiment>/<run_id>/`
(default `~/.roko-bench/viability`): `manifest.json`, `order-<seed>.json`, `records.jsonl`, `ledger.jsonl`,
`archives/`, `private/` (task manifests and pristine bundles, never an agent's), `errors.jsonl` and, with
`--transcripts`, `transcripts/`. Stream positions in records are 1-based.

**Network admission** (from `scripts/dev_benchmark.py`'s `execute`, W10 rec 14). A provider whose base URL is not a
loopback address is a network provider. `vb run` calls one only with both `--allow-network` and an explicit
`--max-cost-usd`, and the check runs before anything else: no directory, no workdir, no request. The budget is then
enforced task by task rather than on the whole run's worst case: a task starts only while the ledger's spend plus
the most one task can cost under the arm's caps (priced from the snapshot) still fits, so the run stops early
instead of overspending. dev_benchmark refuses when the whole run's worst case exceeds the budget; here that would
need a $17 flag for a pilot budgeted at $10 (60 cheap_direct runs at $0.29 worst case each, against about $0.02
typical), and the flag would then no longer cap anything. `vb estimate` and the run manifest show the whole-run
worst case. A model without a price row cannot be bounded and is refused. `--provider-url` with a loopback URL (such
as `stub_provider`'s) runs offline, with neither flag.

The benchmark secret is read only by the census, as a file path handed to `hidden.py`: `--secret-file`, else
`$VB_SECRET_FILE`, else `~/.config/viabilitybench/secret` (mode 0600). `vb run` checks the file's mode before the
first task, without reading it. Keeping it away from agents is gap-a8a160's (`agent_env` is the seam).

Exit status: 0 when every (task, seed) got a record, 1 when some did not (see `errors.jsonl`), 2 for a usage,
configuration or admission error.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib
import json
import os
import re
import secrets
import stat
import sys
import tomllib
from dataclasses import asdict, dataclass
from pathlib import Path
from types import ModuleType

import agent_env
import archive
import caps
import census
import harness
import layout
import ledger
import materialize
import provider
import records
from common import hmac_seed, knobs, repo

DRIVER_VERSION = "vb-driver-1.0.0"
DEFAULT_SECRET_FILE = Path("~/.config/viabilitybench/secret")
DEFAULT_RESULTS = Path("~/.roko-bench/viability")
DEFAULT_WORK = Path("~/vb-work")
ID_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,63}")
SKIP_IN_SUITE_HASH = ("__pycache__", ".pytest_cache", ".venv")


class DriverError(RuntimeError):
    """A usage, configuration or admission error: `vb` exits 2 before running anything."""


@dataclass(frozen=True)
class Stream:
    id: str
    path: Path
    families: dict[str, Path]
    instances: list[str]
    spec_variant: str

    def family_dir(self, instance_id: str) -> Path:
        return self.families[knobs.parse_instance_id(instance_id)[0]]

    def order(self, seed: int) -> list[str]:
        """The stream's fixed order for one run seed (S08 §4.7): a keyed shuffle, the same on every host."""
        return hmac_seed.surface_stream("stream", f"{self.id}/order/{seed}").shuffled(self.instances)


@dataclass(frozen=True)
class Run:
    """What every task of one `vb run` shares."""

    args: argparse.Namespace
    plan: Plan
    runner: ModuleType
    chat: provider.ChatProvider
    book: ledger.Ledger
    secret_file: Path
    run_dir: Path
    work_dir: Path
    run_id: str
    config_hash: str
    head: tuple[str, bool]
    suite: dict


@dataclass(frozen=True)
class Plan:
    arm: dict
    stream: Stream
    model: str
    seeds: list[int]
    instances: list[str]
    snapshot: ledger.Snapshot
    endpoint: provider.Endpoint
    caps: caps.Caps
    worst_task_usd: float | None

    @property
    def runs(self) -> int:
        return len(self.instances) * len(self.seeds)

    @property
    def worst_case_usd(self) -> float | None:
        return None if self.worst_task_usd is None else self.worst_task_usd * self.runs

    def summary(self) -> dict:
        return {"arm": self.arm["arm"]["id"], "stream": self.stream.id, "model": self.model, "seeds": self.seeds,
                "tasks": len(self.instances), "runs": self.runs, "provider": self.endpoint.provider,
                "base_url": self.endpoint.base_url, "network": not self.endpoint.offline,
                "price_snapshot_id": self.snapshot.id, "worst_task_usd": self.worst_task_usd,
                "worst_case_usd": self.worst_case_usd, "caps": asdict(self.caps)}


def main(argv: list[str] | None = None) -> int:
    forwarded = sys.argv[1:] if argv is None else argv
    if forwarded[:1] == ["report"]:
        return run_report(forwarded[1:])
    args = _parser().parse_args(argv)
    args.argv = ["vb", *(sys.argv[1:] if argv is None else argv)]
    try:
        return args.handler(args)
    except (DriverError, caps.CapError, ledger.PriceError) as err:
        print(f"vb: {err}", file=sys.stderr)
        return 2


def run_report(argv: list[str]) -> int:
    """`vb report` (S08 §5.7) is `analysis/report.py`, which parses its own flags: argparse cannot pass them through."""
    sys.path.insert(0, str(layout.VB_ROOT / "analysis"))
    return importlib.import_module("report").main(argv)


def admit(plan: Plan, *, allow_network: bool, max_cost_usd: float | None) -> None:
    """Fail closed before any network call (the module docstring has the rule)."""
    if plan.endpoint.offline:
        return
    if not allow_network:
        raise DriverError(f"{plan.endpoint.base_url} is a network provider; inspect `vb estimate` and pass "
                          "--allow-network explicitly")
    if max_cost_usd is None:
        raise DriverError("network runs require an explicit --max-cost-usd budget")
    if plan.worst_task_usd is None:
        raise DriverError(f"{plan.model} has no row in {plan.snapshot.id}, so its cost cannot be bounded")
    if plan.worst_task_usd > max_cost_usd:
        raise DriverError(f"one task can cost up to ${plan.worst_task_usd:.2f} under the arm's caps, more than "
                          f"--max-cost-usd ${max_cost_usd:.2f}")


def make_plan(args: argparse.Namespace) -> Plan:
    arm = load_arm(args.arm)
    stream = load_stream(args.stream)
    allowed = arm["arm"]["models_allow"]
    if args.model not in allowed:
        raise DriverError(f"arm {arm['arm']['id']} allows {', '.join(allowed)}, not {args.model}")
    snapshot = ledger.load_snapshot(args.price_snapshot)
    row = snapshot.row(args.model)
    endpoint = _endpoint(arm, row, getattr(args, "provider_url", None))
    arm_caps = caps.Caps.from_table(arm.get("caps", {}))
    instances = stream.instances[:args.limit] if args.limit else stream.instances
    return Plan(arm=arm, stream=stream, model=args.model, seeds=parse_seeds(args.seeds), instances=instances,
                snapshot=snapshot, endpoint=endpoint, caps=arm_caps, worst_task_usd=caps.worst_task_usd(arm_caps, row))


def load_arm(name: str) -> dict:
    path = _config_path(name, layout.ARMS_DIR)
    doc = _toml(path)
    arm = doc.get("arm")
    if doc.get("schema_version") != "vb.arm/1" or not isinstance(arm, dict):
        raise DriverError(f"{path}: not a vb.arm/1 file with an [arm] table")
    for key, kind in (("id", str), ("harness", str), ("models_allow", list), ("line", str), ("billed", bool)):
        if not isinstance(arm.get(key), kind):
            raise DriverError(f"{path}: [arm] {key} must be a {kind.__name__}")
    for name_, table in doc.get("providers", {}).items():
        if not isinstance(table, dict) or not isinstance(table.get("base_url"), str):
            raise DriverError(f"{path}: [providers.{name_}] needs a base_url")
    doc["path"] = str(path)
    return doc


def load_stream(name: str) -> Stream:
    path = _config_path(name, layout.STREAMS_DIR)
    doc = _toml(path)
    table = doc.get("stream")
    if doc.get("schema_version") != "vb.stream/1" or not isinstance(table, dict):
        raise DriverError(f"{path}: not a vb.stream/1 file with a [stream] table")
    families = {family: (layout.VB_ROOT / directory).resolve()
                for family, directory in table.get("families", {}).items()}
    instances = table.get("instances", [])
    if not instances or len(set(instances)) != len(instances):
        raise DriverError(f"{path}: instances must be a non-empty list without repeats")
    for instance in instances:
        try:
            family = knobs.parse_instance_id(instance)[0]
        except ValueError as err:
            raise DriverError(f"{path}: {err}") from None
        if family not in families:
            raise DriverError(f"{path}: {instance}'s family {family} has no directory in [stream] families")
    return Stream(id=table["id"], path=path, families=families, instances=list(instances),
                  spec_variant=table.get("spec_variant", "precise"))


def parse_seeds(text: str) -> list[int]:
    """"1-3" -> [1, 2, 3]; "1,3" -> [1, 3]."""
    seeds: list[int] = []
    for part in text.split(","):
        low, _, high = part.strip().partition("-")
        if not low.isdigit() or (high and not high.isdigit()):
            raise DriverError(f"bad --seeds {text!r}: use forms like 1-3 or 1,3")
        seeds += range(int(low), int(high or low) + 1)
    if not seeds or len(set(seeds)) != len(seeds):
        raise DriverError(f"bad --seeds {text!r}: empty or repeated")
    return seeds


def cmd_estimate(args: argparse.Namespace) -> int:
    print(json.dumps(make_plan(args).summary(), indent=2))
    return 0


def cmd_materialize(args: argparse.Namespace) -> int:
    stream = load_stream(args.stream)
    if args.instance not in stream.instances:
        raise DriverError(f"{args.instance} is not in stream {stream.id}")
    out = Path(args.out).absolute()
    private = Path(args.private).absolute() if args.private else out.with_name(out.name + ".private")
    try:
        done = materialize.materialize(family_dir=stream.family_dir(args.instance), instance_id=args.instance,
                                       workdir=out, private_dir=private, spec_variant=stream.spec_variant)
    except (materialize.MaterializeError, repo.RepoError) as err:
        print(f"vb: {err}", file=sys.stderr)
        return 1
    print(json.dumps({"workdir": str(done.workdir), "private_dir": str(done.private_dir),
                      "manifest": str(done.manifest_path), "pristine": done.pristine.as_json()}, indent=2))
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    plan = make_plan(args)
    admit(plan, allow_network=args.allow_network, max_cost_usd=args.max_cost_usd)
    if args.max_cost_usd is not None and (plan.worst_case_usd or 0) > args.max_cost_usd:
        print(f"vb: the worst case of {plan.runs} runs is ${plan.worst_case_usd:.2f}; the run stops before any task "
              f"that could take spend past ${args.max_cost_usd:.2f}", file=sys.stderr)
    if not plan.endpoint.offline and not os.environ.get(plan.endpoint.api_key_env or ""):
        raise DriverError(f"set {plan.endpoint.api_key_env} for {plan.endpoint.provider}")
    for family, directory in sorted(plan.stream.families.items()):
        if any(knobs.parse_instance_id(i)[0] == family for i in plan.instances) and not all(
                (directory / name).is_file() for name in ("gen.py", "hidden.py")):
            raise DriverError(f"family {family} at {directory} needs gen.py and hidden.py")
    secret_file = _secret_file(args.secret_file)
    runner = load_runner(plan.arm)
    results_root = _outside_repo(args.results or os.environ.get("VB_RESULTS") or DEFAULT_RESULTS, "--results")
    work_root = _outside_repo(args.work or os.environ.get("VB_WORK") or DEFAULT_WORK, "--work")
    if layout.within(work_root, results_root) or layout.within(results_root, work_root):
        raise DriverError("--work and --results must not contain each other")
    for value, flag in ((args.experiment, "--experiment"), (args.run_id or "x", "--run-id")):
        if not ID_RE.fullmatch(value):
            raise DriverError(f"{flag} must match {ID_RE.pattern}")
    run_id = args.run_id or f"vb-{dt.datetime.now(dt.UTC):%Y%m%d}-{secrets.token_hex(3)}"
    run_dir = results_root / args.experiment / run_id
    if run_dir.exists():
        raise DriverError(f"{run_dir} already exists; pick another --run-id")
    run_dir.mkdir(mode=0o700, parents=True)
    work_dir = work_root / run_id
    work_dir.mkdir(mode=0o700, parents=True, exist_ok=True)

    line = args.line or plan.arm["arm"]["line"]
    config = {"driver": DRIVER_VERSION, "arm": {k: v for k, v in plan.arm.items() if k != "path"},
              "model": plan.model, "endpoint": asdict(plan.endpoint), "caps": asdict(plan.caps),
              "stream": {"id": plan.stream.id, "instances": plan.instances, "spec_variant": plan.stream.spec_variant},
              "price_snapshot_id": plan.snapshot.id, "line": line,
              "prompt": {"version": getattr(runner, "PROMPT_VERSION", None),
                         "sha256": getattr(runner, "PROMPT_SHA256", None)}}
    config_hash = records.canonical_hash(config)
    head = records.harness_state()
    suite = {"id": "vb", "hash": _suite_hash(plan)}
    _write_json(run_dir / "manifest.json", {
        "schema_version": "vb.run_manifest/1", "experiment_id": args.experiment, "run_id": run_id,
        "started_at": harness.utc_now(), "argv": args.argv, "harness_sha": head[0],
        "dirty": head[1], "config_hash": config_hash, "config": config, "suite": suite,
        "offline": plan.endpoint.offline,
        "secret_file": str(secret_file), "work_dir": str(work_dir), **plan.summary()})
    book = ledger.Ledger(run_dir / "ledger.jsonl", line=line, experiment_id=args.experiment, run_id=run_id,
                         price_snapshot_id=plan.snapshot.id)
    run = Run(args=args, plan=plan, runner=runner, chat=provider.OpenAICompatible(plan.endpoint), book=book,
              secret_file=secret_file, run_dir=run_dir, work_dir=work_dir, run_id=run_id, config_hash=config_hash,
              head=head, suite=suite)
    written = 0
    try:
        for seed in plan.seeds:
            order = [instance for instance in plan.stream.order(seed) if instance in plan.instances]
            _write_json(run_dir / f"order-{seed}.json", {"stream": plan.stream.id, "seed": seed, "order": order})
            for position, instance_id in enumerate(order, 1):
                if args.max_cost_usd is not None and plan.worst_task_usd is not None and \
                        book.spent_bound_usd + plan.worst_task_usd > args.max_cost_usd:
                    _log_error(run_dir, f"{instance_id}.s{seed}", "budget",
                               f"stopped: ${book.spent_bound_usd:.4f} spent; the next task could pass --max-cost-usd")
                    return 1
                written += _run_one(run, instance_id, seed, {"id": plan.stream.id, "position": position,
                                                             "length": len(order), "perturbations_active": []})
    finally:
        _cleanup(args, [work_dir])
        print(f"vb: {written}/{plan.runs} records in {run_dir}", file=sys.stderr)
    return 0 if written == plan.runs else 1


def _run_one(run: Run, instance_id: str, seed: int, stream_position: dict) -> bool:
    """One (task, seed): materialize, run, commit and archive c_i, label it, and append its record."""
    args, plan, run_dir, work_dir = run.args, run.plan, run.run_dir, run.work_dir
    key = f"{instance_id}.s{seed}"
    workdir, private = work_dir / key, run_dir / "private" / key
    homes = [work_dir / "_home" / key, work_dir / "_home" / f"{key}.census"]
    try:
        task = materialize.materialize(family_dir=plan.stream.family_dir(instance_id), instance_id=instance_id,
                                       workdir=workdir, private_dir=private, spec_variant=plan.stream.spec_variant)
    except (materialize.MaterializeError, repo.RepoError, OSError, ValueError) as err:
        _log_error(run_dir, key, "materialize", err)
        _cleanup(args, [workdir])
        return False
    ctx = harness.TaskContext(
        experiment_id=args.experiment, run_id=run.run_id, arm=plan.arm, model=plan.model, endpoint=plan.endpoint,
        provider=run.chat, snapshot=plan.snapshot, caps=plan.caps, ledger=run.book, billed=plan.arm["arm"]["billed"],
        instance_id=instance_id, seed=seed, key=key, workdir=workdir, spec_text=task.spec_text,
        agent_env=agent_env.build(home=homes[0]), visible_verify=tuple(task.manifest["visible_verify"]),
        files_in_scope=tuple(task.manifest["files_in_scope"]))
    try:
        outcome = run.runner.run_task(ctx)
    except Exception as err:  # the runner owns its errors; this catches its bugs
        now = harness.utc_now()
        outcome = harness.TaskOutcome("infra_error", f"runner crashed: {type(err).__name__}: {err}", [], [], now, now)
    final = archived = None
    transcript_text = json.dumps(outcome.transcript, ensure_ascii=False)
    try:
        final = archive.commit_final(workdir, private / "final")
        archived = archive.archive_task(final, task.pristine, run_dir / "archives", key)
        result = census.run_census(final=final, manifest=task.manifest, manifest_path=task.manifest_path,
                                   pristine=task.pristine, family_dir=task.family_dir, secret_file=run.secret_file,
                                   completed=outcome.status == "completed", transcript_text=transcript_text,
                                   diff_text=archived.diff_text, scratch=private / "census",
                                   env=agent_env.build(home=homes[1]))
    except (archive.ArchiveError, repo.RepoError, OSError) as err:
        result = census.CensusResult(completion=int(outcome.status == "completed"), visible_clean=None, hidden=None,
                                     integrity=None, infra_error=f"archive or census failed: {err}")
    transcript_ref = None
    if args.transcripts:
        (run_dir / "transcripts").mkdir(mode=0o700, exist_ok=True)
        transcript_ref = f"transcripts/{key}.json"
        (run_dir / transcript_ref).write_text(transcript_text, encoding="utf-8")
    record = records.build(experiment_id=args.experiment, run_id=run.run_id, arm_id=plan.arm["arm"]["id"],
                           seed=seed, head=run.head, billed=plan.arm["arm"]["billed"], config_hash=run.config_hash,
                           snapshot_id=plan.snapshot.id,
                           suite={**run.suite, "generator_versions": {task.manifest["family"]:
                                                                  task.manifest["generator_version"]}},
                           stream=stream_position, materialized=task, outcome=outcome, result=result, final=final,
                           archived=archived, transcript_ref=transcript_ref)
    try:
        records.append(run_dir / "records.jsonl", record)
    except records.RecordError as err:
        _log_error(run_dir, key, "record", err)
        return False
    finally:
        _cleanup(args, [workdir, *homes, private / "final", private / "census"])
    cost = record["costs"]["api_equiv_usd"]
    print(f"vb: {key} {record['execution']['status']} ({outcome.reason}) VS={record['vs']['label']} "
          f"turns={sum(a.turns for a in outcome.attempts)} cost={'unknown' if cost is None else f'${cost:.4f}'}",
          file=sys.stderr)
    return True


def load_runner(arm: dict) -> ModuleType:
    """The arm's runner module from driver/: `[arm] runner`, or the harness id with "-" read as "_"."""
    name = arm["arm"].get("runner") or arm["arm"]["harness"].replace("-", "_")
    if not re.fullmatch(r"[a-z_][a-z0-9_]*", name) or not (layout.DRIVER_DIR / f"{name}.py").is_file():
        raise DriverError(f"arm {arm['arm']['id']}: no runner module driver/{name}.py")
    module = importlib.import_module(name)
    if not callable(getattr(module, "run_task", None)):
        raise DriverError(f"driver/{name}.py has no run_task(ctx)")
    return module


def _endpoint(arm: dict, row: dict | None, provider_url: str | None) -> provider.Endpoint:
    providers = arm.get("providers", {})
    name = row["provider"] if row else (next(iter(providers)) if len(providers) == 1 else None)
    if name is None or (name not in providers and not provider_url):
        raise DriverError(f"arm {arm['arm']['id']} has no [providers.{name}] endpoint for this model")
    table = providers.get(name, {})
    # An overriding URL (a stub, later the metering proxy) never gets the provider's key.
    keys = ("max_tokens_param", "timeout_s") if provider_url else ("api_key_env", "max_tokens_param", "timeout_s")
    fields = {key: table[key] for key in keys if key in table}
    return provider.Endpoint(provider=name, base_url=provider_url or table["base_url"], **fields)


def _secret_file(value: Path | None) -> Path:
    path = Path(value or os.environ.get("VB_SECRET_FILE") or DEFAULT_SECRET_FILE).expanduser().absolute()
    try:
        info = path.lstat()
    except OSError:
        raise DriverError(f"no secret file at {path}; create one (mode 0600) or pass --secret-file") from None
    if not stat.S_ISREG(info.st_mode) or info.st_mode & 0o077:
        raise DriverError(f"the secret file {path} must be a regular file with mode 0600")
    if layout.within(path, layout.REPO_ROOT):
        raise DriverError("the secret file must live outside the repository")
    return path


def _outside_repo(value: Path | str, flag: str) -> Path:
    path = Path(value).expanduser().absolute()
    if layout.within(path, layout.REPO_ROOT) or layout.within(path, Path("~/.roko").expanduser()):
        raise DriverError(f"{flag} {path} must be outside the repository and ~/.roko")
    return path


def _suite_hash(plan: Plan) -> str:
    """sha256 over the files of the stream's families and the common library (caches excluded)."""
    digests = {}
    for name, directory in sorted({**plan.stream.families, "common": layout.FAMILIES_DIR / "common"}.items()):
        files = sorted(p for p in Path(directory).rglob("*") if p.is_file() and not p.is_symlink()
                       and not set(p.relative_to(directory).parts) & set(SKIP_IN_SUITE_HASH) and p.suffix != ".pyc")
        digests[name] = {p.relative_to(directory).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
    return records.canonical_hash(digests)


def _config_path(name: str, directory: Path) -> Path:
    path = Path(name) if name.endswith(".toml") or "/" in name else directory / f"{name}.toml"
    if not path.is_file():
        raise DriverError(f"no such file: {path}")
    return path.resolve()


def _toml(path: Path) -> dict:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as err:
        raise DriverError(f"{path}: {err}") from None


def _write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")


def _log_error(run_dir: Path, key: str, stage: str, err: object) -> None:
    line = json.dumps({"ts": harness.utc_now(), "task": key, "stage": stage, "error": str(err)[:1000]})
    with (run_dir / "errors.jsonl").open("a", encoding="utf-8") as handle:
        handle.write(line + "\n")
    print(f"vb: {key}: {stage} failed: {str(err)[:300]}", file=sys.stderr)


def _cleanup(args: argparse.Namespace, paths: list[Path]) -> None:
    if args.keep_workdirs:
        return
    for path in paths:
        archive.remove_tree(path)


def _positive_float(text: str) -> float:
    try:
        value = float(text)
    except ValueError:
        raise argparse.ArgumentTypeError("must be a number") from None
    if not value > 0 or value == float("inf"):
        raise argparse.ArgumentTypeError("must be a finite number above 0")
    return value


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="vb", description="The ViabilityBench driver (S08 §5.7).",
                                     allow_abbrev=False)
    commands = parser.add_subparsers(dest="command", required=True)

    def planned(sub: argparse.ArgumentParser) -> None:
        sub.add_argument("--stream", required=True, help="a stream name in streams/, or a path to one")
        sub.add_argument("--arm", required=True, help="an arm name in arms/, or a path to one")
        sub.add_argument("--model", required=True)
        sub.add_argument("--seeds", default="1", help="run seeds, e.g. 1-3 or 1,3 (default 1)")
        sub.add_argument("--limit", type=int, default=0, help="run only the stream's first N instances")
        sub.add_argument("--price-snapshot", default=ledger.DEFAULT_SNAPSHOT)
        sub.add_argument("--provider-url", help="override the provider's base URL; a loopback URL runs offline")

    run = commands.add_parser("run", help="run a stream on one arm and model", allow_abbrev=False)
    planned(run)
    run.add_argument("--experiment", required=True)
    run.add_argument("--run-id")
    run.add_argument("--line", help="budget line for the ledger (default: the arm's)")
    run.add_argument("--allow-network", action="store_true", help="allow calls to a non-loopback provider")
    run.add_argument("--max-cost-usd", type=_positive_float, help="required with --allow-network")
    run.add_argument("--secret-file", type=Path, help="default: $VB_SECRET_FILE, then " + str(DEFAULT_SECRET_FILE))
    run.add_argument("--results", type=Path, help="default: $VB_RESULTS, then " + str(DEFAULT_RESULTS))
    run.add_argument("--work", type=Path, help="default: $VB_WORK, then " + str(DEFAULT_WORK))
    run.add_argument("--transcripts", action="store_true", help="keep transcripts in the run directory")
    run.add_argument("--keep-workdirs", action="store_true", help="keep workdirs and census exports")
    run.set_defaults(handler=cmd_run)

    estimate = commands.add_parser("estimate", help="print the plan and its worst-case cost", allow_abbrev=False)
    planned(estimate)
    estimate.set_defaults(handler=cmd_estimate)

    mat = commands.add_parser("materialize", help="render one instance as the driver would", allow_abbrev=False)
    mat.add_argument("--stream", required=True)
    mat.add_argument("--instance", required=True)
    mat.add_argument("--out", required=True)
    mat.add_argument("--private", help="where the manifest and pristine bundle go (default: OUT.private)")
    mat.set_defaults(handler=cmd_materialize)
    commands.add_parser("report", help="metrics.json and bundle checks (analysis/report.py; see vb report --help)")
    return parser


if __name__ == "__main__":
    sys.exit(main())
