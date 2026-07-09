# Documentation Conventions

> The writing rules for this documentation tree. If another document contradicts
> these conventions, fix the document.

---

## 1. One Concept Per File

The guiding rule. A file covers exactly one concept. The easiest test: a reader
implementing or debugging the concept should open this file and exactly this file.
If they would need to open this file *and three others* before they understand
anything, the granularity is wrong.

But the opposite extreme is equally wrong. A file that covers exactly one field of
an enum, or one method on a trait, is not a concept — it is a fragment. If the
canonical audience for a page is "someone curious about this one method," that
method belongs in the concept page alongside its siblings. Concept-per-file, not
fact-per-file.

When a concept genuinely has distinct sub-concepts with separable APIs, failure
modes, and implementation backends — *then* promote the file to a folder of
focused pages. Not before.

---

## 2. Frontmatter

Every substantive page opens with a status block:

```markdown
# Title

> One-sentence definition.

**Status**: Shipping | Built | Scaffold | Specified | Deferred
**Crate**: `roko-<name>` — or `—` for cross-crate / conceptual pages
**Depends on**: [Concept](relative-path), [Concept](relative-path)
**Used by**: [Concept](relative-path)
**Last reviewed**: YYYY-MM-DD
```

- `Status` is mandatory.
- `Crate` names the primary crate that ships this concept (or `—` for pure concepts).
- `Depends on` and `Used by` are directional:
  - `Depends on` = this page cannot be understood without the listed pages first.
  - `Used by` = concepts that pull on this one.
  - Keep each list to five or fewer entries. If you need more, reconsider
    granularity.
- `Last reviewed` is updated by mechanical passes; do not touch it for content edits.

---

## 3. Status Tiers

| Tier | Meaning |
|---|---|
| **Shipping** | Wired end-to-end, tested, used in the self-hosting loop. CLI-reachable. |
| **Built** | Code exists, compiles, has tests — not yet called from runtime or CLI. |
| **Scaffold** | Struct/trait stubs exist. No meaningful implementation. |
| **Specified** | Spec exists (in these docs). No code. |
| **Deferred** | Intentionally postponed (Phase 2+ / chain-dependent / research-only). |

**Writing about today vs. the target state**: describe the system as it is in body
prose. Put target-state behaviour in a `## Today vs. Planned` section at the
bottom of the page. Do not thread `[target-state]` disclaimers through every
paragraph — that is what the frontmatter status tag is for.

---

## 4. Links and Cross-References

- All links are **repo-relative**. Never use absolute paths like `/Users/will/…`.
- Every Rust type mentioned in prose is a link on first use per file (the link
  target is the page where that type is the subject).
- Every paper cited is a link — either to the page in the tree that summarizes it,
  or to a stable external archive (doi.org, arxiv.org). Prose never embeds arXiv
  IDs or DOIs as bare strings — those belong in links.
- Every code snippet carries a source-pointer HTML comment:

  ```markdown
  <!-- source: crates/roko-core/src/engram.rs -->
  ```

  so the docs ↔ code relationship is grep-able in both directions.

- Anchor text must read naturally. Never use `source`, `here`, `this`, `link`, or
  a raw URL as the clickable text. If you removed every link from the page, the
  prose should still read as complete sentences.

---

## 5. Status of Referenced Code

When quoting or describing code, be explicit about whether the quoted signature is
shipped or target-state. Use a small legend at the top of any section that mixes
the two:

```markdown
> Shipped today: `EventBus<E>`, `Substrate`, `Score`, `Engram` (as `Signal` in code)
> Target state: `Pulse`, `Bus`, `Topic`, `TopicFilter`, `Datum`, `PulseSource`
```

---

## 6. Templates

### 6.1 The Universal Reference Template

See [`README.md`](README.md) for the full template. Not every slot is filled on every
page. The minimum for a Shipping-tier concept is:

- Frontmatter
- TL;DR
- The Idea
- Specification
- Implementation
- API Reference
- Invariants
- Failure Modes
- Examples
- See Also

A Specified or Scaffold concept may fill only TL;DR, The Idea, Specification, and
Open Questions — but the other headings appear as empty sections marked
`_To be written._` so that growing the page is purely additive.

### 6.2 The Index Template

Every folder has a `README.md` that serves as its index. Structure:

```markdown
# <Folder Topic>

> One-paragraph framing: what this folder contains, what it does not contain.

## Contents

| Page | What it covers | Status |
|---|---|---|
| [Name](page.md) | … | Shipping |

## Suggested reading order

For readers new to this topic: A → B → E.
For readers implementing: A → F → H.

## See also

Three to seven links out to related folders.
```

### 6.3 The Integration Page Template

For pages in `00-architecture/analysis/integration-map.md` and any future
interaction-pair pages, use:

```markdown
# <SubsystemA> × <SubsystemB>

**Direction**: bidirectional | A→B | B→A
**Interface**: shared types, Pulses exchanged, Substrate queries

## What flows
## Invariants of the interaction
## Failure modes of the interaction
## Observed metrics
## Open questions
```

---

## 7. File Naming

- `kebab-case.md` for content files.
- `README.md` for folder indexes (not `INDEX.md` — `README.md` renders by default
  on GitHub, Docusaurus, mkdocs, and mdBook).
- Numbered prefixes only where reading order is semantically meaningful —
  e.g. the top-level sections (`00-architecture/`, later `01-orchestration/`,
  …). Inside a section, files are named by concept, not number. A reader
  navigating the core concepts needs to find `engram.md`, not `02-engram.md`.
- Sub-letter prefixes (`02b-`) are forbidden.

---

## 8. Length Discipline

- Concept pages target **8–25 KB**. If a page is much shorter, it probably does
  not have enough substance to justify a file. If a page is much longer, it
  probably covers two concepts — but check first: some concepts (the Engram, the
  cognitive loop, the five-layer taxonomy) are genuinely large and belong on one
  page.
- Analysis pages and perspective essays have no length limit; they are
  meta-documents or essays, not spec.
- The innovations folder has one file per idea; those files may be long because
  each idea is a distinct research sketch.

---

## 9. Open Questions Are First-Class

Every page ends with `## Open Questions`. Unresolved design decisions, known gaps,
and deferred sub-problems live here — not in the spec prose, which should be
declarative. When a question is resolved, promote it into real content and remove
it from Open Questions.

Unresolved questions at folder level go in the folder's `README.md` under an
`## Open Questions` section.

---

## 10. Perspective on the Reader

Write every page from the perspective of a reader with no prior context on Roko.
That does not mean every page re-introduces the entire system — it means every
term used is either

- defined on the page,
- linked on first use to the page where it is defined, or
- in the shared [GLOSSARY](GLOSSARY.md).

If a page references `ContentHash` without linking to `concepts/engram.md`, and
without defining it in-line, the page is broken regardless of how technically
correct its content is.

---

## 11. Forbidden Anti-Patterns

- Absolute machine-local paths in prose or links (`/Users/…`, `C:\…`).
- Inline paragraph-length lists of links with only connecting words between
  them. If you have more than three links in a sentence, restructure.
- Link anchor text that says `source`, `link`, `here`, `this`, or a raw URL.
- `[target-state]` disclaimers threaded through multiple paragraphs of body
  prose — use frontmatter status plus a dedicated `## Today vs. Planned` section.
- Mixing Markdown image syntax with file-system paths (`![foo](/Users/…)`).
- `tmp/` directories holding load-bearing content. If something is canonical,
  move it up and out.
- Duplicating content across pages. Link, don't restate.
- File fragments that cover sub-parts of a concept that has no independent
  failure modes, API, or audience. Those collapse back into the parent concept.
