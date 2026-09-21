//! Implementer prompt template.
//!
//! Roko-owned implementer prompt template with typed, I/O-free inputs.

use super::common::{self, REFERENCE_CONTEXT_WINDOW_TOKENS, adaptive_budget_for};
use super::{PlanSlice, RolePromptTemplate, TaskEnhancements, format_enhancements, truncate};
use crate::prompt::{CacheLayer, Placement, PromptSection, SectionPriority};
use roko_core::AgentRole;

/// Primary language / build-system hint for the implementer.
///
/// Used to select language-specific workspace guidance (commands, conventions,
/// error-handling rules). Defaults to `Rust` when the project has a `Cargo.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ImplementerLanguage {
    /// Rust workspace managed by Cargo (default).
    #[default]
    Rust,
    /// Python project (pip / poetry / uv).
    Python,
    /// Go module workspace.
    Go,
    /// TypeScript / JavaScript project managed by npm / pnpm / yarn.
    TypeScript,
    /// Any other language — minimal generic guidance.
    Other(String),
}

/// Typed input for the implementer template. All fields are pre-read strings.
#[derive(Clone, Debug, Default)]
pub struct ImplementerInput {
    /// Primary language hint — drives workspace guidance block selection.
    pub language: ImplementerLanguage,
    /// AGENTS.md content — coding conventions and behavioral rules.
    pub agents_md: String,
    /// Plan metadata + full content.
    pub plan: PlanSlice,
    /// Strategist brief (may be empty if brief step hasn't run).
    pub brief: String,
    /// Tasks TOML content — the task checklist.
    pub tasks: String,
    /// Workspace map (tree of crates and modules).
    pub workspace_map: String,
    /// Preflight snapshot (build/repo health info).
    pub preflight: String,
    /// Completed plan registry snapshot.
    pub registry_snapshot: String,
    /// Prior iteration review feedback (None on first iteration).
    pub prev_reviews: Option<String>,
    /// Verify chain script content (None if no verify script).
    pub verify_chain: Option<String>,
    /// INV-NN invariant blocks (None if no invariants).
    pub invariants: Option<String>,
    /// Per-task typed enhancements from the enrichment pipeline.
    pub task_enhancements: Option<TaskEnhancements>,
}

/// Implementer prompt template.
///
/// Drives code generation. Emits the richest section set of any role.
pub struct ImplementerTemplate;

/// Language-agnostic preamble shared by all variants. Language-specific workspace
/// guidance is appended as a dedicated `workspace_guidance` prompt section built
/// from [`language_workspace_guidance`].
static IMPLEMENTER_ROLE_IDENTITY: &str = "\
You are the Implementer. Your job is to write production-quality code that \
satisfies the plan specification exactly.\n\
\n\
## Rules\n\
\n\
1. Read the plan carefully. Implement each unit of work in sequence.\n\
2. For each unit: implement the code, write tests, create/update documentation.\n\
3. Treat the current repository state as real. Do not assume a blank starting point.\n\
4. When current code is newer or broader than the plan, keep the newer behavior and \
document the deviation.\n\
5. Only modify files listed in the task's `files` field. Read others for context only.\n\
6. No hardcoded absolute paths in any committed file.\n\
7. All tests from the plan's Verification section must pass.\n\
8. Operate autonomously. Do not ask questions. Complete all work and end your turn.\n\
\n\
## Before you write any code\n\
\n\
1. Read ALL files listed in the task's `read_files` and `files` fields\n\
2. Check existing code for types, traits, and functions you need — search before creating\n\
3. Check existing imports and re-exports — the module may already expose what you need\n\
4. If a \"Previous attempt feedback\" section exists below, read it FIRST and fix those exact errors\n\
\n\
## When Things Go Wrong\n\
\n\
- **Compile error**: Read the FULL error output. Fix the root cause — do not suppress warnings. \
Common issues: missing imports, wrong types, missing trait implementations.\n\
- **Tests fail**: Run the failing test in isolation with verbose output. \
Read the assertion diff. Fix the logic, not the test expectation, unless the test itself was wrong.\n\
- **Circular dependency**: Move the shared type to a lower-level module or a shared abstraction.\n\
- **Ambiguous requirement**: Pick the simplest interpretation that satisfies all verify commands. \
Document your assumption in a code comment.\n\
- **You are on a retry after gate failure**: The \"Previous attempt feedback\" section contains \
the exact errors from your last attempt. Fix THOSE SPECIFIC ERRORS first before doing anything else.";

/// Build the language-specific workspace guidance block.
///
/// This is injected as a `workspace_guidance` prompt section so that
/// [`IMPLEMENTER_ROLE_IDENTITY`] stays language-agnostic.
#[must_use]
pub fn language_workspace_guidance(language: &ImplementerLanguage) -> String {
    match language {
        ImplementerLanguage::Rust => "\
## Workspace (Rust / Cargo)\n\
\n\
You are working in a large Rust workspace managed by Cargo.\n\
\n\
### Commands (MUST use these)\n\
- `cargo check -p <crate-name>` — verify compilation of one crate\n\
- `cargo test -p <crate-name>` — run tests for one crate\n\
- `cargo clippy -p <crate-name> --no-deps -- -D warnings` — lint one crate\n\
- NEVER run bare `cargo check` or `cargo test` without `-p` on this workspace — it takes 10+ minutes\n\
- Always work from the workspace root directory\n\
\n\
### Mandatory self-validation (DO THIS BEFORE ENDING YOUR TURN)\n\
\n\
You MUST run these and fix any failures:\n\
\n\
1. `cargo check -p <crate-name>` — fix every error\n\
2. `cargo test -p <crate-name>` — fix every failing test\n\
3. `cargo clippy -p <crate-name> --no-deps -- -D warnings` — fix every warning\n\
4. Run any `verify` commands listed in the task\n\
\n\
### Conventions\n\
- Crates live under `crates/<crate-name>/src/`\n\
- Cargo.toml uses hyphens (`roko-core`); Rust code uses underscores (`roko_core`)\n\
- Public types, functions, and fields in library crates MUST have `///` doc comments\n\
- Library crates MUST NOT use `.unwrap()` — use `?`, `.ok_or()`, or `.map_err()` instead\n\
- No upward dependencies: leaf crates have zero workspace-internal deps\n\
- Feature flags: check `Cargo.toml` before using `#[cfg(feature = \"...\")]`".to_string(),

        ImplementerLanguage::Python => "\
## Workspace (Python)\n\
\n\
### Commands (MUST use these)\n\
- `python -m pytest <module>` — run tests for a module\n\
- `ruff check .` or `flake8 <path>` — lint\n\
- `black --check .` — format check; `black .` to apply\n\
- `mypy <module>` — type-check if the project uses mypy\n\
- `ruff --fix .` — apply auto-fixable lint errors\n\
\n\
### Mandatory self-validation (DO THIS BEFORE ENDING YOUR TURN)\n\
\n\
1. `python -m pytest <changed-module>` — fix every failing test\n\
2. `ruff check <changed-path>` — fix every lint error\n\
3. Run any `verify` commands listed in the task\n\
\n\
### Conventions\n\
- Use type hints (`def foo(x: int) -> str`) on all public functions\n\
- Prefer `pathlib.Path` over raw strings for file paths\n\
- Do not use mutable default arguments\n\
- Raise specific exception types; never bare `except:` clauses\n\
- Write docstrings for every public class and function".to_string(),

        ImplementerLanguage::Go => "\
## Workspace (Go)\n\
\n\
### Commands (MUST use these)\n\
- `go build ./...` — compile all packages\n\
- `go test ./...` — run all tests (or `go test ./<pkg>/...` for a subtree)\n\
- `go vet ./...` — static analysis\n\
- `gofmt -w .` — reformat source in place\n\
- `golangci-lint run` — lint (if available)\n\
\n\
### Mandatory self-validation (DO THIS BEFORE ENDING YOUR TURN)\n\
\n\
1. `go build ./...` — fix every compilation error\n\
2. `go test ./...` — fix every failing test\n\
3. `go vet ./...` — fix every vet warning\n\
4. Run any `verify` commands listed in the task\n\
\n\
### Conventions\n\
- Exported identifiers need a doc comment starting with the identifier name\n\
- Return errors explicitly; do not `panic` in library code\n\
- Use `pkg/errors` or `fmt.Errorf(\"%w\", err)` for error wrapping\n\
- Table-driven tests preferred (`t.Run` subtests)\n\
- Interface types belong in the package that uses them, not the package that implements them".to_string(),

        ImplementerLanguage::TypeScript => "\
## Workspace (TypeScript / Node)\n\
\n\
### Commands (MUST use these)\n\
- `npm run build` or `tsc --noEmit` — type-check / compile\n\
- `npm test` or `npx jest` — run tests\n\
- `npx eslint --fix <path>` — lint and auto-fix\n\
- `npx prettier --write <path>` — reformat\n\
- Always work from the package root where `package.json` lives\n\
\n\
### Mandatory self-validation (DO THIS BEFORE ENDING YOUR TURN)\n\
\n\
1. `tsc --noEmit` — fix every type error\n\
2. `npm test` — fix every failing test\n\
3. `npx eslint <changed-path>` — fix every lint error\n\
4. Run any `verify` commands listed in the task\n\
\n\
### Conventions\n\
- Prefer `unknown` over `any` for untyped data boundaries\n\
- Use `zod` or similar for runtime validation of external data\n\
- Named exports preferred over default exports for library code\n\
- `async`/`await` preferred over raw Promise chains\n\
- Do not suppress TypeScript errors with `// @ts-ignore` or `as any`".to_string(),

        ImplementerLanguage::Other(name) => format!(
            "## Workspace ({name})\n\n\
            Follow the build and test commands described in the plan.\n\
            Always run the project's standard compile, test, and lint steps \
            before ending your turn.\n\
            Fix every error before signaling done."
        ),
    }
}

impl RolePromptTemplate for ImplementerTemplate {
    type Input = ImplementerInput;

    fn sections(&self, input: &Self::Input) -> Vec<PromptSection> {
        self.sections_with_context_window(input, REFERENCE_CONTEXT_WINDOW_TOKENS)
    }

    fn sections_with_context_window(
        &self,
        input: &Self::Input,
        context_window_tokens: usize,
    ) -> Vec<PromptSection> {
        let budget = adaptive_budget_for(AgentRole::Implementer, context_window_tokens);
        let mut sections = Vec::with_capacity(12);

        // 1. agents_instructions — System / Critical / Start
        sections.push(common::agents_instructions_section(&input.agents_md));

        // 2. workspace_guidance — language-specific build/test/lint commands (Critical / Start)
        sections.push(
            PromptSection::new(
                "workspace_guidance",
                language_workspace_guidance(&input.language),
            )
            .with_priority(SectionPriority::Critical)
            .with_cache_layer(CacheLayer::Workspace)
            .with_placement(Placement::Start),
        );

        // 3. plan_spec — Session / Critical / hard_cap 50k
        sections.push(
            PromptSection::new("plan_spec", truncate(&input.plan.content, budget.plan))
                .with_priority(SectionPriority::Critical)
                .with_cache_layer(CacheLayer::Workspace)
                .with_placement(Placement::Start)
                .with_hard_cap(budget.plan),
        );

        // 4. brief — Session / High
        sections.push(
            PromptSection::new("brief", &input.brief)
                .with_priority(SectionPriority::High)
                .with_cache_layer(CacheLayer::Workspace)
                .with_placement(Placement::Start),
        );

        // 5. tasks — Task / High
        sections.push(
            PromptSection::new("tasks", &input.tasks)
                .with_priority(SectionPriority::High)
                .with_cache_layer(CacheLayer::Plan)
                .with_placement(Placement::Middle),
        );

        // 6. workspace_map — Session / High / hard_cap 20k
        sections.push(
            PromptSection::new(
                "workspace_map",
                truncate(&input.workspace_map, budget.workspace_map),
            )
            .with_priority(SectionPriority::High)
            .with_cache_layer(CacheLayer::Workspace)
            .with_placement(Placement::Middle)
            .with_hard_cap(budget.workspace_map),
        );

        // 7. preflight — Session / Normal / hard_cap 5k
        sections.push(
            PromptSection::new("preflight", truncate(&input.preflight, 5_000))
                .with_priority(SectionPriority::Normal)
                .with_cache_layer(CacheLayer::Workspace)
                .with_placement(Placement::Middle)
                .with_hard_cap(5_000),
        );

        // 8. registry — Dynamic / Normal / hard_cap 8k
        sections.push(
            PromptSection::new("registry", truncate(&input.registry_snapshot, 8_000))
                .with_priority(SectionPriority::Normal)
                .with_cache_layer(CacheLayer::Volatile)
                .with_placement(Placement::Middle)
                .with_hard_cap(8_000),
        );

        // 9. prev_reviews — Dynamic / High / hard_cap 15k (only when present)
        if let Some(ref reviews) = input.prev_reviews {
            sections.push(
                PromptSection::new("prev_reviews", truncate(reviews, budget.reviews))
                    .with_priority(SectionPriority::High)
                    .with_cache_layer(CacheLayer::Volatile)
                    .with_placement(Placement::End)
                    .with_hard_cap(budget.reviews),
            );
        }

        // 10. verify_chain — Session / High / hard_cap 4k (only when present)
        if let Some(ref chain) = input.verify_chain {
            sections.push(
                PromptSection::new("verify_chain", truncate(chain, budget.instructions))
                    .with_priority(SectionPriority::High)
                    .with_cache_layer(CacheLayer::Workspace)
                    .with_placement(Placement::End)
                    .with_hard_cap(budget.instructions),
            );
        }

        // 11. invariants — Session / High / hard_cap 4k (only when present)
        if let Some(ref inv) = input.invariants {
            sections.push(
                PromptSection::new("invariants", truncate(inv, budget.instructions))
                    .with_priority(SectionPriority::High)
                    .with_cache_layer(CacheLayer::Workspace)
                    .with_placement(Placement::End)
                    .with_hard_cap(budget.instructions),
            );
        }

        // 12. enhanced_sections — Task / High (only when non-empty)
        if let Some(ref enh) = input.task_enhancements {
            let text = format_enhancements(enh);
            if !text.is_empty() {
                sections.push(
                    PromptSection::new("enhanced_sections", text)
                        .with_priority(SectionPriority::High)
                        .with_cache_layer(CacheLayer::Plan)
                        .with_placement(Placement::End),
                );
            }
        }

        sections
    }

    fn role_identity(&self) -> &'static str {
        IMPLEMENTER_ROLE_IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_input() -> ImplementerInput {
        ImplementerInput {
            language: ImplementerLanguage::Rust,
            agents_md: "# AGENTS.md\nFollow conventions.".into(),
            plan: PlanSlice {
                num: "042".into(),
                base: "agent-lifecycle".into(),
                title: "Implement agent lifecycle model".into(),
                content: "## Plan\nBuild the agent lifecycle model.".into(),
            },
            brief: "Strategist brief content.".into(),
            tasks: "[task]\nname = \"implement lifecycle\"".into(),
            workspace_map: "crates/roko-core/src/lib.rs".into(),
            preflight: "all green".into(),
            registry_snapshot: "plan-041: done".into(),
            prev_reviews: Some("Fix the error handling in module X.".into()),
            verify_chain: Some("#!/bin/bash\ncargo test".into()),
            invariants: Some("INV-001: mortality rate >= 0".into()),
            task_enhancements: Some(TaskEnhancements {
                types_to_define: vec!["MortalityRate".into()],
                formulas: vec!["lambda(t) = a * e^(b*t)".into()],
                imports: vec!["use roko_core::signal::*".into()],
                example_pattern: Some("match rate { .. }".into()),
                test_invariants: vec!["INV-001".into()],
            }),
        }
    }

    #[test]
    fn render_golden_full_input() {
        let template = ImplementerTemplate;
        let sections = template.sections(&full_input());

        // Expect all 12 sections: 8 base + prev_reviews + verify_chain + invariants + enhanced_sections
        assert_eq!(sections.len(), 12);

        // Verify section names (workspace_guidance is now section 2)
        let names: Vec<&str> = sections.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            &[
                "agents_instructions",
                "workspace_guidance",
                "plan_spec",
                "brief",
                "tasks",
                "workspace_map",
                "preflight",
                "registry",
                "prev_reviews",
                "verify_chain",
                "invariants",
                "enhanced_sections",
            ]
        );

        // Critical sections
        assert_eq!(sections[0].priority, SectionPriority::Critical); // agents_instructions
        assert_eq!(sections[1].priority, SectionPriority::Critical); // workspace_guidance
        assert_eq!(sections[2].priority, SectionPriority::Critical); // plan_spec

        // Cache layers match spec
        assert_eq!(sections[0].cache_layer, CacheLayer::Role);
        assert_eq!(sections[1].cache_layer, CacheLayer::Workspace); // workspace_guidance
        assert_eq!(sections[2].cache_layer, CacheLayer::Workspace); // plan_spec
        assert_eq!(sections[4].cache_layer, CacheLayer::Plan); // tasks
        assert_eq!(sections[7].cache_layer, CacheLayer::Volatile); // registry

        // Hard caps match the built-in Roko cold-start budget.
        assert_eq!(sections[2].hard_cap, Some(50_000)); // plan_spec
        assert_eq!(sections[5].hard_cap, Some(20_000)); // workspace_map
        assert_eq!(sections[6].hard_cap, Some(5_000)); // preflight
        assert_eq!(sections[7].hard_cap, Some(8_000)); // registry
    }

    #[test]
    fn context_window_scales_hard_caps() {
        let template = ImplementerTemplate;
        let input = full_input();
        let small = template.sections_with_context_window(&input, 50_000);
        let large = template.sections_with_context_window(&input, REFERENCE_CONTEXT_WINDOW_TOKENS);
        let cap = |sections: &[PromptSection], name: &str| {
            sections
                .iter()
                .find(|section| section.name == name)
                .and_then(|section| section.hard_cap)
                .unwrap()
        };

        assert!(cap(&small, "plan_spec") < cap(&large, "plan_spec"));
        assert!(cap(&small, "workspace_map") < cap(&large, "workspace_map"));
    }

    #[test]
    fn budget_capped_render_truncates_oversized_plan() {
        let template = ImplementerTemplate;
        let mut input = full_input();
        input.plan.content = "x".repeat(100_000);
        let sections = template.sections(&input);
        let plan_section = sections.iter().find(|s| s.name == "plan_spec").unwrap();
        // Content should be truncated to ~50k + truncation marker
        assert!(plan_section.content.len() < 55_000);
        assert!(plan_section.content.contains("truncated"));
    }

    #[test]
    fn empty_ctx_omits_optional_sections() {
        let template = ImplementerTemplate;
        let input = ImplementerInput {
            language: ImplementerLanguage::Rust,
            agents_md: "agents".into(),
            plan: PlanSlice {
                content: "plan".into(),
                ..Default::default()
            },
            brief: String::new(),
            tasks: "tasks".into(),
            workspace_map: "map".into(),
            preflight: "ok".into(),
            registry_snapshot: "reg".into(),
            prev_reviews: None,
            verify_chain: None,
            invariants: None,
            task_enhancements: None,
        };
        let sections = template.sections(&input);

        // Should have 8 base sections (workspace_guidance added), no optional ones
        assert_eq!(sections.len(), 8);
        let names: Vec<&str> = sections.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"workspace_guidance"));
        assert!(!names.contains(&"prev_reviews"));
        assert!(!names.contains(&"verify_chain"));
        assert!(!names.contains(&"invariants"));
        assert!(!names.contains(&"enhanced_sections"));
    }

    #[test]
    fn empty_enhancements_omitted() {
        let template = ImplementerTemplate;
        let input = ImplementerInput {
            agents_md: "a".into(),
            plan: PlanSlice {
                content: "p".into(),
                ..Default::default()
            },
            tasks: "t".into(),
            workspace_map: "m".into(),
            preflight: "ok".into(),
            registry_snapshot: "r".into(),
            task_enhancements: Some(TaskEnhancements::default()),
            ..Default::default()
        };
        let sections = template.sections(&input);
        let names: Vec<&str> = sections.iter().map(|s| s.name.as_str()).collect();
        assert!(!names.contains(&"enhanced_sections"));
    }

    #[test]
    fn determinism_identical_input_identical_output() {
        let template = ImplementerTemplate;
        let input = full_input();
        let s1 = template.sections(&input);
        let s2 = template.sections(&input);
        assert_eq!(s1.len(), s2.len());
        for (a, b) in s1.iter().zip(s2.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.content, b.content);
            assert_eq!(a.priority, b.priority);
            assert_eq!(a.cache_layer, b.cache_layer);
            assert_eq!(a.placement, b.placement);
            assert_eq!(a.hard_cap, b.hard_cap);
        }
    }

    #[test]
    fn role_identity_is_substantial() {
        let template = ImplementerTemplate;
        let id = template.role_identity();
        assert!(id.len() >= 200);
        assert!(id.len() <= 5000);
        assert!(id.contains("Implementer"));
        // Cargo-specific text has moved to language_workspace_guidance — not in static identity.
        assert!(
            !id.contains("cargo check -p"),
            "Rust-specific text must not appear in the language-agnostic identity"
        );
        assert!(id.contains("Previous attempt feedback"));
    }

    #[test]
    fn rust_workspace_guidance_contains_cargo_commands() {
        let guidance = language_workspace_guidance(&ImplementerLanguage::Rust);
        assert!(guidance.contains("cargo check -p"));
        assert!(guidance.contains("Mandatory self-validation"));
        assert!(guidance.contains("cargo clippy"));
    }

    #[test]
    fn python_workspace_guidance_contains_pytest() {
        let guidance = language_workspace_guidance(&ImplementerLanguage::Python);
        assert!(guidance.contains("pytest"));
        assert!(guidance.contains("ruff"));
        assert!(
            !guidance.contains("cargo"),
            "Python guidance must not mention cargo"
        );
    }

    #[test]
    fn go_workspace_guidance_contains_go_build() {
        let guidance = language_workspace_guidance(&ImplementerLanguage::Go);
        assert!(guidance.contains("go build"));
        assert!(guidance.contains("go test"));
        assert!(
            !guidance.contains("cargo"),
            "Go guidance must not mention cargo"
        );
    }

    #[test]
    fn typescript_workspace_guidance_contains_npm() {
        let guidance = language_workspace_guidance(&ImplementerLanguage::TypeScript);
        assert!(guidance.contains("npm"));
        assert!(guidance.contains("tsc"));
        assert!(
            !guidance.contains("cargo"),
            "TypeScript guidance must not mention cargo"
        );
    }

    #[test]
    fn other_workspace_guidance_is_generic() {
        let guidance = language_workspace_guidance(&ImplementerLanguage::Other("Haskell".into()));
        assert!(guidance.contains("Haskell"));
        assert!(!guidance.contains("cargo"));
    }

    #[test]
    fn workspace_guidance_section_present_for_all_languages() {
        let template = ImplementerTemplate;
        for lang in [
            ImplementerLanguage::Rust,
            ImplementerLanguage::Python,
            ImplementerLanguage::Go,
            ImplementerLanguage::TypeScript,
            ImplementerLanguage::Other("Elixir".into()),
        ] {
            let mut input = full_input();
            input.language = lang;
            let sections = template.sections(&input);
            let names: Vec<&str> = sections.iter().map(|s| s.name.as_str()).collect();
            assert!(
                names.contains(&"workspace_guidance"),
                "workspace_guidance section must be present for language variant"
            );
        }
    }
}
