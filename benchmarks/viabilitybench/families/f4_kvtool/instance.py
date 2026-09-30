"""The F4 instance plan: every choice that gen.py, hidden.py and reference/solutions.py must agree on.

An instance is (level, seed), with id `F4-l<level>-<seed>`. Everything public about it is drawn from
`surface_stream("F4", id)`: the knobs, the key namespaces, the exemplar scripts and the module names (`plan`). The
hidden stores are drawn at audit time from `hidden_stream(secret, "F4", id)` (`hidden_cases`), so they exist nowhere
while an agent works. Both are deterministic: the same inputs give the same instance and the same cases. The latent
(`LATENTS`, gen.py's `--latent`) changes only kvtool's convention and the texts that describe it, so both latents
share the plan and the hidden stores.

The migration renames every key that starts with `plan.src` so that it starts with `plan.dst` instead. From
k_quirks = 3 on, `plan.dst` extends `plan.src` (`user:` -> `user:v2:`): the colliding prefixes of S08's ℓ5
hardening, where a second run renames the renamed keys again unless the script excludes them.

API:
    FAMILY, GENERATOR_VERSION, TRUTH_SUITE, LATENTS, LADDER, kvstore (template/lib/kvstore.py: the store format)
    plan(level: int, seed: int) -> Plan
    visible_store(plan: Plan) -> dict                         # the store document of data/store.db
    hidden_cases(plan: Plan, secret: Secret) -> list[Case]    # stores with a write lease: kvtool gets interrupted
    expected_entries(entries: dict, src: str, dst: str) -> dict    # the entries after a correct migration
    store_text(entries: dict, lease: int | None = None) -> str
"""

from __future__ import annotations

import sys
import types
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from common import hmac_seed, knobs, mutate  # noqa: E402

HERE = Path(__file__).resolve().parent
FAMILY = "F4"
GENERATOR_VERSION = "f4-1.1.1"  # 1.1: latent v2, v1 instances unchanged; 1.1.1: kvtool 2.0 waits 150 s, not 900
TRUTH_SUITE = {"id": "f4-truth", "version": "1.1.0"}
LATENTS = ("v1", "v2")  # kvtool's convention: v1 dry-runs unless --apply; v2 writes by default but needs --yes
LADDER = knobs.load_ladder(HERE / "ladder.toml")

NAMESPACES = ("user", "acct", "sess", "cart", "order", "inv", "cfg", "flag", "job", "tenant", "org", "team", "quota",
              "token", "ticket", "coupon", "device", "member", "profile", "plan", "audit", "event", "metric", "queue",
              "route", "shard", "sku", "stock", "vendor", "wallet")
VERSION_TAGS = ("v2", "legacy", "archived", "migrated", "old")
PROJECTS = ("ledgerly", "shipyard", "tallybook", "parcelhub", "quillstack", "brightcart", "harborline", "orbitdesk",
            "pinecrest", "tidewater")
FEATURES = ("billing", "catalog", "inventory", "search", "notify", "audit", "reports", "ledger", "shipping",
            "pricing", "profile", "support", "checkout", "returns", "loyalty", "tax", "fraud", "payout")
SUFFIXES = ("sync", "index", "export", "rules", "cache", "jobs", "views", "store", "api", "batch")
OPS_PACKAGES = ("maintenance", "oncall", "runbooks", "housekeeping")
STYLES = ("stderr", "status", "function")
NAMES = ("ada", "grace", "linus", "alan", "barbara", "edsger", "margaret", "dennis", "ken", "radia", "frances",
         "john", "hedy", "katherine", "donald", "leslie")
TIERS = ("free", "pro", "team", "enterprise")

# Hidden-case density (k_hid): how many hidden stores, and how many hard key names each gets.
HIDDEN_STORES = {"low": 1, "medium": 2, "high": 2, "high_differential": 3}
EDGE_COUNT = {"low": 0, "medium": 3, "high": 6, "high_differential": 8}
DECOY_COUNT = {"low": 0, "medium": 3, "high": 6, "high_differential": 8}
MEDIUM_IDS = ("first last", "a*b", "what?", "[bracketed]", "semi;colon", "dot.ted", "slash/ed")
HIGH_IDS = MEDIUM_IDS + ('quote"d', "it's", "$HOME", "back\\slash", "-rf", "--apply", "zoë", "名前",
                         "x" * 120, "", "a:b:c", "tab\there", "percent%s", "trailing ")
EDGE_IDS = {"low": (), "medium": MEDIUM_IDS, "high": HIGH_IDS, "high_differential": HIGH_IDS}
EDGE_VALUES = ("", "  padded  ", "two\nlines", "café ✓", '{"json": [1, 2]}', "v" * 300)


def _load_kvstore() -> types.ModuleType:
    """template/lib/kvstore.py as a module, compiled in memory so no bytecode lands in the template."""
    path = HERE / "template" / "lib" / "kvstore.py"
    module = types.ModuleType("f4_kvtool_kvstore")
    module.__file__ = str(path)
    exec(compile(path.read_text(encoding="utf-8"), str(path), "exec"), module.__dict__)  # noqa: S102
    return module


kvstore = _load_kvstore()


@dataclass(frozen=True)
class Exemplar:
    """An earlier migration script in the task repo: correct, or misleading (`style == "legacy"`)."""

    path: str
    style: str  # stderr, status, function (correct) or legacy (misleading)
    src: str
    dst: str
    title: str


@dataclass(frozen=True)
class Plan:
    instance_id: str
    level: int
    seed: int
    knobs: dict
    project: str
    src: str
    dst: str
    others: tuple[str, ...]  # the store's other namespaces, each ending in ":"
    exemplars: tuple[Exemplar, ...]
    modules: tuple[tuple[str, str], ...]  # (module name, the namespace it reads)

    @property
    def collide(self) -> bool:
        """True when dst extends src, so that renamed keys still start with src."""
        return self.dst.startswith(self.src)


@dataclass(frozen=True)
class Case:
    """One hidden store: its entries and write lease, and the interruptions a single rename meets."""

    number: int
    entries: dict
    lease: int
    rounds: int


def plan(level: int, seed: int) -> Plan:
    iid = knobs.instance_id(FAMILY, level, seed)
    surface = hmac_seed.surface_stream(FAMILY, iid)
    drawn = LADDER.draw(level, surface.child("knobs"))
    collide = drawn["k_quirks"] >= 3
    n_correct, n_misleading = drawn["k_ex"], drawn["k_misleading"]
    slots = ["src", "dst", "other1", "other2", "other3"]
    for index in range(n_correct + n_misleading):
        slots += [f"ex{index}_src", f"ex{index}_dst"]
    names = mutate.choose(surface.child("namespaces"), {slot: NAMESPACES for slot in slots})
    tags = surface.child("tags")
    src = names["src"] + ":"
    dst = src + tags.choice(VERSION_TAGS) + ":" if collide else names["dst"] + ":"

    places = surface.child("exemplars")
    styles = places.shuffled(STYLES)
    package = places.choice(OPS_PACKAGES)
    exemplars = []
    for index in range(n_correct + n_misleading):
        misleading = index >= n_correct
        e_src = names[f"ex{index}_src"] + ":"
        e_dst = e_src + tags.choice(VERSION_TAGS) + ":" if collide and not misleading else names[f"ex{index}_dst"] + ":"
        year, month = 2024 + places.randbelow(2) - (2 if misleading else 0), 1 + places.randbelow(12)
        stem = f"{_stem(e_src)}_to_{_stem(e_dst)}.sh"
        if misleading:
            path = f"scripts/legacy/{year}_{month:02d}_{stem}"
        elif drawn["k_sibling"]:
            path = f"ops/{package}/{stem}"
        else:
            path = f"scripts/{year}_{month:02d}_{stem}"
        exemplars.append(Exemplar(path=path, style="legacy" if misleading else styles[index % len(styles)],
                                  src=e_src, dst=e_dst,
                                  title=f"Migration {year}-{month:02d}: rename the {e_src} keys to {e_dst}."))
    others = tuple(names[slot] + ":" for slot in ("other1", "other2", "other3")) + tuple(e.dst for e in exemplars)

    modules = surface.child("modules").sample([f"{a}_{b}" for a in FEATURES for b in SUFFIXES], drawn["k_size"])
    return Plan(instance_id=iid, level=level, seed=seed, knobs=drawn,
                project=surface.child("project").choice(PROJECTS), src=src, dst=dst, others=others,
                exemplars=tuple(sorted(exemplars, key=lambda e: e.path)),
                modules=tuple((name, others[index % len(others)]) for index, name in enumerate(sorted(modules))))


def visible_store(plan: Plan) -> dict:
    """The sample store in the task repo: no write lease, so kvtool is never interrupted on it."""
    stream = hmac_seed.surface_stream(FAMILY, plan.instance_id).child("store")
    entries: dict[str, str] = {}
    _add_keys(entries, plan.src, stream.randint(5, 9), stream.child("src"), "low", avoid=plan.dst)
    for namespace in plan.others:
        _add_keys(entries, namespace, stream.randint(2, 5), stream.child(namespace), "low")
    doc = kvstore.new_store(entries)
    history = stream.child("journal")
    for op, exemplar in enumerate((e for e in plan.exemplars if e.style != "legacy"), 1):
        total, lease = history.randint(6, 30), history.randint(4, 9)
        doc["journal"].append({"op": op, "event": "start", "from": exemplar.src, "to": exemplar.dst,
                               "exclude": [exemplar.dst] if exemplar.dst.startswith(exemplar.src) else [],
                               "total": total})
        for done in range(lease, total, lease):
            token = history.token_hex(6)
            doc["journal"].append({"op": op, "event": "interrupted", "token": token, "done": done, "total": total})
            doc["journal"].append({"op": op, "event": "resumed", "token": token})
        doc["journal"].append({"op": op, "event": "done", "renamed": total})
    return doc


def hidden_cases(plan: Plan, secret: hmac_seed.Secret) -> list[Case]:
    """The hidden stores for `plan`. Each has a write lease that interrupts one rename `k_rounds` times."""
    stream = hmac_seed.hidden_stream(secret, FAMILY, plan.instance_id)
    density, rounds = plan.knobs["k_hid"], plan.knobs["k_rounds"]
    cases = []
    for number in range(1, HIDDEN_STORES[density] + 1):
        case = stream.child(f"case{number}")
        wide = density == "high_differential" and number == HIDDEN_STORES[density]
        lease = case.randint(8, 14) if wide else case.randint(3, 6)
        sources = rounds * lease + case.randint(1, lease)  # ceil(sources / lease) == rounds + 1 kvtool runs
        entries: dict[str, str] = {}
        _add_keys(entries, plan.src, sources, case.child("src"), density, avoid=plan.dst)
        if plan.collide and density in ("high", "high_differential"):  # starts like dst, but is a source key
            del entries[sorted(key for key in entries if key.startswith(plan.src))[0]]
            entries[plan.dst[:-1] + "x:" + str(case.randint(1, 99))] = _value(case, density)
        for namespace in plan.others:
            _add_keys(entries, namespace, case.randint(2, 6), case.child("other " + namespace), density)
        _add_decoys(entries, plan, case.child("decoys"), density)
        if sum(key.startswith(plan.src) for key in entries) != sources:
            raise AssertionError(f"{plan.instance_id} case {number}: wrong number of source keys")
        expected_entries(entries, plan.src, plan.dst)  # raises if two keys would collide
        cases.append(Case(number=number, entries=entries, lease=lease, rounds=rounds))
    return cases


def expected_entries(entries: dict, src: str, dst: str) -> dict:
    """The entries after renaming every key that starts with `src` so that it starts with `dst` instead."""
    out: dict[str, str] = {}
    for key, value in entries.items():
        new = dst + key[len(src):] if key.startswith(src) else key
        if new in out:
            raise ValueError(f"the rename would merge two keys into {new!r}")
        out[new] = value
    return out


def store_text(entries: dict, lease: int | None = None) -> str:
    return kvstore.dumps(kvstore.new_store(entries, lease))


def _add_keys(entries: dict, namespace: str, count: int, stream: hmac_seed.Stream, density: str,
              avoid: str | None = None) -> None:
    """Add `count` new keys under `namespace`: some hard names (by density), the rest plain ones."""
    hard = list(stream.sample(EDGE_IDS[density], min(EDGE_COUNT[density], len(EDGE_IDS[density]), count // 2)))
    added = 0
    while added < count:
        rest = hard.pop() if hard else _plain_id(stream)
        key = namespace + rest
        if key in entries or (avoid is not None and avoid.startswith(namespace) and key.startswith(avoid)):
            continue
        entries[key] = _value(stream, density)
        added += 1


def _add_decoys(entries: dict, plan: Plan, stream: hmac_seed.Stream, density: str) -> None:
    """Keys that look like source keys but are not: they share its letters, or hold it after another prefix."""
    base = plan.src[:-1]
    prefixes = (base + "s:", base + "_old:", "x" + plan.src, "cache:" + plan.src)
    for index in range(DECOY_COUNT[density]):
        entries.setdefault(prefixes[index % len(prefixes)] + _plain_id(stream), _value(stream, density))
    if DECOY_COUNT[density]:
        entries.setdefault(base, _value(stream, density))


def _plain_id(stream: hmac_seed.Stream) -> str:
    form = stream.randbelow(3)
    if form == 0:
        return str(stream.randint(100, 99999))
    if form == 1:
        return stream.choice(NAMES) + str(stream.randint(1, 99))
    return stream.choice(NAMES) + "-" + stream.token_hex(2)


def _value(stream: hmac_seed.Stream, density: str) -> str:
    if density in ("high", "high_differential") and stream.randbelow(4) == 0:
        return stream.choice(EDGE_VALUES)
    return f"name={stream.choice(NAMES)};tier={stream.choice(TIERS)};since={stream.randint(2015, 2026)}"


def _stem(namespace: str) -> str:
    return namespace.rstrip(":").replace(":", "_")
