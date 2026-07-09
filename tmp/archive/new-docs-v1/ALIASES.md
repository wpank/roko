# Aliases

> Public-facing aliases and their canonical internal terms. Use the **Canonical** column in code,
> architecture docs, and internal engineering communication. Use the **Public alias** in user-facing
> docs, CLI output, UI labels, and external material.
>
> For full definitions, see [`GLOSSARY.md`](GLOSSARY.md).

---

## Alias Table

| Public alias | Canonical internal term | Status | Notes |
|---|---|---|---|
| `AffectBias` | `Daimon` | `[built]` | Use in CLI flags, user docs, and API responses; keep `Daimon` in code and arch prose |
| `Situation` | `TypedContext` | `[planned]` | The user-facing name for structured domain situation payloads |
| `StateHub` | `StateHub` | `[built]` | `StateHub` *is* both; currently the TUI hub, target-state the Bus+Substrate projection layer |
| `Bus` | `EventBus<E>` | `[planned]` | `Bus` is the target-state abstraction; `EventBus<E>` is the live implementation today |
| `Pulse` | *(no shipped term)* | `[planned]` | Replaces `Event`, `Envelope`, `Message`, `Signal` (ephemeral); no shipped type yet |
| `Engram` | `Signal` (retired) | `[shipping]` | `Engram` is canonical; `Signal` appears in some older code paths as the struct name |
| `Agent` | `Golem` (retired) | `[shipping]` | `Agent` is canonical everywhere now |
| `Fleet` | `Clade` (retired) | `[planned]` | `Fleet` not yet shipped; `Clade` is fully retired |
| `Mesh` | `Styx` (retired) | `[planned]` | `Mesh` not yet shipped; `Styx` is fully retired |
| `Neuro` | `Grimoire` (retired) | `[built]` | `Neuro` is canonical; `Grimoire` fully retired |
| `Roko` | `Bardo` / `Mori` (retired) | `[shipping]` | Project name; predecessors fully retired |

---

## Conventions

- The **Canonical** column is the term used in:
  - Rust code (`struct`, `trait`, `fn` names)
  - Architecture documents
  - Migration logs
  - Internal engineering communication

- The **Public alias** column is the term used in:
  - `roko --help` output and CLI flags
  - API response field names (where applicable)
  - UI labels and dashboard text
  - External documentation (guides, quickstart, README)
  - Investor and sales material

- Where **Public alias = Canonical** (as with `StateHub`), the term is already user-friendly
  enough that no alias is needed.

- Retired terms are listed here for traceability only. They must not appear in new code,
  new docs, or new UI — only in `strategy/refinements/naming-history.md` and in comments
  explicitly marked `// retired:`.

---

## CLI Flag Mapping

| CLI flag / output label | Canonical type | Crate |
|---|---|---|
| `--affect-bias` / `AffectBias` in TUI | `Daimon` | `roko-daimon` |
| `situation:` in prompt context blocks | `TypedContext` | *(planned)* |
| `state-hub` in `roko dashboard` | `StateHub` | `roko-cli` |

---

## See Also

- [`GLOSSARY.md`](GLOSSARY.md) — full A-Z definitions
- [`strategy/refinements/naming-history.md`](strategy/refinements/naming-history.md) — narrative of how names changed
