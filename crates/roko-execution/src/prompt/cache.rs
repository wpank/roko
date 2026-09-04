//! Prompt cache handle — wraps the pre-loaded prompt context data.
//!
//! The actual `PromptCache` struct (neuro entries, episodes, playbooks,
//! effectiveness) lives in `roko-cli`'s dispatch/prompt_cache.rs today.
//! This handle provides the layer-3 contract for cache lifecycle management.

use std::path::Path;
use std::time::{Duration, Instant};

use roko_learn::episode_logger::Episode;
use roko_learn::playbook::Playbook;
use roko_learn::section_effect::SectionEffectivenessRegistry;
use roko_neuro::KnowledgeEntry;

/// Default maximum age before the cache is considered stale.
const DEFAULT_MAX_AGE: Duration = Duration::from_mins(5);

/// Pre-loaded prompt context data.
///
/// All fields are read-only snapshots taken at `built_at`. The cache is
/// intentionally cheap to clone (inner vecs are behind `Arc` when shared
/// across tasks, but the cache itself is typically wrapped in `Arc` by the
/// caller).
#[derive(Debug, Clone)]
pub struct PromptCacheHandle {
    /// Knowledge entries loaded from the neuro store.
    pub neuro_entries: Vec<KnowledgeEntry>,
    /// Episodes loaded from JSONL logs.
    pub episodes: Vec<Episode>,
    /// Playbooks loaded from the playbook directory.
    pub playbooks: Vec<Playbook>,
    /// Section effectiveness registry loaded from disk.
    pub effectiveness: SectionEffectivenessRegistry,
    /// Instant when this cache was built.
    built_at: Instant,
    /// Configured staleness threshold.
    max_age: Duration,
}

impl PromptCacheHandle {
    /// Load all prompt context data from disk.
    ///
    /// This performs all the I/O that would otherwise happen per-task:
    /// neuro knowledge query, episode JSONL scan, playbook iteration, and
    /// section effectiveness registry load.
    ///
    /// Individual load failures are silently ignored (the corresponding
    /// vec/registry will be empty), matching existing fallback behavior.
    pub fn load(workdir: &Path) -> Self {
        let now = Instant::now();

        let neuro_entries = load_neuro_entries(workdir);
        let episodes = load_episodes(workdir);
        let playbooks = load_playbooks(workdir);
        let effectiveness = load_effectiveness(workdir);

        tracing::debug!(
            neuro = neuro_entries.len(),
            episodes = episodes.len(),
            playbooks = playbooks.len(),
            elapsed_ms = now.elapsed().as_millis(),
            "prompt cache loaded"
        );

        Self {
            neuro_entries,
            episodes,
            playbooks,
            effectiveness,
            built_at: now,
            max_age: DEFAULT_MAX_AGE,
        }
    }

    /// Returns `true` if the cache has exceeded its maximum age.
    pub fn is_stale(&self) -> bool {
        self.built_at.elapsed() > self.max_age
    }

    /// Override the maximum age for staleness checks.
    #[must_use]
    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    /// Age of this cache snapshot.
    pub fn age(&self) -> Duration {
        self.built_at.elapsed()
    }

    /// Create an empty cache for testing.
    pub fn empty() -> Self {
        Self {
            neuro_entries: Vec::new(),
            episodes: Vec::new(),
            playbooks: Vec::new(),
            effectiveness: SectionEffectivenessRegistry::default(),
            built_at: Instant::now(),
            max_age: DEFAULT_MAX_AGE,
        }
    }
}

// ---- Loaders ---------------------------------------------------------------

fn load_neuro_entries(workdir: &Path) -> Vec<KnowledgeEntry> {
    let store = roko_neuro::KnowledgeStore::for_workdir(workdir);
    store.query("", 500).unwrap_or_default()
}

fn load_episodes(workdir: &Path) -> Vec<Episode> {
    let mut episodes = Vec::new();
    let path = roko_learn::runtime_feedback::resolve_project_episode_path(workdir);
    if let Ok(file) = std::fs::File::open(&path) {
        let reader = std::io::BufReader::new(file);
        for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(episode) = serde_json::from_str::<Episode>(trimmed) {
                episodes.push(episode);
            }
        }
    }
    episodes
}

fn load_playbooks(workdir: &Path) -> Vec<Playbook> {
    let root = workdir.join(".roko").join("learn").join("playbooks");
    let mut playbooks = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path)
                && let Ok(playbook) = serde_json::from_str::<Playbook>(&text)
            {
                playbooks.push(playbook);
            }
        }
    }
    playbooks
}

fn load_effectiveness(workdir: &Path) -> SectionEffectivenessRegistry {
    let path = workdir.join(roko_learn::section_effect::DEFAULT_SECTION_EFFECTS_PATH);
    SectionEffectivenessRegistry::load_or_new(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = PromptCacheHandle::load(tmp.path());
        assert!(cache.neuro_entries.is_empty());
        assert!(cache.episodes.is_empty());
        assert!(cache.playbooks.is_empty());
        assert!(!cache.is_stale());
    }

    #[test]
    fn staleness_check() {
        let cache = PromptCacheHandle::empty().with_max_age(Duration::from_millis(0));
        assert!(cache.is_stale());
    }

    #[test]
    fn empty_cache_is_fresh() {
        let cache = PromptCacheHandle::empty();
        assert!(!cache.is_stale());
    }
}
