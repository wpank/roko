//! Skill library — a registry of reusable capabilities the agent can invoke.
//!
//! A [`Skill`] captures a named capability with a prompt template, the tools
//! it depends on, and example input/output pairs. Skills accumulate
//! lightweight usage telemetry (`usage_count`, `success_rate`) each time they
//! are invoked so the library can surface the most reliable patterns to
//! future prompts.
//!
//! The [`SkillLibrary`] is an in-memory, JSON-file-backed registry. Writes
//! to the in-memory map are guarded by a [`parking_lot::RwLock`]; persistence
//! uses [`tokio::fs`] with a tempfile+rename to keep the on-disk store
//! consistent under concurrent writers.
//!
//! # Example
//!
//! ```no_run
//! # async fn run() -> Result<(), roko_learn::skill_library::SkillLibraryError> {
//! use roko_learn::skill_library::{Skill, SkillLibrary};
//!
//! let library = SkillLibrary::new("/tmp/skills.json").await?;
//! let skill = Skill::new(
//!     "summarize_diff",
//!     "Summarize a git diff into a short changelog entry.",
//!     "You are given a diff. Produce a 1-2 sentence changelog entry.",
//! );
//! library.register(&skill).await?;
//! library.record_use("summarize_diff", true).await?;
//! # Ok(()) }
//! ```

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex as AsyncMutex;

/// Errors produced by [`SkillLibrary`].
#[derive(Debug, Error)]
pub enum SkillLibraryError {
    /// A skill with the requested name already exists in the library.
    #[error("skill '{0}' is already registered")]
    Duplicate(String),
    /// No skill with the requested name exists.
    #[error("skill '{0}' is not registered")]
    NotFound(String),
    /// I/O error while reading or writing the persistence file.
    #[error("skill library I/O error: {0}")]
    Io(#[from] io::Error),
    /// JSON (de)serialization error.
    #[error("skill library serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// A reusable capability the agent can invoke.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    /// Unique, human-readable identifier for the skill (`snake_case` preferred).
    pub name: String,
    /// One-line description of what the skill does.
    pub summary: String,
    /// Prompt template injected when the skill is selected.
    pub prompt_template: String,
    /// Names of tools this skill expects the caller to expose.
    #[serde(default)]
    pub required_tools: Vec<String>,
    /// Illustrative inputs the skill was designed for.
    #[serde(default)]
    pub example_inputs: Vec<String>,
    /// Illustrative outputs corresponding to `example_inputs`.
    #[serde(default)]
    pub example_outputs: Vec<String>,
    /// Free-form tags used by [`SkillLibrary::search_by_tag`].
    #[serde(default)]
    pub tags: Vec<String>,
    /// Smoothed success rate in `[0.0, 1.0]`. Starts at `0.0`.
    #[serde(default)]
    pub success_rate: f64,
    /// Number of times [`SkillLibrary::record_use`] has been called.
    #[serde(default)]
    pub usage_count: u64,
}

impl Skill {
    /// Construct a new skill with defaults for telemetry + example fields.
    pub fn new(
        name: impl Into<String>,
        summary: impl Into<String>,
        prompt_template: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            summary: summary.into(),
            prompt_template: prompt_template.into(),
            required_tools: Vec::new(),
            example_inputs: Vec::new(),
            example_outputs: Vec::new(),
            tags: Vec::new(),
            success_rate: 0.0,
            usage_count: 0,
        }
    }

    /// Builder helper: attach required tool names.
    #[must_use]
    pub fn with_required_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.required_tools = tools.into_iter().map(Into::into).collect();
        self
    }

    /// Builder helper: attach example input/output pairs.
    #[must_use]
    pub fn with_examples<I, S1, S2>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (S1, S2)>,
        S1: Into<String>,
        S2: Into<String>,
    {
        for (input, output) in pairs {
            self.example_inputs.push(input.into());
            self.example_outputs.push(output.into());
        }
        self
    }

    /// Builder helper: attach tags.
    #[must_use]
    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }
}

/// In-memory, JSON-backed registry of [`Skill`] records.
#[derive(Debug)]
pub struct SkillLibrary {
    path: PathBuf,
    skills: RwLock<BTreeMap<String, Skill>>,
    write_lock: AsyncMutex<()>,
}

impl SkillLibrary {
    /// Open (or create) a skill library at `path`. If the file exists it is
    /// deserialized; if it does not, an empty library is returned and will
    /// be created on the next mutating call.
    pub async fn new(path: impl AsRef<Path>) -> Result<Self, SkillLibraryError> {
        let path = path.as_ref().to_path_buf();
        let skills = match tokio::fs::read(&path).await {
            Ok(bytes) if bytes.is_empty() => BTreeMap::new(),
            Ok(bytes) => {
                let list: Vec<Skill> = serde_json::from_slice(&bytes)?;
                list.into_iter().map(|s| (s.name.clone(), s)).collect()
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => BTreeMap::new(),
            Err(err) => return Err(SkillLibraryError::Io(err)),
        };
        Ok(Self {
            path,
            skills: RwLock::new(skills),
            write_lock: AsyncMutex::new(()),
        })
    }

    /// Register a new skill. Returns [`SkillLibraryError::Duplicate`] if a
    /// skill with the same name is already present.
    pub async fn register(&self, skill: &Skill) -> Result<(), SkillLibraryError> {
        {
            let mut guard = self.skills.write();
            if guard.contains_key(&skill.name) {
                return Err(SkillLibraryError::Duplicate(skill.name.clone()));
            }
            guard.insert(skill.name.clone(), skill.clone());
        }
        self.persist().await
    }

    /// Retrieve a cloned snapshot of the skill with the given name.
    pub fn get(&self, name: &str) -> Option<Skill> {
        self.skills.read().get(name).cloned()
    }

    /// Return all skills in the library, sorted by name.
    pub fn list(&self) -> Vec<Skill> {
        self.skills.read().values().cloned().collect()
    }

    /// Number of registered skills.
    pub fn len(&self) -> usize {
        self.skills.read().len()
    }

    /// Whether the library has zero registered skills.
    pub fn is_empty(&self) -> bool {
        self.skills.read().is_empty()
    }

    /// Record an invocation of a skill. Updates `usage_count` and folds the
    /// outcome into a rolling mean `success_rate`.
    ///
    /// Returns [`SkillLibraryError::NotFound`] if `name` is not registered.
    #[allow(clippy::significant_drop_tightening)]
    pub async fn record_use(
        &self,
        name: &str,
        success: bool,
    ) -> Result<(), SkillLibraryError> {
        {
            let mut guard = self.skills.write();
            let Some(skill) = guard.get_mut(name) else {
                return Err(SkillLibraryError::NotFound(name.to_string()));
            };
            let prior = skill.usage_count;
            let outcome = f64::from(u8::from(success));
            // Running mean: new_mean = (prior_mean * n + outcome) / (n + 1).
            // f64 is wide enough for all realistic counters; cast is lossy
            // only beyond 2^53 which we clamp at below.
            #[allow(clippy::cast_precision_loss)]
            let prior_f = prior as f64;
            skill.success_rate =
                (skill.success_rate.mul_add(prior_f, outcome)) / (prior_f + 1.0);
            skill.usage_count = prior.saturating_add(1);
        }
        self.persist().await
    }

    /// Return all skills that carry `tag` in their `tags` vector.
    pub fn search_by_tag(&self, tag: &str) -> Vec<Skill> {
        self.skills
            .read()
            .values()
            .filter(|s| s.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    /// On-disk path backing this library.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Serialize the in-memory map to the on-disk file atomically.
    ///
    /// Writes are serialized via an async mutex so that the tempfile+rename
    /// dance never races against itself under concurrent writers.
    async fn persist(&self) -> Result<(), SkillLibraryError> {
        let _guard = self.write_lock.lock().await;
        let snapshot: Vec<Skill> = self.skills.read().values().cloned().collect();
        let bytes = serde_json::to_vec_pretty(&snapshot)?;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        let tmp = self.path.with_extension("json.tmp");
        tokio::fs::write(&tmp, &bytes).await?;
        tokio::fs::rename(&tmp, &self.path).await?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn sample_skill(name: &str) -> Skill {
        Skill::new(
            name,
            "sample summary",
            "You are a helpful assistant. Do the thing.",
        )
        .with_required_tools(["read", "write"])
        .with_tags(["sample", "test"])
        .with_examples([("in-a", "out-a"), ("in-b", "out-b")])
    }

    #[tokio::test]
    async fn register_and_get_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("skills.json");
        let library = SkillLibrary::new(&path).await.unwrap();

        let skill = sample_skill("alpha");
        library.register(&skill).await.unwrap();

        let fetched = library.get("alpha").unwrap();
        assert_eq!(fetched.name, "alpha");
        assert_eq!(fetched.required_tools, vec!["read", "write"]);
        assert_eq!(fetched.example_inputs.len(), 2);
        assert_eq!(fetched.example_outputs.len(), 2);
        assert_eq!(fetched.usage_count, 0);
        assert!((fetched.success_rate - 0.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn register_rejects_duplicates() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("skills.json");
        let library = SkillLibrary::new(&path).await.unwrap();

        library.register(&sample_skill("dup")).await.unwrap();
        let err = library
            .register(&sample_skill("dup"))
            .await
            .expect_err("duplicate should fail");
        assert!(matches!(err, SkillLibraryError::Duplicate(name) if name == "dup"));
        assert_eq!(library.len(), 1);
    }

    #[tokio::test]
    async fn list_returns_all_sorted() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("skills.json");
        let library = SkillLibrary::new(&path).await.unwrap();

        library.register(&sample_skill("gamma")).await.unwrap();
        library.register(&sample_skill("alpha")).await.unwrap();
        library.register(&sample_skill("beta")).await.unwrap();

        let names: Vec<String> = library.list().into_iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);
        assert_eq!(library.len(), 3);
        assert!(!library.is_empty());
    }

    #[tokio::test]
    async fn record_use_updates_counters_and_rate() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("skills.json");
        let library = SkillLibrary::new(&path).await.unwrap();
        library.register(&sample_skill("s1")).await.unwrap();

        library.record_use("s1", true).await.unwrap();
        library.record_use("s1", true).await.unwrap();
        library.record_use("s1", false).await.unwrap();
        library.record_use("s1", true).await.unwrap();

        let s = library.get("s1").unwrap();
        assert_eq!(s.usage_count, 4);
        // 3 successes / 4 attempts
        assert!((s.success_rate - 0.75).abs() < 1e-9);
    }

    #[tokio::test]
    async fn record_use_missing_returns_not_found() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("skills.json");
        let library = SkillLibrary::new(&path).await.unwrap();

        let err = library
            .record_use("ghost", true)
            .await
            .expect_err("missing skill should fail");
        assert!(matches!(err, SkillLibraryError::NotFound(name) if name == "ghost"));
    }

    #[tokio::test]
    async fn persist_and_reload_preserves_state() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nested").join("skills.json");

        {
            let library = SkillLibrary::new(&path).await.unwrap();
            library.register(&sample_skill("persist")).await.unwrap();
            library.record_use("persist", true).await.unwrap();
            library.record_use("persist", false).await.unwrap();
        }

        let reloaded = SkillLibrary::new(&path).await.unwrap();
        let s = reloaded.get("persist").unwrap();
        assert_eq!(s.usage_count, 2);
        assert!((s.success_rate - 0.5).abs() < 1e-9);
        assert_eq!(s.required_tools, vec!["read", "write"]);
        assert_eq!(s.tags, vec!["sample", "test"]);
    }

    #[tokio::test]
    async fn search_by_tag_filters_correctly() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("skills.json");
        let library = SkillLibrary::new(&path).await.unwrap();

        let a = Skill::new("a", "", "").with_tags(["rust", "fs"]);
        let b = Skill::new("b", "", "").with_tags(["rust"]);
        let c = Skill::new("c", "", "").with_tags(["python"]);
        library.register(&a).await.unwrap();
        library.register(&b).await.unwrap();
        library.register(&c).await.unwrap();

        let rust = library.search_by_tag("rust");
        let names: Vec<String> = rust.into_iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["a", "b"]);

        let fs = library.search_by_tag("fs");
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].name, "a");

        let none = library.search_by_tag("ruby");
        assert!(none.is_empty());
    }

    #[tokio::test]
    async fn missing_skill_returns_none() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("skills.json");
        let library = SkillLibrary::new(&path).await.unwrap();

        assert!(library.get("nope").is_none());
        assert!(library.is_empty());
        assert_eq!(library.len(), 0);
        assert!(library.list().is_empty());
    }

    #[tokio::test]
    async fn new_on_missing_file_returns_empty_library() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("does-not-exist.json");

        let library = SkillLibrary::new(&path).await.unwrap();
        assert!(library.is_empty());
        assert_eq!(library.path(), path.as_path());
    }

    #[tokio::test]
    async fn concurrent_register_produces_consistent_state() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("skills.json");
        let library = Arc::new(SkillLibrary::new(&path).await.unwrap());

        let mut handles = Vec::new();
        for i in 0..16 {
            let lib = Arc::clone(&library);
            handles.push(tokio::spawn(async move {
                let s = Skill::new(
                    format!("skill_{i:02}"),
                    "summary",
                    "template",
                );
                lib.register(&s).await
            }));
        }
        for h in handles {
            h.await.unwrap().unwrap();
        }

        assert_eq!(library.len(), 16);
        let names: Vec<String> = library.list().into_iter().map(|s| s.name).collect();
        assert_eq!(names.first().map(String::as_str), Some("skill_00"));
        assert_eq!(names.last().map(String::as_str), Some("skill_15"));

        // Reload from disk to confirm every concurrent write reached the file.
        let reloaded = SkillLibrary::new(&path).await.unwrap();
        assert_eq!(reloaded.len(), 16);
    }

    #[tokio::test]
    async fn concurrent_record_use_tracks_every_outcome() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("skills.json");
        let library = Arc::new(SkillLibrary::new(&path).await.unwrap());
        library
            .register(&Skill::new("race", "sum", "tmpl"))
            .await
            .unwrap();

        let mut handles = Vec::new();
        for i in 0..20 {
            let lib = Arc::clone(&library);
            handles.push(tokio::spawn(async move {
                lib.record_use("race", i % 2 == 0).await
            }));
        }
        for h in handles {
            h.await.unwrap().unwrap();
        }
        let s = library.get("race").unwrap();
        assert_eq!(s.usage_count, 20);
        // 10 of 20 were successes → 0.5
        assert!((s.success_rate - 0.5).abs() < 1e-9);
    }

    #[tokio::test]
    async fn corrupt_file_surfaces_serde_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("broken.json");
        tokio::fs::write(&path, b"not valid json").await.unwrap();

        let err = SkillLibrary::new(&path)
            .await
            .expect_err("corrupt file should error");
        assert!(matches!(err, SkillLibraryError::Serde(_)));
    }

    #[tokio::test]
    async fn builder_helpers_compose() {
        let skill = Skill::new("builder", "sum", "tmpl")
            .with_required_tools(vec!["a", "b", "c"])
            .with_tags(vec!["x"])
            .with_examples(vec![("q", "r")]);
        assert_eq!(skill.required_tools.len(), 3);
        assert_eq!(skill.tags, vec!["x"]);
        assert_eq!(skill.example_inputs, vec!["q"]);
        assert_eq!(skill.example_outputs, vec!["r"]);
    }
}
