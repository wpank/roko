//! Word-overlap clustering for `.roko/notes/` markdown files.
//!
//! Used by `roko plan generate --from-notes` to group related notes
//! before generating an implementation plan per cluster.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// A single parsed note from `.roko/notes/`.
#[derive(Debug, Clone)]
pub struct NoteEntry {
    /// Path to the source `.md` file.
    pub path: PathBuf,
    /// Full text content of the note (body, excluding metadata header).
    pub text: String,
    /// Tags extracted from the `**Tags:** ...` line, if present.
    pub tags: Vec<String>,
}

/// A cluster of related notes grouped by word overlap.
#[derive(Debug, Clone)]
pub struct NoteCluster {
    /// Notes in this cluster.
    pub notes: Vec<NoteEntry>,
    /// Theme string derived from the top-3 most frequent words.
    pub theme: String,
}

/// Stop words excluded from clustering comparisons.
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "to", "for", "in", "on", "of", "and", "or",
    "it", "this", "that", "with", "from", "but", "not", "be", "as", "at", "by", "has", "have",
    "had", "do", "does", "did", "will", "would", "could", "should",
];

/// Load all `.md` notes from `notes_dir`, optionally filtering by tag.
///
/// Returns an empty `Vec` if the directory does not exist or contains no notes.
pub fn load_notes(notes_dir: &Path, tag_filter: Option<&str>) -> Vec<NoteEntry> {
    let entries = match std::fs::read_dir(notes_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut notes: Vec<NoteEntry> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "warning: skipping {}: {e}",
                    path.display()
                );
                continue;
            }
        };

        let (tags, text) = parse_note(&content);

        if let Some(filter) = tag_filter {
            let filter_lower = filter.to_lowercase();
            if !tags.iter().any(|t| t.to_lowercase() == filter_lower) {
                continue;
            }
        }

        notes.push(NoteEntry { path, text, tags });
    }

    // Sort by filename (chronological, since filenames are YYYY-MM-DD-HHMMSS.md)
    notes.sort_by(|a, b| a.path.file_name().cmp(&b.path.file_name()));
    notes
}

/// Parse tags and body text from a note's raw markdown content.
fn parse_note(content: &str) -> (Vec<String>, String) {
    let mut tags = Vec::new();
    let mut body_lines = Vec::new();
    let mut past_header = false;

    for line in content.lines() {
        if !past_header {
            if let Some(rest) = line.strip_prefix("**Tags:**") {
                tags = rest
                    .split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
                continue;
            }
            // Skip the `# Note` header and `**Date:**` line
            if line.starts_with("# Note") || line.starts_with("**Date:**") {
                continue;
            }
            // First non-metadata, non-empty line marks the start of body
            if !line.trim().is_empty() {
                past_header = true;
            }
        }
        if past_header {
            body_lines.push(line);
        }
    }

    (tags, body_lines.join("\n"))
}

/// Extract significant (non-stop) words from text.
fn significant_words(text: &str) -> HashSet<String> {
    let stops: HashSet<&str> = STOP_WORDS.iter().copied().collect();
    text.split_whitespace()
        .map(|w| w.to_lowercase())
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| w.len() > 1 && !stops.contains(w.as_str()))
        .collect()
}

/// Cluster notes by shared significant words using union-find.
///
/// Two notes belong together if they share more than 2 significant words.
/// Singleton clusters are created for notes with no partners.
pub fn cluster_notes(notes: Vec<NoteEntry>) -> Vec<NoteCluster> {
    if notes.is_empty() {
        return Vec::new();
    }

    let n = notes.len();
    let word_sets: Vec<HashSet<String>> = notes.iter().map(|n| significant_words(&n.text)).collect();

    // Union-find
    let mut parent: Vec<usize> = (0..n).collect();

    fn find(parent: &mut [usize], i: usize) -> usize {
        let mut root = i;
        while parent[root] != root {
            root = parent[root];
        }
        // Path compression
        let mut curr = i;
        while parent[curr] != root {
            let next = parent[curr];
            parent[curr] = root;
            curr = next;
        }
        root
    }

    fn union(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[rb] = ra;
        }
    }

    // Merge notes that share >2 significant words
    for i in 0..n {
        for j in (i + 1)..n {
            let shared = word_sets[i].intersection(&word_sets[j]).count();
            if shared > 2 {
                union(&mut parent, i, j);
            }
        }
    }

    // Group by root
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        groups.entry(root).or_default().push(i);
    }

    // Build clusters with theme from top-3 frequent words
    let mut clusters: Vec<NoteCluster> = groups
        .into_values()
        .map(|indices| {
            let cluster_notes: Vec<NoteEntry> =
                indices.iter().map(|&i| notes[i].clone()).collect();

            // Count word frequency across all notes in cluster
            let mut freq: HashMap<String, usize> = HashMap::new();
            for &idx in &indices {
                for word in &word_sets[idx] {
                    *freq.entry(word.clone()).or_default() += 1;
                }
            }

            let mut words_by_freq: Vec<(String, usize)> = freq.into_iter().collect();
            words_by_freq.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            let theme: String = words_by_freq
                .iter()
                .take(3)
                .map(|(w, _)| w.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            NoteCluster {
                notes: cluster_notes,
                theme,
            }
        })
        .collect();

    // Stable ordering: sort by first note's filename
    clusters.sort_by(|a, b| {
        a.notes[0]
            .path
            .file_name()
            .cmp(&b.notes[0].path.file_name())
    });

    clusters
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_note_with_tags() {
        let content = "# Note\n\n**Date:** 2026-08-10 14:30:00\n**Tags:** cli, ux\n\nSome body text here.";
        let (tags, text) = parse_note(content);
        assert_eq!(tags, vec!["cli", "ux"]);
        assert!(text.contains("Some body text here."));
    }

    #[test]
    fn test_parse_note_without_tags() {
        let content = "# Note\n\n**Date:** 2026-08-10 14:30:00\n\nJust text, no tags.";
        let (tags, text) = parse_note(content);
        assert!(tags.is_empty());
        assert!(text.contains("Just text, no tags."));
    }

    #[test]
    fn test_significant_words_excludes_stops() {
        let words = significant_words("the quick brown fox is a fast animal");
        assert!(words.contains("quick"));
        assert!(words.contains("brown"));
        assert!(!words.contains("the"));
        assert!(!words.contains("is"));
        assert!(!words.contains("a"));
    }

    #[test]
    fn test_cluster_empty() {
        let clusters = cluster_notes(vec![]);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_cluster_singletons() {
        let notes = vec![
            NoteEntry {
                path: PathBuf::from("a.md"),
                text: "alpha beta gamma".to_string(),
                tags: vec![],
            },
            NoteEntry {
                path: PathBuf::from("b.md"),
                text: "delta epsilon zeta".to_string(),
                tags: vec![],
            },
        ];
        let clusters = cluster_notes(notes);
        // No shared words, so each note is its own cluster
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn test_cluster_merge() {
        let notes = vec![
            NoteEntry {
                path: PathBuf::from("a.md"),
                text: "cursor support agent routing model".to_string(),
                tags: vec![],
            },
            NoteEntry {
                path: PathBuf::from("b.md"),
                text: "cursor agent model selection improvement".to_string(),
                tags: vec![],
            },
        ];
        let clusters = cluster_notes(notes);
        // 3 shared words: cursor, agent, model -> merged
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].notes.len(), 2);
    }

    #[test]
    fn test_load_notes_missing_dir() {
        let notes = load_notes(Path::new("/nonexistent/dir"), None);
        assert!(notes.is_empty());
    }
}
