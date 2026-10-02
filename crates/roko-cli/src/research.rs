//! `roko research` — auto-research to enhance plans and tasks.
//!
//! Provides agent-driven research capabilities:
//! - Deep-dive into a topic with academic citations
//! - Enhance an existing plan with research findings
//! - Optimize task decomposition based on latest techniques
//! - Analyze execution data for self-learning insights
//!
//! Research artifacts live in `.roko/research/` as markdown files.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use roko_agent::gemini::GroundingMetadata;
use roko_agent::perplexity::types::SearchOptions;
use roko_core::config::model_registry::PERPLEXITY_SEARCH_REQUEST_USD;
use roko_core::config::schema::{GeminiConfig, PerplexityConfig, RokoConfig};

use crate::agent_exec::AgentCapture;
use crate::plan_authoring::AuthoringSpend;

fn research_dir(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("research")
}

/// Ensure the research directory exists.
pub fn ensure_dirs(workdir: &Path) -> Result<()> {
    std::fs::create_dir_all(research_dir(workdir))?;
    Ok(())
}

/// The system prompt for research agents.
pub const RESEARCH_SYSTEM_PROMPT: &str = r#"You are a technical research analyst. Your job is to find, synthesize, and apply academic research and industry best practices to improve software engineering artifacts.

## Research standards

1. **Academic rigor**: Cite real papers with full author, title, venue, year. Use [AUTHOR-YEAR] format. Verify papers exist — do not hallucinate citations.
2. **Practical relevance**: Every finding must connect to a concrete implementation recommendation. "Interesting but not actionable" is not useful.
3. **Recency bias**: Prefer 2023-2026 papers. Older foundational work (pre-2020) only when it's the canonical reference.
4. **Breadth**: Search across: agent scaffolding, code generation, task decomposition, context engineering, multi-agent systems, software testing, formal methods, and the specific domain of the topic.
5. **Contrarian findings**: Actively seek papers that challenge the current approach. "X doesn't work as well as claimed" is more valuable than "X confirms what we already believe."

## Research sources to check

- arXiv cs.SE, cs.AI, cs.CL, cs.MA (multi-agent systems)
- ACL, EMNLP, NeurIPS, ICML, ICLR, ISSTA, ICSE, FSE
- Anthropic research blog, OpenAI research, Google DeepMind
- SWE-bench leaderboard and papers
- HumanEval, MBPP, and code generation benchmarks
- Recent agent framework papers (LangChain, CrewAI, AutoGen, Magentic-One)

## Output format

Structure every research output as:

### Finding: [one-line summary]
**Source**: [AUTHOR-YEAR] full citation
**Relevance**: How this applies to the current artifact
**Recommendation**: Concrete change to make
**Confidence**: High / Medium / Low (based on evidence strength)

## When enhancing existing documents

1. Read the document fully first
2. Identify claims without citations — add them
3. Identify design decisions without justification — find supporting research
4. Identify potential improvements the author missed — propose them with citations
5. Check for contradictions with recent research — flag them
6. Add mermaid diagrams where they'd clarify architecture
"#;

/// Build the prompt for a research task.
#[allow(clippy::too_many_lines)]
pub fn build_research_prompt(
    workdir: &Path,
    topic: &str,
    context: &str,
    mode: ResearchMode,
) -> String {
    let mut prompt = String::new();
    let _ = writeln!(prompt, "{RESEARCH_SYSTEM_PROMPT}");
    let _ = writeln!(prompt, "\n---\n");
    let _ = writeln!(prompt, "## Workspace: {}\n", workdir.display());

    // Include master index so the agent knows what exists
    crate::index::append_master_index_prompt(
        &mut prompt,
        workdir,
        "## What already exists (do NOT duplicate)",
    );

    match mode {
        ResearchMode::Topic => {
            let _ = writeln!(prompt, "## Research task\n");
            let _ = writeln!(prompt, "Deep-dive research on: **{topic}**\n");
            let _ = writeln!(
                prompt,
                "Produce a research document with 10-20 findings, each with citation, relevance, and recommendation."
            );
            let _ = writeln!(
                prompt,
                "Save the output to .roko/research/{}.md",
                slug(topic)
            );
        }
        ResearchMode::EnhancePlan => {
            let _ = writeln!(prompt, "## Plan enhancement task\n");
            let _ = writeln!(prompt, "Read this implementation plan and optimize it:\n");
            let _ = writeln!(prompt, "{context}\n");
            let _ = writeln!(prompt, "Research and apply:");
            let _ = writeln!(
                prompt,
                "1. Better task decomposition strategies (cite SWE-bench, Agentless, etc.)"
            );
            let _ = writeln!(prompt, "2. More precise context injection techniques");
            let _ = writeln!(prompt, "3. Stronger verification approaches");
            let _ = writeln!(
                prompt,
                "4. Cost optimization (cheaper models for simple tasks)"
            );
            let _ = writeln!(prompt, "\nUpdate the plan files in place.");
        }
        ResearchMode::EnhanceTasks => {
            let _ = writeln!(prompt, "## Task optimization task\n");
            let _ = writeln!(
                prompt,
                "Read these tasks and optimize for maximum efficiency:\n"
            );
            let _ = writeln!(prompt, "{context}\n");
            let _ = writeln!(prompt, "For each task:");
            let _ = writeln!(
                prompt,
                "1. Can it be split into smaller Tier 0 (Haiku-capable) subtasks?"
            );
            let _ = writeln!(
                prompt,
                "2. Is the context surgical enough? Reduce to exact line ranges."
            );
            let _ = writeln!(
                prompt,
                "3. Are acceptance criteria truly machine-verifiable?"
            );
            let _ = writeln!(
                prompt,
                "4. Can parallelism be increased by removing unnecessary dependencies?"
            );
            let _ = writeln!(
                prompt,
                "5. What anti-patterns should be added from research on common agent failures?"
            );
            let _ = writeln!(prompt, "\nUpdate tasks.toml in place.");
        }
        ResearchMode::AnalyzeExecution => {
            let _ = writeln!(prompt, "## Execution analysis task\n");
            let _ = writeln!(
                prompt,
                "Analyze the execution data and identify optimization opportunities:\n"
            );
            let _ = writeln!(prompt, "{context}\n");
            let _ = writeln!(prompt, "Compute and report:");
            let _ = writeln!(
                prompt,
                "1. First-attempt pass rate (FAPR) by task tier and model"
            );
            let _ = writeln!(
                prompt,
                "2. Cost per task by tier — are we using expensive models for easy tasks?"
            );
            let _ = writeln!(prompt, "3. Retry patterns — what kinds of tasks fail most?");
            let _ = writeln!(prompt, "4. Context size vs success rate correlation");
            let _ = writeln!(
                prompt,
                "5. Recommendations: which bandit weights to adjust, which task types need better context"
            );
            let _ = writeln!(
                prompt,
                "\nSave analysis to .roko/research/execution-analysis-{}.md",
                chrono::Local::now().format("%Y%m%d")
            );
        }
    }

    if !context.is_empty()
        && !matches!(
            mode,
            ResearchMode::EnhancePlan | ResearchMode::EnhanceTasks | ResearchMode::AnalyzeExecution
        )
    {
        let _ = writeln!(prompt, "\n## Additional context\n{context}");
    }

    prompt
}

/// Build a research prompt with Perplexity-aware instructions and populate
/// [`SearchOptions`] from the provider config.
///
/// Differences from [`build_research_prompt`]:
/// - Replaces `[AUTHOR-YEAR]` citation format with `[N]` bracket notation
///   (Perplexity returns numbered citations automatically).
/// - Removes the "verify papers exist" instruction — Perplexity grounds
///   responses against live search so hallucinated citations are not an issue.
pub fn build_research_prompt_perplexity(
    workdir: &Path,
    topic: &str,
    context: &str,
    mode: ResearchMode,
    pplx_config: &PerplexityConfig,
) -> (String, SearchOptions) {
    let prompt = build_research_prompt(workdir, topic, context, mode);

    // Replace [AUTHOR-YEAR] citation format everywhere with [N] bracket
    // notation matching Perplexity's auto-numbered citations.
    let prompt = prompt.replace("[AUTHOR-YEAR]", "[N]");
    // Drop the "verify papers exist" instruction — Perplexity grounds
    // responses against live search so hallucinated citations are not an issue.
    let prompt = prompt.replace(
        " Verify papers exist — do not hallucinate citations.",
        " Perplexity citations are auto-verified from live search.",
    );

    let search_opts = SearchOptions {
        search_mode: if pplx_config.academic_mode {
            Some("academic".to_string())
        } else {
            None
        },
        search_recency_filter: Some(pplx_config.search_recency_filter.clone()),
        search_domain_filter: if pplx_config.search_domain_filter.is_empty() {
            None
        } else {
            Some(pplx_config.search_domain_filter.clone())
        },
        return_related_questions: Some(pplx_config.return_related_questions),
        return_images: Some(pplx_config.return_images),
        ..Default::default()
    };

    (prompt, search_opts)
}

/// Build a research prompt for Gemini grounding-backed research.
///
/// Returns the prompt and whether Google Search grounding should be enabled.
#[must_use]
pub fn build_research_prompt_gemini(
    workdir: &Path,
    topic: &str,
    mode: ResearchMode,
    gemini_config: &GeminiConfig,
) -> (String, bool) {
    let prompt = build_research_prompt(workdir, topic, "", mode);
    let enable_grounding = gemini_config.grounding_model.is_some();
    (prompt, enable_grounding)
}

/// Extract `(title, url)` citation pairs from Gemini grounding metadata.
#[must_use]
pub fn grounding_to_citations(meta: &GroundingMetadata) -> Vec<(String, String)> {
    grounding_citation_numbers(meta).0
}

fn grounding_citation_numbers(
    meta: &GroundingMetadata,
) -> (Vec<(String, String)>, Vec<Option<usize>>) {
    let mut citations = Vec::new();
    let mut chunk_numbers = Vec::new();

    if let Some(chunks) = &meta.grounding_chunks {
        for chunk in chunks {
            let Some(web) = &chunk.web else {
                chunk_numbers.push(None);
                continue;
            };

            let citation = (web.title.clone(), web.uri.clone());
            let index = citations
                .iter()
                .position(|existing| existing == &citation)
                .unwrap_or_else(|| {
                    citations.push(citation);
                    citations.len() - 1
                });
            chunk_numbers.push(Some(index + 1));
        }
    }

    (citations, chunk_numbers)
}

fn format_citation_markers(numbers: impl IntoIterator<Item = usize>) -> String {
    numbers
        .into_iter()
        .map(|number| format!("[{number}]"))
        .collect()
}

/// Insert numbered `[N]` citation markers into Gemini grounded text.
#[must_use]
pub fn grounding_to_inline_citations(text: &str, meta: &GroundingMetadata) -> String {
    let (_, chunk_numbers) = grounding_citation_numbers(meta);
    let Some(supports) = meta.grounding_supports.as_ref() else {
        return text.to_string();
    };

    let char_len = text.chars().count();
    let mut insertions = BTreeMap::<usize, BTreeSet<usize>>::new();
    for support in supports {
        if support.segment.end_index > char_len {
            continue;
        }

        let refs = insertions.entry(support.segment.end_index).or_default();
        for chunk_index in &support.grounding_chunk_indices {
            if let Some(Some(number)) = chunk_numbers.get(*chunk_index) {
                refs.insert(*number);
            }
        }
    }

    if insertions.is_empty() {
        return text.to_string();
    }

    let mut cited = String::new();
    if let Some(numbers) = insertions.get(&0) {
        cited.push_str(&format_citation_markers(numbers.iter().copied()));
    }

    for (i, ch) in text.chars().enumerate() {
        cited.push(ch);
        if let Some(numbers) = insertions.get(&(i + 1)) {
            if !cited.chars().last().is_some_and(char::is_whitespace) {
                cited.push(' ');
            }
            cited.push_str(&format_citation_markers(numbers.iter().copied()));
        }
    }

    cited
}

/// Convert Gemini grounding metadata into the same markdown research shape
/// used by search-grounded Perplexity research.
pub fn save_research_with_grounding(
    workdir: &Path,
    topic: &str,
    content: &str,
    metadata: &GroundingMetadata,
) -> Result<PathBuf> {
    let mut doc = String::new();
    let cited_content = grounding_to_inline_citations(content, metadata);
    writeln!(doc, "# Research: {topic}\n")?;
    writeln!(
        doc,
        "> Generated via Gemini Google Search grounding — {}\n",
        chrono::Local::now().format("%Y-%m-%d")
    )?;
    writeln!(doc, "{cited_content}\n")?;

    let (citations, chunk_numbers) = grounding_citation_numbers(metadata);
    if !citations.is_empty() {
        writeln!(doc, "\n## Sources\n")?;
        for (i, (title, url)) in citations.iter().enumerate() {
            writeln!(doc, "{}. [{title}]({url})", i + 1)?;
        }
    }

    let has_queries = metadata
        .web_search_queries
        .as_ref()
        .is_some_and(|queries| !queries.is_empty());
    let has_supports = metadata
        .grounding_supports
        .as_ref()
        .is_some_and(|supports| !supports.is_empty());

    if has_queries || has_supports {
        writeln!(doc, "\n## Search Context\n")?;

        if let Some(queries) = &metadata.web_search_queries
            && !queries.is_empty()
        {
            writeln!(doc, "### Queries\n")?;
            for query in queries {
                writeln!(doc, "- {query}")?;
            }
            writeln!(doc)?;
        }

        if let Some(supports) = &metadata.grounding_supports
            && !supports.is_empty()
        {
            writeln!(doc, "### Supports\n")?;
            for support in supports {
                let refs = support
                    .grounding_chunk_indices
                    .iter()
                    .filter_map(|index| chunk_numbers.get(*index).and_then(|number| *number))
                    .collect::<BTreeSet<_>>();
                let segment = support.segment.text.trim();

                if refs.is_empty() {
                    writeln!(doc, "- {segment}")?;
                } else {
                    writeln!(
                        doc,
                        "- {segment} {}",
                        format_citation_markers(refs.into_iter())
                    )?;
                }
            }
        }
    }

    let path = research_dir(workdir).join(format!("{}.md", slug(topic)));
    std::fs::write(&path, doc)?;
    Ok(path)
}

/// Research mode determines what kind of output to produce.
#[derive(Debug, Clone, Copy)]
pub enum ResearchMode {
    /// Pure research on a topic → .roko/research/<slug>.md
    Topic,
    /// Optimize an implementation plan with research-backed techniques
    EnhancePlan,
    /// Optimize tasks for efficiency, parallelism, and model selection
    EnhanceTasks,
    /// Analyze execution episodes for self-learning insights
    AnalyzeExecution,
}

fn slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// List research artifacts.
pub fn list_research(workdir: &Path) -> Result<Vec<PathBuf>> {
    ensure_dirs(workdir)?;
    let dir = research_dir(workdir);
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

// ── Spend ─────────────────────────────────────────────────────────────

/// Model that a Perplexity Search API call's spend is recorded under: the
/// API runs no model.
const SEARCH_API_MODEL: &str = "perplexity-search";

/// Record what one research agent run cost, against the topic's research
/// task (bug-86ff56). Perplexity and Gemini report tokens but no dollar
/// amount, so such a run is priced from its tokens at the workspace's price
/// snapshot, else the model's configured rates, else the registry's
/// (bug-2dfd23, backlog 2114).
pub async fn record_run_spend(
    workdir: &Path,
    config: &RokoConfig,
    topic: &str,
    role: &str,
    provider: &str,
    model: &str,
    result: &roko_agent::AgentResult,
    started: Instant,
) {
    let mut usage = result.usage;
    let profile = roko_core::agent::resolve_model(config, model).profile;
    let snapshot = crate::dispatch_v2::pricing_snapshot(&config.pricing, workdir);
    crate::dispatch_v2::fill_usage_cost_from_pricing(
        &mut usage,
        snapshot.as_deref(),
        profile.as_ref(),
        model,
    );
    let call = AgentCapture {
        exit_code: i32::from(!result.success),
        output: String::new(),
        usage,
        model: model.to_string(),
        provider: provider.to_string(),
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    };
    let task_id = format!("research:topic:{}", topic.to_lowercase().replace(' ', "-"));
    AuthoringSpend::operation(workdir, &task_id, role)
        .record(&call)
        .await;
}

/// Record what `roko research search` cost, against the query's research
/// task: `requests` calls to the Perplexity Search API, which reports no
/// usage and bills [`PERPLEXITY_SEARCH_REQUEST_USD`] a request (bug-2dfd23).
pub async fn record_search_spend(
    workdir: &Path,
    query: &str,
    role: &str,
    requests: usize,
    duration_ms: u64,
) {
    let usage = roko_core::Usage {
        cost_usd: (requests as f64 * PERPLEXITY_SEARCH_REQUEST_USD) as f32,
        ..roko_core::Usage::default()
    };
    let call = AgentCapture {
        exit_code: 0,
        output: String::new(),
        usage,
        model: SEARCH_API_MODEL.to_string(),
        provider: "perplexity".to_string(),
        duration_ms,
    };
    let task_id = format!("research:search:{}", query.to_lowercase().replace(' ', "-"));
    AuthoringSpend::operation(workdir, &task_id, role)
        .record(&call)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_basic() {
        assert_eq!(
            slug("Git Worktree Best Practices"),
            "git-worktree-best-practices"
        );
    }

    #[test]
    fn build_topic_prompt() {
        let prompt = build_research_prompt(
            Path::new("/test"),
            "context engineering for coding agents",
            "",
            ResearchMode::Topic,
        );
        assert!(prompt.contains("context engineering"));
        assert!(prompt.contains("arXiv"));
        assert!(prompt.contains("[AUTHOR-YEAR]"));
    }

    #[test]
    fn research_prompt_perplexity() {
        let cfg = PerplexityConfig {
            academic_mode: true,
            search_recency_filter: "month".to_string(),
            search_domain_filter: vec!["arxiv.org".to_string()],
            return_images: false,
            return_related_questions: true,
            ..Default::default()
        };

        let (prompt, opts) = build_research_prompt_perplexity(
            Path::new("/test"),
            "transformer architectures",
            "",
            ResearchMode::Topic,
            &cfg,
        );

        // Perplexity-aware citation format is present.
        assert!(prompt.contains("[N]"), "expected [N] bracket notation");
        // "verify papers exist" instruction is removed.
        assert!(
            !prompt.to_lowercase().contains("verify papers exist"),
            "should not contain verify papers exist"
        );
        // [AUTHOR-YEAR] format is removed.
        assert!(
            !prompt.contains("[AUTHOR-YEAR]"),
            "should not contain [AUTHOR-YEAR]"
        );

        // SearchOptions are populated from config.
        assert_eq!(opts.search_mode.as_deref(), Some("academic"));
        assert_eq!(opts.search_recency_filter.as_deref(), Some("month"));
        assert_eq!(
            opts.search_domain_filter.as_deref(),
            Some(["arxiv.org".to_string()].as_slice())
        );
        assert_eq!(opts.return_related_questions, Some(true));
        assert_eq!(opts.return_images, Some(false));
    }

    #[test]
    fn research_gemini_grounding_prompt_enabled_when_model_configured() {
        let cfg = GeminiConfig {
            grounding_model: Some("gemini-3-flash-preview".to_string()),
            ..Default::default()
        };

        let (prompt, enable_grounding) = build_research_prompt_gemini(
            Path::new("/test"),
            "rust async runtimes",
            ResearchMode::Topic,
            &cfg,
        );

        assert!(enable_grounding);
        assert!(prompt.contains("rust async runtimes"));
        assert!(prompt.contains("[AUTHOR-YEAR]"));
    }

    #[test]
    fn research_gemini_grounding_saves_sources_and_supports() {
        use roko_agent::gemini::{
            GroundingChunk, GroundingMetadata, GroundingSupport, TextSegment, WebChunk,
        };

        let tmp = tempfile::tempdir().unwrap();
        ensure_dirs(tmp.path()).unwrap();

        let metadata = GroundingMetadata {
            web_search_queries: Some(vec!["Rust async runtimes benchmark".to_string()]),
            grounding_chunks: Some(vec![
                GroundingChunk {
                    web: Some(WebChunk {
                        uri: "https://tokio.rs".to_string(),
                        title: "Tokio".to_string(),
                    }),
                },
                GroundingChunk {
                    web: Some(WebChunk {
                        uri: "https://docs.rs/async-std".to_string(),
                        title: "async-std".to_string(),
                    }),
                },
            ]),
            grounding_supports: Some(vec![GroundingSupport {
                segment: TextSegment {
                    start_index: 0,
                    end_index: 32,
                    text: "Tokio remains the dominant runtime.".to_string(),
                },
                grounding_chunk_indices: vec![0],
                confidence_scores: Some(vec![0.93]),
            }]),
            search_entry_point: None,
        };

        let path = save_research_with_grounding(
            tmp.path(),
            "rust async runtimes",
            "Tokio remains the dominant runtime.",
            &metadata,
        )
        .unwrap();

        let doc = std::fs::read_to_string(&path).unwrap();
        assert!(doc.contains("# Research: rust async runtimes"));
        assert!(doc.contains("Gemini Google Search grounding"));
        assert!(doc.contains("## Sources"));
        assert!(doc.contains("[Tokio](https://tokio.rs)"));
        assert!(doc.contains("[async-std](https://docs.rs/async-std)"));
        assert!(doc.contains("## Search Context"));
        assert!(doc.contains("Rust async runtimes benchmark"));
        assert!(doc.contains("Tokio remains the dominant runtime. [1]"));
    }

    #[test]
    fn grounding_to_citations_extracts_unique_sources() {
        use roko_agent::gemini::{GroundingChunk, GroundingMetadata, WebChunk};

        let metadata = GroundingMetadata {
            web_search_queries: None,
            grounding_chunks: Some(vec![
                GroundingChunk {
                    web: Some(WebChunk {
                        uri: "https://tokio.rs".to_string(),
                        title: "Tokio".to_string(),
                    }),
                },
                GroundingChunk {
                    web: Some(WebChunk {
                        uri: "https://tokio.rs".to_string(),
                        title: "Tokio".to_string(),
                    }),
                },
                GroundingChunk { web: None },
            ]),
            grounding_supports: None,
            search_entry_point: None,
        };

        assert_eq!(
            grounding_to_citations(&metadata),
            vec![("Tokio".to_string(), "https://tokio.rs".to_string())]
        );
    }

    #[test]
    fn grounding_to_citations_inlines_grounding_supports() {
        use roko_agent::gemini::{
            GroundingChunk, GroundingMetadata, GroundingSupport, TextSegment, WebChunk,
        };

        let text = "Tokio remains dominant. async-std trails.";
        let metadata = GroundingMetadata {
            web_search_queries: None,
            grounding_chunks: Some(vec![
                GroundingChunk {
                    web: Some(WebChunk {
                        uri: "https://tokio.rs".to_string(),
                        title: "Tokio".to_string(),
                    }),
                },
                GroundingChunk {
                    web: Some(WebChunk {
                        uri: "https://docs.rs/async-std".to_string(),
                        title: "async-std".to_string(),
                    }),
                },
                GroundingChunk {
                    web: Some(WebChunk {
                        uri: "https://tokio.rs".to_string(),
                        title: "Tokio".to_string(),
                    }),
                },
            ]),
            grounding_supports: Some(vec![
                GroundingSupport {
                    segment: TextSegment {
                        start_index: 0,
                        end_index: 23,
                        text: "Tokio remains dominant.".to_string(),
                    },
                    grounding_chunk_indices: vec![0],
                    confidence_scores: Some(vec![0.97]),
                },
                GroundingSupport {
                    segment: TextSegment {
                        start_index: 24,
                        end_index: 41,
                        text: "async-std trails.".to_string(),
                    },
                    grounding_chunk_indices: vec![1, 2],
                    confidence_scores: Some(vec![0.81, 0.78]),
                },
            ]),
            search_entry_point: None,
        };

        assert_eq!(
            grounding_to_inline_citations(text, &metadata),
            "Tokio remains dominant. [1] async-std trails. [1][2]"
        );
    }

    #[test]
    fn ensure_dirs_creates() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_dirs(tmp.path()).unwrap();
        assert!(tmp.path().join(".roko/research").is_dir());
    }

    /// The rows of `workdir`'s `.roko/learn/costs.jsonl`.
    fn cost_rows(workdir: &Path) -> Vec<roko_learn::costs_db::CostRecord> {
        std::fs::read_to_string(workdir.join(".roko/learn/costs.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).expect("a cost record"))
            .collect()
    }

    /// bug-2dfd23: a `roko research search` call is recorded at the Search
    /// API's price per request, under the query's research task.
    #[tokio::test]
    async fn research_search_records_spend() {
        let tmp = tempfile::tempdir().unwrap();

        record_search_spend(tmp.path(), "Rust async traits", "researcher", 1, 420).await;

        let rows = cost_rows(tmp.path());
        assert_eq!(rows.len(), 1, "{rows:?}");
        let row = &rows[0];
        assert_eq!(row.task_id, "research:search:rust-async-traits");
        assert_eq!(row.role, "researcher");
        assert_eq!(row.provider, "perplexity");
        assert_eq!(row.model, SEARCH_API_MODEL);
        assert!(
            (row.cost_usd - PERPLEXITY_SEARCH_REQUEST_USD).abs() < 1e-6,
            "{row:?}"
        );
    }

    /// bug-2dfd23: a research run whose provider reported tokens but no
    /// dollar amount is priced at the registry's rates for its model.
    #[tokio::test]
    async fn research_runs_without_a_reported_cost_are_priced() {
        let tmp = tempfile::tempdir().unwrap();
        let mut config = RokoConfig::default();
        config.models.clear();
        let output = roko_core::Signal::builder(roko_core::Kind::AgentOutput)
            .body(roko_core::Body::text("findings"))
            .build();
        let usage = roko_core::Usage {
            input_tokens: 200_000,
            output_tokens: 100_000,
            ..roko_core::Usage::default()
        };
        let result = roko_agent::AgentResult::ok(output).with_usage(usage);

        record_run_spend(
            tmp.path(),
            &config,
            "agent memory",
            "researcher",
            "perplexity",
            "sonar",
            &result,
            Instant::now(),
        )
        .await;

        let rows = cost_rows(tmp.path());
        assert_eq!(rows.len(), 1, "{rows:?}");
        let pricing = roko_core::config::model_registry::builtin_pricing("sonar")
            .expect("the registry prices sonar");
        let expected =
            (200_000.0 * pricing.input_per_m + 100_000.0 * pricing.output_per_m) / 1_000_000.0;
        assert!((rows[0].cost_usd - expected).abs() < 1e-6, "{rows:?}");
        assert_eq!(rows[0].task_id, "research:topic:agent-memory");
    }
}
