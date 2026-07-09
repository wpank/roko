# Batch REF01 — Critique "one-noun, six-verbs" across docs/00-architecture

**Refinement source**: `tmp/refinements/01-critique-one-noun.md` (injected
above under "Canonical refinement source"). That source is the diagnosis;
your job is to propagate it into the canonical `docs/` so the existing
documentation no longer advertises the reductive framing as the last
word.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies, but
the primary candidates are:

- `docs/00-architecture/INDEX.md` — the chapter index; lead paragraph
  still says "one noun, six verbs" in most corpora.
- `docs/00-architecture/06-synapse-traits.md` — "The number six is not
  arbitrary" passage; soften and forward-link to the
  two-medium / two-fabric refinement.
- `docs/00-architecture/23-architectural-analysis-improvements.md` —
  audit doc. Add a footer noting that §2.2 "Adequate / awkward but
  functional" trait fits and §3.2 roko-conductor → roko-learn violation
  are the motivating evidence for the two-fabric reframing.
- `docs/INDEX.md` — top-level index. Update any "one noun, six verbs"
  one-liners.

## Required outputs

- Every updated doc retains its filename and high-level structure.
- The lead paragraph / abstract of `docs/00-architecture/INDEX.md`
  (and `docs/INDEX.md` if it carries a similar framing) evolves to
  something like:
  > Roko's kernel has two mediums (durable Engram + ephemeral Pulse)
  > moving through two fabrics (Substrate + Bus), acted on by six
  > operators, across five layers at three speeds with three
  > cross-cuts.
- A cross-reference to `tmp/refinements/01-critique-one-noun.md` and
  the remainder of the refinements folder appears near the top of any
  file you touch.
- No line asserts "one noun, six verbs" as the complete story. If the
  original phrase is retained for historical context, it is framed as
  "the original mnemonic; see the two-medium / two-fabric refinement."
- `06-synapse-traits.md` adds a prominent "See also" section pointing at
  the refinements folder and notes the signature generalization
  (covered in REF04).

## Cross-references

Dependents (downstream refinements that assume this critique has been
acknowledged in docs): REF02, REF03, REF04, REF05, REF07.

## Rules

Follow all rules in `context-pack/00-REFINEMENTS-RULES.md`:

- Only touch files under `docs/`.
- Substantive edits — no placeholders.
- Any retired terms ("Signal = Engram" disclaimer, `Bardo`, `Golem`,
  `Mori`, `Grimoire`, `Styx`, `Clade`) must either be removed or
  explicitly framed as retired.
- Required new vocabulary for this batch (verify): words matching
  `two mediums|two fabrics|six operators` should appear in at least
  one of the changed files.

## Done when

- The diff gate, scope gate, terminology gate, and required-term gate
  all pass.
- A commit message `refinements(REF01): Critique ...` is ready for the
  runner.
- Final message lists: which files changed, which retired disclaimers
  were removed, and which cross-references were added.
