# roko-plugin — Extension System

> Status-quo audit · verified 2026-07-07 · re-verified 2026-07-08 @ HEAD 5852c93c05 (all file:line refs re-checked; only trivial drift found — see notes below) · sources: 4 crate files (roko-plugin src+tests+Cargo.toml — lib.rs 1,080 + manifest.rs 583 + tests/sdk.rs 120 = 1,783 lines src/test; Cargo.toml separate), roko-core extension.rs + config/agent.rs, 11 roko-cli files (main.rs, commands/config_cmd.rs, runner/{extension_loader,types,event_loop}.rs, orchestrate.rs, event_sources.rs, scaffold.rs, commands/{plan,do_cmd}.rs, serve_runtime.rs), 5 roko-serve files, roko-fs layout.rs, 6 design docs (docs/v2/12-EXTENSIONS.md, docs/v1/18-tools/{14,15-16,16}*.md, docs/v2-depth/08-extension-system/INDEX.md, docs/v2-depth/13-builtin-catalog/03-plugin-spi-as-extension.md), `.roko/GAPS.md`, `.git/logs/HEAD` — ~30 total. All file:line refs checked against source this date.

Status vocab: ✅ wired · 🔌 built-not-wired · 🟡 partial · ❌ not implemented · 🕰️ old paradigm.

> **2026-07-08 DEEP SECOND PASS (@ HEAD 5852c93c05).** Added four exhaustive sections below the current-state table: (1) *The 17 hooks — enumeration + runtime invocation* (each hook → layer → when-fired → is-it-invoked → caller file:line); (2) *Extension chain execution trace* (both runners, ordered); (3) *The three plugin notions, side by side*; (4) *Install → load split-brain path diagram*. **Headline finding: of the 17 trait hooks, exactly 6 have a production (non-test) call site — `on_init`, `on_shutdown`, `pre_inference`, `post_inference`, `on_gate`, `on_error`.** The other 11 are unreachable at runtime. Worse subtlety confirmed this pass: the *only* concrete non-test `Extension` impl is `PluginExtension`, hardcoded to `ExtensionLayer::Cognition` (extension_loader.rs:161). The chain's `run_on_error` filters to `Recovery` (extension.rs:582), so a loaded plugin's `on_error` can **never** fire even though the hook has a runtime caller — leaving **5** hooks reachable for real plugins, all log-only. `run_pre_action`/`run_on_tool_call` exist (extension.rs:539/557) but have zero production callers, so the Action-layer veto is dead regardless of layer.
>
> **2026-07-08 re-verification note.** Every load-bearing claim below re-confirmed at HEAD 5852c93c05. Corrections vs the 2026-07-07 pass (all minor, none change status): the CLI handler is `cmd_plugin` at **config_cmd.rs:1055** (dispatches `PluginCmd::{List,Install,Remove,Audit}`); List body :1065-1130, Install :1131-1189 (copy-to-`.roko/plugins/<name>/` at :1153-1156), Remove :1191-1201, Audit (display-only) :1203-1278 (the "security audit" is a `println!` of `tool.command`+`timeout_ms`, :1244-1250). `run_pre_action`/`run_on_tool_call` (extension.rs:539/557) have **only** test call sites (:685/:697) — Action-layer veto still dead. Trait exposes exactly **17** hooks (extension.rs:290-426); v2 spec (12-EXTENSIONS.md:188) mandates **22** + `FilterDecision`/`BudgetAction`/`CamelTag`/`TaintLevel` — none exist in code. Zero real `plugin.toml` in-repo (grep `^[plugin]`/`^[[prompts]]` across crates/*.toml → 0 files). GAPS.md still has **0** plugin/extension entries. Line drift ≤10 lines elsewhere; blocks unchanged.

## Summary

There are **three disjoint "plugin" notions** in the codebase, and none of them can make an installed plugin actually *do* anything today:

1. **`crates/roko-plugin/` — the Rust SDK** (1,080-line lib.rs + 583-line manifest.rs; that's the whole crate). Half of it is live: the `EventSource` trait (lib.rs:135) with two concrete impls — `CronEventSource` (lib.rs:148) and `FileWatchEventSource` (lib.rs:82, with 500ms debounce, glob include/exclude, default `.git`/swap excludes) — which roko-serve spawns as daemon background tasks (`roko-serve/src/scheduler.rs:36`, `fswatcher.rs:21`, `lib.rs:2773-2795`) and `roko event-sources` inspects (`roko-cli/src/event_sources.rs:41`). **Crucially those sources are configured from `roko.toml` `[scheduler]`/`[watcher]` sections, never from plugin manifests.** The other half is dead: `FeedbackCollector`/`FeedbackSignal` (lib.rs:604/53) have **zero implementors or pollers outside roko-plugin itself** (workspace grep: only lib.rs + tests/sdk.rs), and the code-side `PluginManifest`/`PluginBuilder` (lib.rs:619/631) have no production consumer — a parity-era fluent API (`parity(2B.09/2B.10)`, 2026-04-08 reflog) nothing calls.

2. **TOML manifest plugins** (`manifest.rs`). `plugin.toml` declares Tier-1 prompts, Tier-2 tool profiles, Tier-3 declarative shell tools, triggers (cron/file_watch/webhook), and dependencies (manifest.rs:62-202), with validation (unique names, non-empty commands, manifest.rs:234-282) and directory discovery (manifest.rs:297). The CLI surface is `roko config plugins list/install/remove/audit` (nested under config — `main.rs:1184-1217,1866-1871`; the v1-doc target `roko plugin …` top-level group with `search/enable/disable/info` does not exist). Install copies files to `.roko/plugins/<name>/` (config_cmd.rs:1152-1161). `audit` is display-only: it prints tier counts, tool commands, triggers, deps (config_cmd.rs:1218-1275) — no policy, permission, checksum, or sandbox checks.

3. **The `Extension` trait in roko-core** (extension.rs:269) — 8 layers matching the v2 spec exactly, 17 of the spec's 22 hooks, 3 decision enums, and an `ExtensionChain` (extension.rs:437). This part is genuinely wired: both runner paths load the chain at startup and invoke `init_all`/`pre_inference`/`post_inference`/`on_gate`/`on_error`/`shutdown_all` per task (orchestrate.rs:8507,16484,17081,17935,17471,5929; runner-v2 event_loop.rs:696,5232,5268,5293,5327,5336; wired via `audit(A1)`+`audit(C2+E1)` commits, 2026-04-28).

The three meet in `roko-cli/src/runner/extension_loader.rs`, and that's where the story collapses: discovered `plugin.toml`s are wrapped as `PluginExtension` whose every hook is a **no-op debug log** (extension_loader.rs:24-100 — "this wrapper is the right place to enforce them **in the future**"). Prompts are never fed to roko-compose, declarative tools are never registered in any tool registry, profiles are never enforced by the safety layer, and manifest triggers never become EventSources. Worse, there is an **install/load directory mismatch**: install writes to `.roko/plugins/<name>/`, but the runtime loader scans only `.roko/extensions/` and `<workdir>/plugins/` (extension_loader.rs:123, layout.rs:190) — so a plugin installed via the CLI is listed by `plugins list` but **never loaded at run time**. And the repo contains **zero real `plugin.toml` files** (grep `[plugin]` outside target/: only inline test fixtures in manifest.rs and extension_loader.rs), so nothing has ever exercised this path outside unit tests. `.roko/GAPS.md` contains no plugin/extension entries at all.

Bottom line: the extension *chain plumbing* is v2-shaped and live; the plugin *content pipeline* (tiers 1–3) is a façade; tiers 4–5 (native ABI, WASM), CaMeL IFC, registry, and audit-as-policy exist only on paper.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| `EventSource` trait (object-safe, cancel-aware) | v1 14-plugin-sdk.md:11-13 ("narrow SDK for event sources") | `roko-plugin/src/lib.rs:135-144` | ✅ | Consumed by roko-serve lib.rs:126, scheduler.rs:8, fswatcher.rs:7 |
| `CronEventSource` (6-field cron, next-fire calc) | 15-16-agent-templates.md triggers `scheduler.cron` | lib.rs:148-332 | ✅ | Spawned in serve daemon: scheduler.rs:36, lib.rs:2776-2779; `roko event-sources` lists schedules (event_sources.rs:41) |
| `FileWatchEventSource` (notify, debounce, globs) | v2/13-TRIGGERS.md family | lib.rs:82-127,334-597 | ✅ | fswatcher.rs:21, lib.rs:2780-2788; from `roko.toml [watcher]` only |
| Webhook event source | manifest.rs `TriggerDef::Webhook` (:182-191); EventSourceKind::Webhook (lib.rs:71) | — | ❌ | No impl anywhere; serve's webhooks are separate axum routes; only a string in scaffold test (scaffold.rs:889) |
| `FeedbackCollector` / `FeedbackSignal` | Crate description ("event sources and feedback collectors", Cargo.toml:8) | lib.rs:604-616,53-64 | 🔌 dead | Zero implementors/pollers outside roko-plugin (workspace grep: 2 files, both in-crate) |
| Code-side `PluginManifest` + `PluginBuilder` | parity(2B.09/2B.10) reflog 2026-04-08 | lib.rs:619-676 | 🕰️ orphan | No production call sites; unrelated to TOML `PluginManifestFile` — two manifest types, unconnected |
| `plugin.toml` schema (T1 prompts / T2 profiles / T3 tools / triggers / deps) | v1 14-plugin-sdk.md tiers 1-3; v2/12-EXTENSIONS.md §3 | manifest.rs:62-202 + validation :234-282 | 🔌 | Parsed + validated + 11 unit tests; content has zero runtime effect |
| `discover_plugins` (dir + subdir scan) | v1 16-plugin-loading.md "discovery-first" | manifest.rs:297-342 | ✅ (mechanism) | Called from config_cmd.rs:1071,1209 and extension_loader.rs:130 |
| `roko config plugins list` | v1 16: `roko plugin list` | config_cmd.rs:1065-1130 | 🟡 | Works; scans `plugins/` + `.roko/plugins/`; nested under `config`, not top-level |
| `roko config plugins install <path>` | v1 16: `install <id>` (registry) | config_cmd.rs:1131-1190 | 🟡 | Local path only; copies to `.roko/plugins/<name>/`; help text says "or registry" (main.rs:1195) — false |
| `roko config plugins remove` | v1 16: `uninstall` | config_cmd.rs:1191-1202 | ✅ | `rm -rf .roko/plugins/<name>` (commit "plugin remove" 2026-04-20) |
| `roko config plugins audit` | v1 16: "audit reports permissions, sandbox requirements, ABI version, policy conflicts" | config_cmd.rs:1203-1278 | 🟡 display-only | Prints T1/T2/T3 counts, tool cmd+timeout, triggers, deps; **no** permissions/sandbox/policy model exists to check |
| `search`/`enable`/`disable`/`info` subcommands | v1 16-plugin-loading.md:64-73 (8 commands) | — | ❌ | Only 4 of 8 exist (main.rs:1188-1217) |
| `Extension` trait, 8 layers | v2/12-EXTENSIONS.md §4 (same 8 names) | roko-core/src/extension.rs:168-185,269-432 | ✅ | Layer enum matches spec 1:1 |
| 22 hooks | v2/12 §5 | 17 hooks in extension.rs | 🟡 | Missing: `on_budget_exceeded`, `on_tick_start/end`, `on_slot_assigned/completed`; `on_filter(Vec<Observation>)` ≠ spec `filter_input→FilterDecision` |
| Decision enums (6 in spec) | v2/12 §6 | ActionDecision/ToolDecision/RecoveryAction (:189-220) + Adjustment struct (:224) | 🟡 | No `FilterDecision`, no `BudgetAction`; variants drift (code `Rewrite(String)`/`Fallback(String)` vs spec `Substitute(ToolCall)`/`Escalate`) |
| `ExtensionChain` (sort, init/shutdown, run_* helpers) | v2/12 §7 | extension.rs:437-596 | ✅ | 10 in-crate tests + phase0_wiring.rs:181-186 |
| Chain invoked at runtime (cognition + recovery) | v2/12 §15 signal flow | orchestrate.rs:8507,16484,17081,17935,17471,5929; event_loop.rs:696,5217-5336 | ✅ | Both legacy PlanRunner and runner-v2; loaded via `RunConfig::from_roko_config` (runner/types.rs:1391-1398) |
| Action-layer interception (`pre_action`, `on_tool_call`) | v2/12 §6 (safety veto, tool substitute) | `run_pre_action`/`run_on_tool_call` exist (extension.rs:539,557) | 🔌 | **No production call sites** (grep: only extension.rs tests) — tool blocking/rewrite dead |
| Memory/Perception/Social/Meta hook dispatch | v2/12 §5 | trait methods exist | 🔌 | ExtensionChain has no run helpers for on_observe/on_filter/on_retrieve/on_store/post_action/on_message_*/on_reflect/on_cost_update (9 hooks unreachable) |
| `PluginExtension` (manifest → Extension) | v2-depth/13/03 §8 pipeline step 6 "Register" | extension_loader.rs:24-100 | 🕰️ façade | All hooks log-only; layer hardcoded `Cognition`, `optional: true` (:161-162); prompt/tool **counts** logged, content dropped |
| Runtime extension loading + allow-list | v2/12 §17 `extensions = […]` | extension_loader.rs:117-200; `[agent].extensions` (roko-core config/agent.rs:87) | 🟡 | Scans `.roko/extensions/` + `<workdir>/plugins/` — **not** `.roko/plugins/` where install writes; empty list = load all; no per-extension config tables |
| Extension manifest w/ layer/tier/tags | v2/12 §3 `ExtensionManifest` | — | ❌ | `PluginMeta` (manifest.rs:84) has name/version/description/author/license only |
| Dependency topo-sort + cycle detection | v2/12 §11 | `depends_on` copied into meta (extension_loader.rs:163-168) | ❌ | Chain sorts by layer only (`sort_by_layer`, extension.rs:455); deps never resolved, no cycle check |
| Fault isolation + 5-failure circuit breaker | v2/12 §8 | call sites warn-and-continue (orchestrate.rs:16484-16487 etc.) | 🟡 | Errors logged, run continues; no failure counting, no auto-disable; `optional` flag unused at runtime |
| Hook timeout (5s default, configurable) | v2/12 §9 | — | ❌ | No timeout wrapping anywhere |
| CaMeL IFC (CamelTag, taint, provenance, no-laundering) | v2/12 §2, EX-19–22 | — | ❌ | No CamelTag/TaintLevel/provenance types in workspace |
| 9 built-in extensions (git, compiler, safety, …) | v2/12 §12 | — | ❌ | Zero `impl Extension` outside tests + PluginExtension; equivalents live as gates/tools, not Extensions |
| Tier 4 native ABI (`roko-extension-abi`, cdylib) | v1 14 §Tier4; v2-depth/13/03 §6 | — | ❌ | Workspace denies `unsafe_code` (root Cargo.toml:188); in-tree only, by design for now |
| Tier 5 WASM sandbox (fuel, host imports) | v1 14 §Tier5; v2/12 EX-23 | — | ❌ | No wasmtime/wasm dep anywhere |
| Plugin registry (fetch, SHA-256, marketplace) | v2/12 §10 registry flow | — | ❌ | No PluginRegistry type; no remote fetch; no checksum |
| `GET /api/extensions` status route | v2/12 §18 crate mapping (roko-serve) | — | ❌ | No plugin/extension routes in roko-serve/src/routes/ (grep: only axum `req.extensions()` in middleware.rs) |
| Plugin health Lens (metrics, auto-disable) | v2-depth/13/03 §9 | — | ❌ | — |
| Agent templates (18 defs) | v1 15-16-agent-templates.md | roko-serve/src/templates.rs: `AgentTemplate` + `TemplateRegistry.scan()` of `.roko/templates/*.toml` (:331) + 6 builtins (:688-703) | 🟡 | Builtins: pr-review, code-implementer, auto-plan, gate-fixer, doc-lifecycle, slack-notify (test asserts 6, :963-964). 12 of 18 spec templates absent; no `.roko/templates/` dir in repo |
| `roko new event-source` scaffold | SDK DX | scaffold.rs:693-750 | 🕰️ | Generates a freestanding poll struct that does **not** implement `roko_plugin::EventSource` |
| Tests | — | lib.rs 12 unit (1 `#[ignore]` flaky fswatch, :808) + manifest.rs 11 + tests/sdk.rs 4 + extension.rs 10 + extension_loader.rs 4 | 🟡 | Good coverage of parsing/chain mechanics; **zero** tests that a plugin changes agent behavior (nothing to test — it can't) |

## The 17 hooks — enumeration + runtime invocation

The `Extension` trait (extension.rs:269-432) declares 17 hooks over 8 layers (v2 spec §5 mandates 22 — the 5 absent ones are listed at the bottom). "Chain helper" = the `ExtensionChain::run_*` dispatcher that filters by layer and calls the hook. "Invoked at runtime?" = whether any **production** (non-test) code path reaches the hook. All chain-helper call sites live in `orchestrate.rs` (legacy `PlanRunner`) and `runner/event_loop.rs` (runner-v2).

| # | Hook | Layer | Signature site | When it *should* fire | Chain helper | Invoked at runtime? | Production caller(s) |
|---|---|---|---|---|---|---|---|
| 1 | `on_init` | Foundation | extension.rs:290 | Agent/run startup | `init_all` (extension.rs:470, calls **all** layers) | ✅ **yes** | orchestrate.rs:8507 · event_loop.rs:698 |
| 2 | `on_shutdown` | Foundation | extension.rs:295 | Run teardown (reverse order) | `shutdown_all` (extension.rs:481) | ✅ **yes** | orchestrate.rs:5929 · event_loop.rs:5336 |
| 3 | `on_observe` | Perception | extension.rs:302 | New observation arrives | — none — | ❌ **no** | — (no helper, no caller) |
| 4 | `on_filter` | Perception | extension.rs:310 | Filter observations pre-cognition | — none — | ❌ **no** | — |
| 5 | `on_retrieve` | Memory | extension.rs:320 | Knowledge-store read | — none — | ❌ **no** | — |
| 6 | `on_store` | Memory | extension.rs:328 | Knowledge-store write | — none — | ❌ **no** | — |
| 7 | `pre_inference` | Cognition | extension.rs:338 | Before LLM dispatch | `run_pre_inference` (extension.rs:494, Cognition filter) | ✅ **yes** | orchestrate.rs:16484 · event_loop.rs:5232 |
| 8 | `post_inference` | Cognition | extension.rs:346 | After LLM response | `run_post_inference` (extension.rs:509, Cognition filter) | ✅ **yes** | orchestrate.rs:17081 · event_loop.rs:5268 |
| 9 | `on_gate` | Cognition | extension.rs:354 | Per gate verdict | `run_on_gate` (extension.rs:524, Cognition filter) | ✅ **yes** | orchestrate.rs:17935 · event_loop.rs:5293 |
| 10 | `pre_action` | Action | extension.rs:364 | Before an action (veto/rewrite) | `run_pre_action` (extension.rs:539, Action filter) | 🔌 **helper exists, no caller** | test-only: extension.rs:697 |
| 11 | `post_action` | Action | extension.rs:372 | After an action completes | — none — | ❌ **no** | — |
| 12 | `on_tool_call` | Action | extension.rs:380 | Tool invocation (allow/deny/rewrite) | `run_on_tool_call` (extension.rs:557, Action filter) | 🔌 **helper exists, no caller** | test-only: extension.rs:685 |
| 13 | `on_message_send` | Social | extension.rs:390 | Outbound inter-agent msg | — none — | ❌ **no** | — |
| 14 | `on_message_receive` | Social | extension.rs:398 | Inbound inter-agent msg | — none — | ❌ **no** | — |
| 15 | `on_reflect` | Meta | extension.rs:408 | Reflection phase (returns `Vec<Adjustment>`) | — none — | ❌ **no** | — |
| 16 | `on_cost_update` | Meta | extension.rs:416 | Cost/token update | — none — | ❌ **no** | — |
| 17 | `on_error` | Recovery | extension.rs:426 | On error (returns `RecoveryAction`) | `run_on_error` (extension.rs:575, Recovery filter) | ✅ **yes** | orchestrate.rs:17471 · event_loop.rs:5327 |

**Tally: 6 of 17 invoked at runtime** (#1,2,7,8,9,17). 2 have a chain helper but only test callers (#10,12 — Action veto dead). 9 have neither a helper nor a caller (#3-6,11,13-16 — the entire Perception/Memory/Social/Meta surface plus `post_action` is unreachable; the doc's original "9 hooks unreachable" line refers to exactly these).

**Layer-filter subtlety (confirmed this pass).** The chain helpers `run_pre_inference/run_post_inference/run_on_gate` iterate only `layer() == Cognition` extensions (extension.rs:501,516,531); `run_on_error` only `layer() == Recovery` (extension.rs:582). `init_all`/`shutdown_all` iterate **all** extensions. The single concrete non-test impl, `PluginExtension`, is hardcoded `layer = Cognition` (extension_loader.rs:161). Therefore, for a *real installed plugin*: `on_init`+`on_shutdown` fire (all-layer), `pre_inference`+`post_inference`+`on_gate` fire (Cognition match), but **`on_error` never fires** (plugin is Cognition, filter wants Recovery). Net: **5 hooks reachable for actual plugins, all log-only no-ops** (extension_loader.rs:46-99 override exactly these 5; the trait defaults cover the rest). No plugin can ever supply a `Recovery`-layer extension because the layer is not read from the manifest.

### Hooks in the v2 spec but absent from code (17 → 22)

Per `docs/v2/12-EXTENSIONS.md:188-230`: **`filter_input → FilterDecision`** (code has `on_filter(&mut Vec<Observation>)`, wrong shape + no decision type), **`on_budget_exceeded → BudgetAction`**, **`on_tick_start`**, **`on_tick_end`**, **`on_slot_assigned`/`on_slot_completed`**. None exist. Spec hooks also take an `AgentContext` param (§16) that the code omits entirely.

## Extension chain execution trace (both runners)

The chain is constructed once per run from `RunConfig::from_roko_config` (runner/types.rs:1391-1398 → `load_extensions`), stored as `Option<Arc<Mutex<ExtensionChain>>>` (types.rs:1308). The legacy `PlanRunner` owns a bare `ExtensionChain` field (orchestrate.rs:2797) built at `4719/4949/5177`.

**Legacy `PlanRunner` (orchestrate.rs), per run:**
```
run start   → extension_chain.init_all()            :8507   (all extensions, on_init)
per task    → run_pre_inference(&mut req)           :16484  (Cognition, before dispatch)
            → run_post_inference(&mut resp)          :17081  (Cognition, after dispatch)
per gate    → run_on_gate(&mut gate_event)           :17935  (Cognition, each verdict)
on error    → run_on_error(&error_event)             :17471  (Recovery; result IGNORED — see below)
run end     → extension_chain.shutdown_all()          :5929   (all extensions, reverse, on_shutdown)
```

**Runner-v2 (runner/event_loop.rs), per run — same order, `try_lock` guarded:**
```
run start   → chain.init_all()                        :698
per task    → fire_pre_inference_hook → run_pre_inference   :5232
            → fire_post_inference_hook → run_post_inference :5268
per gate    → fire_on_gate_hook → run_on_gate (loop verdicts):5293
on error    → fire_on_error_hook → run_on_error        :5327  (result IGNORED)
run end     → shutdown_subsystems → chain.shutdown_all():5336
```
Each event_loop `fire_*` helper `try_lock`s the mutex and **skips the hook on contention** (event_loop.rs:5221-5223 etc.) — hooks are best-effort, never block the loop, and a lock-contended `pre_inference` is silently dropped (logged `warn!`).

**Both runners discard the veto return value.** `run_on_error` returns a `RecoveryAction`, but both call sites only check `.is_ok()` / log on error and never branch on `Retry`/`Skip`/`Fallback` (orchestrate.rs:17471, event_loop.rs:5327). So even the one Recovery hook that *is* invoked cannot change control flow — its decision is inert. Combined with the dead Action helpers, **no extension can influence runtime behaviour today**; the chain is observe-only (log + TUI `extension_hook` breadcrumb, event_loop.rs:5236).

## The three plugin notions, side by side

| | (a) `roko-plugin` Rust SDK | (b) TOML plugin manifest | (c) `roko-core` `Extension` chain |
|---|---|---|---|
| **Where** | `roko-plugin/src/lib.rs` (1,080 L) | `roko-plugin/src/manifest.rs` (583 L) | `roko-core/src/extension.rs` (760 L) |
| **Core type** | `EventSource` trait (lib.rs:135); `FeedbackCollector` (lib.rs:604); orphan `PluginManifest`/`PluginBuilder` (lib.rs:619) | `PluginManifestFile` (manifest.rs:62) | `Extension` trait + `ExtensionChain` (extension.rs:269/437) |
| **What it models** | Runtime *event producers* (cron/filewatch) + feedback intake | Declarative *content*: T1 prompts, T2 profiles, T3 tools, triggers, deps | 17 lifecycle *hooks* over 8 layers |
| **Author writes** | Rust `impl EventSource` | `plugin.toml` | Rust `impl Extension` |
| **Wired at runtime?** | ✅ `CronEventSource`/`FileWatchEventSource` spawned by roko-serve daemon — but **only from `roko.toml [scheduler]`/`[watcher]`, never from a plugin** | 🟡 parsed/validated/listed; **content has zero effect** | ✅ chain fires 6 hooks per run in both runners |
| **Dead parts** | `FeedbackCollector` (no poller), `PluginManifest`/`PluginBuilder` (no consumer), `EventSourceKind::Webhook` (no impl) | prompts→compose, tools→registry, profiles→safety, triggers→EventSource: **none connected** | 11 of 17 hooks unreachable; Action veto dead |
| **Bridge to the others** | none — SDK `PluginManifest` ≠ TOML `PluginManifestFile`; a TOML plugin cannot yield an `EventSource` | → (c) only, via `PluginExtension` wrapper (extension_loader.rs:24) that drops all content | receives (b) as a log-only `PluginExtension`; layer hardcoded Cognition |

The crate holds **two unrelated `*Manifest` types** (SDK `PluginManifest` at lib.rs:619 vs TOML `PluginManifestFile` at manifest.rs:62) with no converter between them — the clearest symptom of the split-brain.

## Install → load split-brain (path diagram)

```
  AUTHOR                     roko config plugins install <src>
     │                                 │  (config_cmd.rs:1131-1189)
  plugin.toml ────────────────────────┤  validates via load_manifest
                                       ▼
                        .roko/plugins/<name>/plugin.toml     ◄── INSTALL TARGET
                                       │                          (config_cmd.rs:1152-1156)
              ┌────────────────────────┴───────────────────────┐
              │                                                 │
   roko config plugins list / audit                   runner: load_extensions()
   (config_cmd.rs:1066-1067, 1204-1205)               (extension_loader.rs:122-123
   scans:  <workdir>/plugins/                          via RunConfig::from_roko_config,
           .roko/plugins/   ✔ FINDS IT                 types.rs:1391-1398)
                                                        scans: layout.extensions_dir()
                                                               = .roko/extensions/   ✘
                                                               <workdir>/plugins/     ✘
                                                        (layout.rs:190-191)
                                                                 │
                                                                 ▼
                                              .roko/plugins/ is NEVER scanned by the loader
                                              ⇒ installed plugin is LISTED but NEVER LOADED
```

**Mismatch, precisely:** install writes `.roko/plugins/<name>/` (config_cmd.rs:1153-1156). `list`/`audit` scan `<workdir>/plugins/` **and** `.roko/plugins/` (config_cmd.rs:1066-1067, 1204-1205) → so they see it. The runtime loader scans `layout.extensions_dir()` (= `.roko/extensions/`, layout.rs:190) **and** `<workdir>/plugins/` (extension_loader.rs:123) → the intersection with the install dir is **empty**. Only a plugin *hand-placed* in `<workdir>/plugins/` is both listed and loaded. Fix is one line either way: install into `.roko/extensions/<name>/`, or add `.roko/plugins/` to the loader's `scan_dirs`. There are **zero** real `plugin.toml` files in-repo (grep `^\[plugin\]` outside `target/`: only test fixtures in manifest.rs and extension_loader.rs), so this path has never run outside unit tests.

## Deep-pass verification checklist

- [x] All 17 hooks enumerated with signature file:line (extension.rs:290-426) — verified
- [x] Runtime callers grep'd across `crates/` (non-test): 6 hooks reachable, exact sites confirmed in orchestrate.rs + event_loop.rs
- [x] `run_pre_action`/`run_on_tool_call` have **only** test callers (extension.rs:697/685) — Action veto dead
- [x] `PluginExtension` layer hardcoded `Cognition` (extension_loader.rs:161) ⇒ its `on_error` never fires (Recovery filter, extension.rs:582)
- [x] Both runners `.is_ok()`-check but never branch on `RecoveryAction` ⇒ veto inert (orchestrate.rs:17471, event_loop.rs:5327)
- [x] Install/load dir mismatch traced to file:line (install .roko/plugins vs loader .roko/extensions + workdir/plugins)
- [x] Manifest schema fully documented (manifest.rs:62-202): plugin/prompts/profiles/tools/triggers/dependencies
- [x] v2 spec delta: 17 vs 22 hooks; 5 missing named (filter_input, on_budget_exceeded, on_tick_start/end, on_slot_assigned/completed)
- [x] Zero real `plugin.toml` in-repo; `.roko/GAPS.md` has zero plugin entries (unchanged)

## V2-aligned

- **Layer model**: `ExtensionLayer` in code is byte-for-byte the v2 spec's 8 layers (extension.rs:168-185 vs 12-EXTENSIONS.md §4).
- **Typed async hooks**: the C2+E1 audit rewrite gave hooks typed parameter structs (`InferenceRequest`, `InferenceResponse`, `GateEvent`, `ToolCallEvent`, `CostUpdate`, …, extension.rs:29-161) closely mirroring spec §5 — better than the spec's older `serde_json::Value` drafts.
- **Chain-in-the-loop**: init → pre/post-inference → on_gate → on_error → shutdown genuinely fire around every LLM dispatch in *both* runners; reverse-order shutdown matches spec §14 (`shutdown_all` iterates `.rev()`, extension.rs:485).
- **Tier 1–3 vocabulary**: manifest schema names its sections exactly as the 5-tier SPI's tiers 1–3 (manifest.rs:1-51 doc comment; audit prints `T1:prompts/T2:profiles/T3:tools`, config_cmd.rs:1226-1234).
- **Discovery-first stance**: loading is manifest/directory-driven, `roko.toml` holds only an allow-list — matches v1 16-plugin-loading.md's "manifests are the source of truth" (though only two roots, not the five tier-specific roots).

## Old paradigm & tech debt

- 🕰️ **Two unrelated `*Manifest` types in one crate**: code-side `PluginManifest` (boxed trait objects, lib.rs:619) vs TOML `PluginManifestFile` (manifest.rs:62). The former is a mori-parity orphan (`parity(2B.09)`) with no consumer; nothing converts one to the other. A loaded TOML plugin cannot contribute an `EventSource`.
- 🕰️ **`PluginExtension` façade**: extension_loader.rs:24-100 admits it — hooks "log lifecycle events and can be extended to execute declarative hooks … in the future". It carries `prompt_count`/`tool_count` integers instead of the actual prompts/tools.
- **Install/load split-brain**: `install` → `.roko/plugins/<name>/` (config_cmd.rs:1153-1156); runtime loader → `.roko/extensions/` + `<workdir>/plugins/` (extension_loader.rs:123). `list`/`audit` scan the install dirs; the loader scans different ones. Only a plugin manually placed in `<workdir>/plugins/` is both listed *and* loaded.
- **Naming drift**: crate emits `Engram` (`SignalSender = Sender<Engram>`, lib.rs:33); roko-core aliases `Engram as Signal` (signal.rs:6). v2 vocabulary (Cell, Pulse, graduation) absent; L1/L5 "Pulse medium" distinction (spec §1) has no counterpart.
- **Misleading UX strings**: `install` help claims registry support (main.rs:1195-1197); `audit`'s "security audit" comment (config_cmd.rs:1244) is a `println!` of the command string; `EventSourceKind::Webhook` (lib.rs:71) advertises a source kind with no implementation.
- **Dead SDK half**: `FeedbackCollector` has a poll `interval()` contract but no scheduler drives it; `FeedbackOutcome::Merged/Approved/…` never enter the learning loop (roko-learn ingests episodes by other paths).
- **v1 docs already stale in the other direction**: 14-plugin-sdk.md:373 says "Today there is no shipped `roko plugin` command group" — the `config plugins` group shipped after that doc was written (2026-04-20 reflog) and the doc wasn't updated.
- **Doc topology gap**: docs/v2-depth/08-extension-system/ contains only a 17-line INDEX ("Depth docs: _None yet_") pointing at `docs/unified/08-EXTENSION-SYSTEM.md`, which doesn't exist (renamed to docs/v2/12-EXTENSIONS.md — INDEX.md:3 is a dangling link). The real depth doc lives under 13-builtin-catalog/03.
- **`.roko/GAPS.md` silence**: zero plugin/extension entries despite CLAUDE.md rule 4 — this subsystem's gaps are tracked nowhere.
- **Cargo metadata**: `[package.metadata.roko] layer = 2` (roko-plugin/Cargo.toml:27-28) — a homegrown layering marker no tooling reads (grep found no consumer).

## Not implemented

Concept-by-concept against v2 12-EXTENSIONS.md: **CaMeL IFC** (§2, EX-19–22) — nothing; **ExtensionManifest with tier/layer/tags** (§3) — nothing; **5 of 22 hooks** (§5: budget + 4 cross-cutting); **FilterDecision/BudgetAction** (§6); **hook timeout** (§9); **registry + checksum fetch** (§10); **dependency topo-sort/cycles** (§11); **all 9 built-in extensions** (§12); **Extension-as-Cell** (§13); **AgentContext param** (§16); **per-extension TOML config + `disable_extensions`** (§17); **`GET /api/extensions`** (§18); **Tier 4 ABI + Tier 5 WASM** (v1 14, v2-depth 13/03 §§6-7); **plugin health Lens + auto-disable** (13/03 §9); **permissions model** (`network`/`files_read`/`bus_*`, v1 14 §Permissions); **profile merge semantics** (16-plugin-loading.md §Profile Composition). Also: no example/real plugin exists in-repo; 12 of 18 v1 agent templates missing.

## Migration checklist

- [ ] **[P0]** Fix install/load dir mismatch — either install into `.roko/extensions/<name>/` or add `.roko/plugins/` to `load_extensions` scan_dirs (extension_loader.rs:123) — verify: `roko config plugins install <dir> && roko plan run plans/ 2>&1 | grep "loaded plugin extension"`
- [ ] **[P0]** Make Tier 1 real: feed `PluginManifestFile.prompts` into roko-compose template registry at load — verify: install a prompt plugin, run a task, grep the episode's system prompt for the template text
- [ ] **[P0]** Make Tier 3 real: register `DeclarativeTool` entries (command+timeout+env) into the runtime tool registry behind the existing tool safety layer — verify: agent turn lists the plugin tool in its tool schema; `.roko/episodes.jsonl` shows an invocation
- [ ] **[P1]** Make Tier 2 real: apply `ToolProfileBundle` allow/deny to the safety layer / role tool profiles — verify: denied tool call is blocked in a run with the profile active
- [ ] **[P1]** Wire manifest `triggers` → `CronEventSource`/`FileWatchEventSource` in serve's `start_builtin_event_sources` (roko-serve/lib.rs:2773) — verify: `roko event-sources` (or serve logs) show a plugin-declared schedule
- [ ] **[P1]** Call `run_pre_action`/`run_on_tool_call` from the dispatch/tool loop so Action-layer vetoes work (spec EX-4/EX-5) — verify: test extension returning `ToolDecision::Deny` blocks a bash call
- [ ] **[P1]** Ship at least one real `plugin.toml` in-repo (examples/ or plugins/) + an e2e test through install→load→invoke — verify: `cargo test -p roko-cli extension` + `roko config plugins audit` shows it
- [ ] **[P2]** Read `layer`/`optional` from the manifest instead of hardcoding Cognition/true (extension_loader.rs:161-162); add dependency topo-sort + cycle error (spec §11, EX-11/12) — verify: unit test with `depends_on` ordering
- [ ] **[P2]** Hook timeout (5s default, per-ext override) + consecutive-failure auto-disable (spec §§8-9, EX-8/17) — verify: unit test with a hanging hook
- [ ] **[P2]** Decide `FeedbackCollector`: wire a poll loop feeding roko-learn, or delete it and fix Cargo.toml description — verify: grep for implementors, or crate description matches reality
- [ ] **[P2]** `GET /api/extensions` in roko-serve exposing `ExtensionChain::metadata()` (spec §18) — verify: `curl :6677/api/extensions`
- [ ] **[P2]** Complete CLI: `info`, `enable`/`disable` (toggle without deleting), align help text with real capabilities; consider promoting to top-level `roko plugin` per v1 docs — verify: `roko config plugins info <name>`
- [ ] **[P3]** Add missing hooks (`on_budget_exceeded`, tick/slot events), `FilterDecision`/`BudgetAction` enums, AgentContext param (spec §§5-6,16) — verify: `cargo check` + chain unit tests
- [ ] **[P3]** CaMeL IFC tag types + propagation through decision enums (spec §2, EX-19–22) — likely lands with the security workstream (docs/v2/16-SECURITY.md)
- [ ] **[P3]** Tier 5 WASM loader (wasmtime, fuel, 6 host imports) then Tier 4 ABI decision (13/03 open question 2 leans workspace-local-only) — verify: EX-23 integration test
- [ ] **[P3]** Registry/marketplace fetch with SHA-256 (spec §10) — blocked on relay/registry infra (docs/v2/22-REGISTRIES.md)
- [ ] **[P3]** Backfill the 12 missing v1 agent templates as builtins or `.roko/templates/` files, or re-scope the doc to the 6 shipped — verify: `builtin_templates().len()` test vs doc
- [ ] **[P3]** Log this subsystem's gaps into `.roko/GAPS.md`; fix v2-depth/08 INDEX dangling link; update v1 14/16 "no shipped CLI" notes — verify: grep GAPS.md for "plugin"

## Open questions

1. **"registry cleanup"** (mentioned in the audit brief) does not appear in `.git/logs/HEAD` for roko-plugin — the nearest matches are agent-registry (AR01-04) and model-routing registry commits. Was plugin-registry code removed on another branch / squashed away, or does the phrase refer to `SubscriptionRegistry`/`RepoRegistry` cleanup? Nothing named `PluginRegistry` ever existed in-tree (workspace grep).
2. **Which loader wins?** `PluginExtension` (chain-based, roko-cli) vs the spec's Tier-1/2/3 registration into compose/config/tools are architecturally different: should manifest content be *registered into subsystems at load* (v1 16 lifecycle step 7) with the Extension wrapper only for lifecycle, or should hooks themselves inject prompts/enforce profiles per-inference? The current no-op wrapper defers this decision.
3. **Cron dialect**: manifest docs demand 6-field-with-seconds (manifest.rs:161), the `cron` 0.12 crate parses 6/7-field, but v1 template docs and lib.rs tests use 5-field expressions (`"0 9 * * MON"`, lib.rs:762) that never pass through `Schedule::from_str` in tests — is 5-field config silently broken in `[scheduler]`?
4. **Extension vs MCP**: MCP servers already deliver the "third-party tool" value proposition and are wired (CLAUDE.md). Is Tier 3 declarative-shell-tools worth finishing, or should plugin.toml `tools` compile down to an MCP config instead?
5. **`[agent].extensions` semantics**: empty = load-all (extension_loader.rs:116) — is silent load-all of anything in `<workdir>/plugins/` acceptable once plugins can execute shell commands (Tier 3)? A signed/allow-listed default seems required before P0-3 ships.
