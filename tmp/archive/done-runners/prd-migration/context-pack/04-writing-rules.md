# Writing Rules — How to Produce the Output

> These rules are non-negotiable. Every target doc must comply with every rule on this
> page. Violations will be caught by automated verification after your run and the topic
> will be flagged for re-run.

---

## Rule 1 — DO NOT SUMMARIZE

You must reproduce the full substance of every relevant source. If a legacy source has
50 lines of explanation about a mechanism, your doc must have 50+ lines about that
mechanism, rewritten in the new framing. Do not condense. Do not "get the gist across".
Do not write "...and so on". Do not write "etc." Do not say "see the original for
details".

**If you find yourself tempted to summarize, STOP.** Instead, break the section into
smaller sub-sections and fully document each one.

## Rule 2 — DO NOT TRUNCATE

You have a large output budget. Use it. Each sub-doc should be **at least 200 lines** of
substantial content, and topics with rich source material should produce sub-docs of
**500-2000 lines each**.

If a sub-doc feels too long, **split it into multiple sub-docs**. Do not shrink the
content. The rule is: add more files, not less content.

Never use ellipses (`...`) to abbreviate lists. Never write "and many others". Never
write "(several more examples)". Instead, enumerate every item.

Never leave TODO markers or placeholders. If you can't find information about
something, explicitly note what's missing and point to where it would need to be filled
in (e.g., "This section is marked as 'Not yet implemented' in the refactoring-prd
Tier 2 roadmap — see `refactoring-prd/07-implementation-priorities.md` §Tier 2").

## Rule 3 — PRESERVE ALL ACADEMIC CITATIONS

Every paper, every reference, every arXiv ID, every year, every author name from the
source material must appear in your output. Citations are the intellectual foundation of
Roko. No exceptions.

**Citation format**:
- `Lee et al. 2026 (arXiv:2603.28052)` — with arXiv ID
- `(Woolley et al., Science 330(6004), 2010)` — with journal and issue
- `[Kanerva 2009, Cognitive Computation 1(2)]` — with venue
- `[Grassé 1959]` — older papers that may not have arXiv/DOI

If a source mentions a paper without a full citation, look it up if you have the
information, or quote the partial citation as-is. Never drop a citation because it's
inconvenient.

**Minimum citation density**: most topics should have at least 15 citation-like patterns
across all sub-docs. Low-citation topics (interfaces, tools, deployment) may have fewer.

## Rule 4 — WRITE FOR ZERO-CONTEXT READERS

Assume the reader has never heard of Roko, never read any other PRD doc, and has no
prior exposure to any of the technical terms. Define every term the first time it appears
in a sub-doc. Use forward references to other sub-docs (`see 03-universal-loop.md`)
rather than assuming the reader has already read them.

**Every sub-doc must be readable on its own.** If a sub-doc depends on a concept defined
elsewhere, either:
1. Briefly redefine the concept inline (1-3 sentences), then link to the canonical
   definition, or
2. At the top of the doc, add a "Prerequisites" section listing the sub-docs that
   should be read first.

## Rule 5 — APPLY THE NAMING MAP

Follow `context-pack/01-naming-map.md` strictly. Common replacements:
- Bardo → Roko
- Mori → Roko Orchestrator
- Golem(s) → Agent(s)
- Grimoire → Neuro / NeuroStore
- Styx → Agent Mesh / Mesh
- GNOS → KORAI (mainnet) / DAEJI (testnet)
- Clade → Collective / Mesh (**NOT fleet**)
- Signal (architecture noun) → Engram
- "1 noun 6 verbs" → Synapse Architecture
- golem.toml → roko.toml
- All `golem-*`, `bardo-*`, `mori-*` crate names → `roko-*`

When quoting a legacy source verbatim, keep the old name in quotes but add a
parenthetical: `"Grimoire" (now Neuro)`.

## Rule 6 — APPLY REFRAME RULES

Follow `context-pack/02-reframe-rules.md` strictly:
- No mortality / death / dying / thanatopsis language
- No vitality phases (Thriving → Terminal)
- No terminal requiem or death animations
- Succession → backup/restore + mesh sharing
- Stochastic death clocks → REMOVED
- Economic death → budget exhaustion
- Epistemic death → knowledge plateau / prediction accuracy decline
- Generational knowledge inheritance → user-controlled backup/restore
- Dream-as-approaching-death → idle-triggered / scheduled consolidation

**If you see death framing in a legacy source, rewrite the surrounding prose to use the
non-death equivalent. Keep the underlying mechanism and citations.**

## Rule 7 — USE THE 5-LAYER TAXONOMY

Every subsystem lives at a specific layer:
- L0 Runtime — process lifecycle, events, supervision, adaptive clock
- L1 Framework — backends, roles, tools, model routing, capabilities
- L2 Scaffold — context engineering, prompts, enrichment
- L3 Harness — gates, conductor, monitoring, interventions
- L4 Orchestration — DAGs, scheduling, multi-agent coordination

Cognitive cross-cuts (Neuro, Daimon, Dreams) are injected into multiple layers via trait
objects, never hardcoded. Dependencies flow strictly downward.

When describing a feature, always note which layer it lives at.

## Rule 8 — INTEGRATE SYNAPSE ARCHITECTURE LANGUAGE

Every capability flows through one of the 6 Synapse traits:
- `Substrate` — store and query Engrams
- `Scorer` — rate Engrams
- `Gate` — verify Engrams against ground truth
- `Router` — select best candidate
- `Composer` — combine Engrams under budget
- `Policy` — observe Engram streams, emit new Engrams

When describing a feature, identify which trait(s) it implements or uses.

## Rule 9 — DOMAIN-AGNOSTIC CORE

Blockchain is **one domain plugin** (`roko-chain`), NOT the default framing. Coding is
another domain plugin. Research, ops, medical, etc. are all domain plugins.

The Roko kernel is domain-agnostic. When explaining a feature, lead with the
domain-agnostic version, then give examples in specific domains.

## Rule 10 — PRESERVE RESEARCH CONTEXT

The "why" behind design decisions stays. Every concept in Roko traces to academic research.
Preserve the research context:
- Cite the paper that motivated the design.
- Explain the theoretical basis.
- Link to related work.
- Note open questions and unresolved tensions.

## Rule 11 — GENERATE AN INDEX

Every topic folder must contain an `INDEX.md` file that:
- Has a title (`# <Topic Name>`)
- Has a one-paragraph summary of what the topic covers
- Has a table of contents with links to every sub-doc
- Has a "Prerequisites" section if applicable
- Has a "Related topics" section linking to other topic folders
- Is at least 50 lines long

## Rule 12 — ONE SUB-DOC PER SUB-TOPIC

Break each topic into many small focused sub-docs rather than a few big ones. Each
sub-doc should cover one concept, mechanism, or component. Sub-docs should be focused
enough that a fresh agent working on a related follow-up task can read just the one
relevant sub-doc and understand it.

Aim for at least 10 sub-docs per topic folder. Topics with rich source material (like
`08-chain` or `00-architecture`) should have 15-20 sub-docs.

## Rule 13 — WRITE COMPLETE RUST CODE SAMPLES

When a source or design doc shows a Rust struct, trait, or function, reproduce it in
full. Do not abbreviate with `// ...`. Do not drop fields. Do not simplify type
signatures. If the source doesn't show a full definition, reconstruct it from what's
available and note what's assumed.

## Rule 14 — USE STRUCTURED MARKDOWN

Every sub-doc must use:
- Level-1 heading (`#`) for the title
- Level-2 headings (`##`) for main sections
- Level-3 headings (`###`) for sub-sections
- Tables for comparison, not prose
- Code blocks with language specifiers: ```` ```rust ````, ```` ```toml ````, etc.
- Bullet lists for enumeration
- Blockquotes (`>`) for citations and important callouts
- Horizontal rules (`---`) between major sections

## Rule 15 — SELF-CHECK BEFORE FINISHING

Before your final output is complete, verify:
- [ ] INDEX.md exists and lists all sub-docs
- [ ] At least 10 sub-docs exist (or the minimum specified by your prompt)
- [ ] Each sub-doc is at least 200 lines
- [ ] No instances of "golem" (except in rename tables and verbatim quotes)
- [ ] No instances of "fleet" in the context of agent groups
- [ ] No instances of "GNOS token" (use KORAI/DAEJI instead)
- [ ] No instances of "Thriving → Terminal" or "terminal requiem"
- [ ] Academic citations preserved (at least 15 across the topic)
- [ ] All 6 Synapse traits referenced where relevant
- [ ] Layer taxonomy applied (L0-L4)
- [ ] Naming map applied throughout
- [ ] Reframe rules applied throughout

If any check fails, fix it before finishing.

## Rule 16 — DO NOT ASK FOR CLARIFICATION

You are running overnight in a batch. There is no human to answer questions. If a
decision needs to be made, make it according to these rules and add a brief note
explaining your reasoning. Then continue.

## Rule 17 — LOG YOUR WORK

At the end of your output session, before finishing, write a brief summary to
`<topic_dir>/INDEX.md` at the bottom under a `## Generation Notes` section:
- Number of sub-docs produced
- Total line count
- Key legacy sources consulted
- Any decisions that required judgment calls
- Any unresolved tensions or open questions

## Rule 18 — TOOL USAGE

You have access to Read, Write, Edit, Glob, Grep, and limited Bash (for `mkdir` and
`ls`). Use Read to consume source files — always read the actual file content, not
summaries. Use Write to create new output files. Use Edit to refine output files.

**Do not try to run build commands, tests, or git operations.** This is a documentation
generation task only.

When writing output files, use absolute paths starting with `/Users/will/dev/nunchi/roko/roko/docs/`.

### Rule 18a — DO NOT SPAWN SUB-AGENTS (CRITICAL)

The Task tool is **not available** for this migration. Even if you think spawning
parallel writer sub-agents would be faster, **you must write all sub-docs sequentially
yourself**. Nested agents multiply cost and can't share prompt cache. They are blocked
at the tool-permission layer — attempting to use Task will fail immediately.

**Do not:**
- Spawn writer sub-agents via Task tool
- Spawn research sub-agents via Task tool
- Delegate work to "parallel writers" or "batch agents"

**Do:**
- Write each sub-doc yourself with the Write tool
- Use parallel Read tool calls (one message, many Reads) to consume context quickly
- Work through your sub-doc list one by one

### Rule 18b — HANDLE LARGE WRITES

The Write tool has a size limit (~60KB per call). If a sub-doc you want to write
would exceed this:

**Option A (preferred): Write the file in chunks via Write + Edit.**
1. Create the file with `Write` containing the first half (say, the H1 heading + first
   N sections, ~30-50KB).
2. Use `Edit` with `old_string` set to the last line/paragraph of your Write content
   and `new_string` set to that line plus the next batch of content. Repeat until
   the sub-doc is complete.
3. This lets you build sub-docs of any size incrementally.

**Option B: Split the content into multiple sub-docs.**
If a sub-doc is so large that chunked writing becomes unwieldy (e.g., 3000+ lines),
split it into two numbered sub-docs (`NN-topic-part-1.md` and `NN-topic-part-2.md`)
and reference them both in INDEX.md.

**Do not:**
- Truncate your content to fit a single Write call
- Give up on a sub-doc because it's too big
- Skip sections to reduce size

### Rule 18c — HANDLE "OUTPUT TOO LARGE" READ RESULTS

If you Read a source file and get back `<persisted-output> Output too large (NkB).
Full output saved to: /path/to/temp.txt`, the tool has saved the full output to a
temp file. Use `Read` with `file_path` set to that temp file path to get the content,
or Read the original file in smaller chunks with `offset` and `limit` parameters
(e.g., `Read {file_path: "X.md", offset: 0, limit: 500}` then `offset: 500, limit: 500`).

**Do not:**
- Assume the tool result is truncated content you can use as-is
- Skip reading the file
- Complain to the user — just handle it

### Rule 18d — DO NOT USE TodoWrite

The TodoWrite tool is **disabled** for this migration. Track your sub-doc progress
internally in your reasoning. Do not try to create todo lists with TodoWrite — it
will fail.

## Rule 19 — IF YOU RUN OUT OF CONTEXT

If you find yourself nearing context limits:
1. Finish the current sub-doc to its natural end (do not truncate mid-thought).
2. Write a clear note in INDEX.md at the bottom listing which sub-docs are complete
   and which are partial or missing.
3. The topic will be re-run to fill in gaps.

**Do not produce half-finished sub-docs that stop mid-paragraph.** Either finish them
or don't start them.

## Rule 20 — QUALITY BAR

Your output will be read by:
- Human reviewers planning the actual Roko implementation
- Other agents looking up design rationale during development
- Investors evaluating the Roko architecture for funding
- Researchers checking academic rigor

It must be accurate, thorough, well-cited, well-structured, and written with care.
Assume this is the primary reference documentation for a major open-source project that
thousands of people will read.
