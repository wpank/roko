# Modularity, Composability, and Cleaner Dependencies

> **TL;DR**: Roko's 18 crates have organic boundaries. Some are clean;
> several leak abstractions across layers (confirmed by the audit in
> `docs/00-architecture/23-architectural-analysis-improvements.md`).
> This doc proposes a stricter dependency graph, three new kernel
> crates that capture currently-implicit concepts, and a set of
> rules that make future refactors cheaper. The goal: any subsystem
> can be replaced without touching unrelated subsystems, and new
> integrations can slot in without leaking anywhere they shouldn't.

> **For first-time readers**: Roko has 18 crates today. This doc's
> proposal adds 3 new crates (`roko-bus`, `roko-hdc`, `roko-spi`) and
> splits 2 (`roko-std` → `roko-defaults` + `roko-tools`, `roko-compose`
> → `roko-compose-core` + `roko-templates`) to make the dependency
> story cleaner. Read 03 (Bus promotion) and 17 (plugin SPI) first;
> this doc operationalizes the crate boundaries those propose.

## 1. Current dependency graph problems

From the audit and from `grep`ing cross-crate imports:

1. **`roko-agent` reaches into `roko-learn`** to persist efficiency
   events. It should publish to the Bus; `roko-learn` should
   subscribe.
2. **`roko-cli` imports from almost everything**. Much of this is
   warranted (it's the main binary) but some is accidental
   tight-coupling.
3. **`roko-fs` and `roko-std` have cyclic-shaped references** in
   commentary even if not in actual deps; the substrate-tier
   boundary is fuzzy.
4. **`roko-primitives` (HDC) is leaked into crates** that should
   see it through a trait, not a type.
5. **Role templates live next to template engines** in
   `roko-compose`. A new role should be addable without touching
   the engine.
6. **No `roko-bus` crate** — event-bus code sits in `roko-runtime`
   and is reached for across layers (see `03`).

## 2. Proposed crate reorganization

Three new crates:

### 2.1 `roko-bus`

Extracts the event bus primitives from `roko-runtime` into its own
kernel-tier crate. `roko-core` depends on `roko-bus` for the `Bus`
trait. `roko-runtime` provides a concrete implementation. This is
the direct follow-through on `03-bus-as-first-class.md`.

### 2.2 `roko-hdc`

Extracts HDC from `roko-primitives` into a focused crate with one
job: vectors, encoders, binding/bundling/permutation, similarity.
Everything else currently in `roko-primitives` (tier routing) moves
into its own crate or upstream. This matters because HDC is used
by so many crates that keeping it minimal and well-documented pays
off.

### 2.3 `roko-spi`

A new crate that holds the *extension SPI* from `17`. Third-party
plugins depend only on `roko-spi`, not on kernel crates. This is
the ABI-stability promise made concrete.

Two crate splits:

### 2.4 Split `roko-std` into `roko-defaults` + `roko-tools`

Today `roko-std` mixes default implementations with the 19 builtin
tools. Tools should live alone so new tools can be added without
touching defaults, and defaults should live alone so a minimal
runtime doesn't inherit every builtin.

### 2.5 Split `roko-compose` into `roko-compose-core` + `roko-templates`

Compose's engine (builder, layer stacking) is stable. Templates
(9 roles today, many more tomorrow) should be separately versioned
and third-party-contributable.

## 3. Target dependency graph

```
                 ┌─────────────────────────┐
                 │        roko-spi         │  (plugin ABI)
                 └───────────┬─────────────┘
                             │
                 ┌───────────▼──────────────┐
                 │        roko-core         │  (signals, traits, types)
                 └───┬──────┬───────────┬───┘
                     │      │           │
        ┌────────────▼──┐   │   ┌───────▼─────────┐
        │   roko-bus    │   │   │    roko-hdc     │
        └────────┬──────┘   │   └───────┬─────────┘
                 │          │           │
        ┌────────▼──────────▼───────────▼────────┐
        │          roko-runtime + roko-fs          │ (substrate + bus impls)
        └─────────┬─────────────────────┬──────────┘
                  │                     │
     ┌────────────▼──────┐   ┌──────────▼─────────┐
     │   roko-defaults   │   │     roko-tools     │
     └────────┬──────────┘   └──────────┬─────────┘
              │                         │
     ┌────────▼───────────┐   ┌─────────▼──────────┐
     │  roko-compose-core │   │   roko-templates   │
     └────────┬───────────┘   └────────┬───────────┘
              │                        │
     ┌────────▼────────────────────────▼───────────┐
     │   roko-agent / roko-gate / roko-orchestrator  │
     └──────┬────────────────┬─────────────────┬────┘
            │                │                 │
     ┌──────▼──────┐  ┌──────▼──────┐   ┌──────▼──────┐
     │  roko-learn │  │ roko-neuro  │   │ roko-dreams │
     └──────┬──────┘  └──────┬──────┘   └──────┬──────┘
            └────────────────┴─────────────────┘
                          (Phase 2+)
                                 │
                           ┌─────▼─────┐
                           │  roko-cli │
                           │ roko-serve│
                           └───────────┘
```

Rules in this graph:

1. **Only `roko-core`, `roko-spi`, `roko-bus`, `roko-hdc` are
   kernel-tier.** Nothing else imports types from one another at
   that layer — everything flows through trait interfaces.
2. **Implementations** (`roko-runtime`, `roko-fs`, `roko-defaults`,
   `roko-tools`) never import each other.
3. **Compose is above substrate**, below agent. Templates are data.
4. **Agent / gate / orchestrator** form the "cognitive" tier; they
   share only the kernel.
5. **Learning / neuro / dreams** are "reflective" tier; they
   subscribe to the Bus and read from Substrate but never reach
   down into concrete impls.
6. **CLI / serve** sit on top and compose everything.

## 4. Rules that keep the graph clean

### 4.1 Speak through traits, not types

Cross-crate `pub use Concrete` is forbidden except at the CLI tier.
Use traits everywhere else. If a concrete type is genuinely needed,
move it to `roko-core`.

### 4.2 Publish events, don't write fields

Subsystems that want to record something emit a Pulse on the Bus.
Other subsystems subscribe. This dissolves most of the observed
cross-crate writes.

### 4.3 Data is more composable than code

Templates, role prompts, tool manifests, gate thresholds, scorer
weights — all as TOML/YAML. Engines consume data; they don't embed
it. This produces cleaner deps automatically.

### 4.4 Don't pass `Engram` where a `View` suffices

Read-only consumers take `&EngramView` (a trait with just accessors),
not `&Engram`. This keeps data changes from requiring consumer
changes.

### 4.5 Feature flags have to be cheap

Each optional capability (chain, dreams, daimon) is a feature flag
on the top-level crate. Compiling without it should produce a
smaller, simpler binary — not error.

## 5. Stability tiers for public APIs

Three tiers, clearly documented per crate:

| Tier | Stability | Examples |
|---|---|---|
| **Core** | Semver-major-only breaks | `Engram`, `Substrate`, `Bus`, `Scorer` |
| **Extended** | Minor-version breaks permitted with notice | `Pulse`, `GateRung`, `Calibration` |
| **Experimental** | Anything goes, gated behind `experimental` feature | `Dream`, `Chain`, `Daimon` |

A third-party plugin depends only on Core types. An application
can depend on Extended. Experimental is for internal use only until
it graduates.

## 6. Conventions that compound

Small conventions that pay big dividends in modularity:

### 6.1 One trait, one method per file

At the kernel tier. Makes the call graph legible.

### 6.2 Traits name the *what*, types name the *which*

`Substrate` is a what; `InMemorySubstrate`, `FileSubstrate`,
`RedisSubstrate` are whiches. The trait goes in core; the whiches
go in impl crates. No exceptions in kernel tier.

### 6.3 Errors are per-crate and narrowly typed

No `anyhow` at crate boundaries; each crate exposes a typed
`Error` enum. This is load-bearing for the SPI — plugins need to
handle specific errors, not untyped strings.

### 6.4 Async boundaries are documented

Every `async fn` documents cancellation behavior and whether it
holds resources. Without this, replacing an implementation is a
research project. This is a README-level discipline.

## 7. Migration plan

The graph above is where we want to land. A reasonable path:

### Phase 1 (weeks 1–2): `roko-bus` extraction

- New crate.
- Move bus-related types from `roko-runtime`.
- Add `Bus` trait to `roko-core`.
- Everything else imports from the new crate. No feature changes.

### Phase 2 (weeks 2–3): `roko-spi` scaffold

- New crate.
- Define Extension, Capability, Permissions traits.
- No functional change; just establishes the extension surface.

### Phase 3 (weeks 3–4): `roko-hdc` extraction

- Move HDC code out of `roko-primitives`.
- `roko-primitives` continues to exist with the remaining tier
  routing until that too finds a home.

### Phase 4 (month 2): split `roko-std` and `roko-compose`

- `roko-defaults` / `roko-tools`.
- `roko-compose-core` / `roko-templates`.
- Import-fix churn but no behavior changes.

### Phase 5 (month 2+): enforce rules

- Add a CI job that checks cross-crate imports stay within the
  dependency graph.
- Deprecate any direct type imports that should be through traits.

## 8. What this unlocks

Once the graph is clean:

- **Substrate swap**: replacing `roko-fs` with a Postgres backend
  becomes a self-contained effort.
- **Bus swap**: adding a NATS/Redis bus doesn't require touching
  agent code.
- **Template distribution**: roles ship as separately-versioned
  packages.
- **Plugin safety**: the SPI's narrow surface becomes enforceable.
- **Parallelism in development**: two subsystems can evolve in
  parallel without merge pain.

## 9. Non-goals

This refactor isn't about making Roko abstract for its own sake.
Every new crate and trait boundary must justify its existence with
an actual use case within the next 6 months. Over-abstraction is a
worse failure mode than under-abstraction at this stage.

Specifically, we are NOT proposing:

- A "pluggable everything" nightmare where every type is behind a
  trait object.
- A microservices split of the binary.
- Abstractions that only pay off in fantasy use cases.

Modularity has to earn its keep.

## 10. How to know this is working

Three signals:

1. **Time to add a new backend** (Substrate or Bus impl) drops
   below a day.
2. **Time to add a new role/template** drops below an hour.
3. **Cross-crate cyclic references** in `cargo-depgraph` stays at
   zero.

When all three hit, modularity has paid off and we stop.

## 11. CI enforcement of the dep graph

The rules in §4 are worth nothing without automation. A small
workspace-level check:

```bash
# scripts/check-deps.sh (new)
# Fails CI if any forbidden dependency appears in Cargo.toml files.

set -euo pipefail

# Rule: roko-conductor must not depend on roko-learn.
if grep -q "roko-learn" crates/roko-conductor/Cargo.toml; then
  echo "FAIL: roko-conductor depends on roko-learn (see docs/00-architecture/23)"
  exit 1
fi

# Rule: kernel crates must not depend on impl crates.
for kernel in roko-core roko-spi roko-bus roko-hdc; do
  for impl in roko-runtime roko-fs roko-defaults roko-tools; do
    if grep -q "$impl" "crates/$kernel/Cargo.toml" 2>/dev/null; then
      echo "FAIL: kernel crate $kernel depends on impl $impl"
      exit 1
    fi
  done
done

# Rule: at most one roko-foo line in Cargo.toml per impl crate
# (prevents accidental double-import). Trim whitespace+comments.
# ...
echo "OK"
```

Run from `.github/workflows/ci.yml` or equivalent. Fail fast so
cross-layer imports get caught at PR time, not after landing.

## 12. Migration safety net

Each crate extraction/split is a mechanical refactor. To keep it safe:

1. **Start with a workspace-local symlink.** Temporarily add the
   new crate path while keeping the old one. Both compile.
2. **Move one type at a time.** Move the canonical definition to
   the new crate; leave a `pub use` shim in the old crate.
3. **Update consumers.** Change one crate at a time from old path
   to new path.
4. **Remove the shim.** Once no consumer uses the old path, delete it.
5. **Compile the workspace.** Any failure is a missed consumer.

This is the pattern the Rust std lib uses for moving items between
modules. It works for crate boundaries too, and avoids big-bang
refactors that get stuck.

## 13. Dep graph visualization

A one-line command worth adding to `cargo roko` (see 22 §8):

```bash
cargo roko depgraph --layer --format svg > deps.svg
```

Generates the §3 graph from actual Cargo.toml parsing. Color-coded
by layer tier. Forbidden arrows drawn red. Shipped as an image the
team can inspect during PR review.

## 14. Cross-references

- Bus extraction from `roko-runtime` to `roko-bus` is
  `03-bus-as-first-class.md` §6 and
  `06-refactoring-plan.md` Phase B.
- HDC extraction into `roko-hdc` is
  `11-hyperdimensional-substrate.md` §11.1.
- SPI extraction into `roko-spi` is
  `17-plugin-extension-architecture.md` §3.
- Rewrite candidates (per-crate) are in
  `21-from-scratch-redesigns.md`.
- Testing strategy for the new boundaries rolls into
  `33-observability-telemetry.md` (test-as-observation).
