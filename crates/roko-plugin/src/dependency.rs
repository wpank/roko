//! Plugin dependency resolution and duplicate detection.
//!
//! Provides topological ordering of plugins based on their declared
//! dependencies and detects version conflicts or duplicate plugin names
//! across a set of discovered manifests.
//!
//! # Resolution algorithm
//!
//! [`resolve_plugin_dependencies`] performs a Kahn-style topological sort
//! over the dependency graph implied by [`PluginManifestFile::dependencies`].
//! Before sorting it checks for:
//!
//! - Missing dependencies (a plugin requires a name that is not in the input set).
//! - Version conflicts (a dependency constraint that the discovered version does
//!   not satisfy).
//! - Cycles (two or more plugins that transitively depend on each other).
//!
//! # Duplicate detection
//!
//! [`detect_duplicate_plugins`] groups manifests by `plugin.name` and reports
//! every name that appears more than once, along with the conflicting versions.

use std::collections::{HashMap, HashSet, VecDeque};

use roko_core::{Result, RokoError};

use crate::manifest::PluginManifestFile;

// ─── Version helpers ──────────────────────────────────────────────────────

/// Parse a semver string `"major.minor.patch"` into a comparable tuple.
///
/// Non-conforming strings fall back to `(0, 0, 0)` so they sort last.
fn parse_version(v: &str) -> (u64, u64, u64) {
    let mut parts = v.splitn(3, '.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

/// Check whether `actual` satisfies the `constraint` version.
///
/// The constraint is treated as a minimum version requirement: the actual
/// version must be >= the constraint (compared numerically per semver component).
fn version_satisfies(actual: &str, constraint: &str) -> bool {
    parse_version(actual) >= parse_version(constraint)
}

// ─── ResolvedPlugin ───────────────────────────────────────────────────────

/// A plugin that has been placed in the resolved load order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlugin {
    /// Plugin name (from `manifest.plugin.name`).
    pub name: String,
    /// Plugin version (from `manifest.plugin.version`).
    pub version: String,
    /// Zero-based position in the load order.
    pub load_order: usize,
    /// Names of plugins this plugin depends on (already loaded before it).
    pub depends_on: Vec<String>,
}

// ─── DuplicateReport ──────────────────────────────────────────────────────

/// Report of a plugin name that appears in multiple manifests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateReport {
    /// The duplicated plugin name.
    pub name: String,
    /// All versions discovered for this name.
    pub versions: Vec<String>,
    /// Number of manifests with this name.
    pub count: usize,
}

// ─── resolve_plugin_dependencies ──────────────────────────────────────────

/// Resolve plugin load order via topological sort and validate version constraints.
///
/// Given a set of discovered plugin manifests, this function:
///
/// 1. Checks for version conflicts — every declared dependency must have its
///    minimum version constraint satisfied by the discovered version.
/// 2. Checks for missing dependencies — every declared dependency name must be
///    present in the input set.
/// 3. Performs a Kahn-style topological sort to determine safe load order
///    (dependencies before dependents).
/// 4. Detects cycles — if the sort cannot consume all nodes, a cycle exists.
///
/// # Errors
///
/// Returns [`RokoError::invalid`] if any of the checks above fail.
pub fn resolve_plugin_dependencies(
    manifests: &[PluginManifestFile],
) -> Result<Vec<ResolvedPlugin>> {
    if manifests.is_empty() {
        return Ok(Vec::new());
    }

    // Build name→manifest index. If there are duplicates, take the first one
    // (callers should run `detect_duplicate_plugins` separately if they want
    // to warn about that).
    let mut by_name: HashMap<&str, &PluginManifestFile> = HashMap::new();
    for manifest in manifests {
        by_name
            .entry(manifest.plugin.name.as_str())
            .or_insert(manifest);
    }

    // ── Phase 1: validate dependencies exist and versions satisfy ────────

    for manifest in manifests {
        for dep in &manifest.dependencies {
            let Some(dep_manifest) = by_name.get(dep.name.as_str()) else {
                return Err(RokoError::invalid(format!(
                    "plugin '{}' depends on '{}' which is not in the discovered set",
                    manifest.plugin.name, dep.name
                )));
            };

            if let Some(ref constraint) = dep.version {
                if !version_satisfies(&dep_manifest.plugin.version, constraint) {
                    return Err(RokoError::invalid(format!(
                        "plugin '{}' requires '{}' >= {}, but found {}",
                        manifest.plugin.name, dep.name, constraint, dep_manifest.plugin.version,
                    )));
                }
            }
        }
    }

    // ── Phase 2: topological sort (Kahn's algorithm) ─────────────────────

    // Compute in-degree for each plugin.
    let names: Vec<&str> = by_name.keys().copied().collect();
    let mut in_degree: HashMap<&str, usize> = names.iter().map(|n| (*n, 0usize)).collect();
    let mut dependents: HashMap<&str, Vec<&str>> = names.iter().map(|n| (*n, Vec::new())).collect();

    for manifest in manifests {
        // Skip duplicates (only process the first instance per name).
        if !std::ptr::eq(by_name[manifest.plugin.name.as_str()], manifest) {
            continue;
        }
        for dep in &manifest.dependencies {
            if let Some(count) = in_degree.get_mut(manifest.plugin.name.as_str()) {
                *count += 1;
            }
            if let Some(deps) = dependents.get_mut(dep.name.as_str()) {
                deps.push(manifest.plugin.name.as_str());
            }
        }
    }

    // Seed the queue with plugins that have no dependencies, sorted for determinism.
    let mut sorted_seed: Vec<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(name, _)| *name)
        .collect();
    sorted_seed.sort_unstable();
    let mut queue: VecDeque<&str> = sorted_seed.into_iter().collect();

    let mut resolved: Vec<ResolvedPlugin> = Vec::with_capacity(by_name.len());
    let mut visited: HashSet<&str> = HashSet::new();

    while let Some(name) = queue.pop_front() {
        if !visited.insert(name) {
            continue;
        }

        let manifest = by_name[name];
        let depends_on: Vec<String> = manifest
            .dependencies
            .iter()
            .map(|d| d.name.clone())
            .collect();

        resolved.push(ResolvedPlugin {
            name: name.to_string(),
            version: manifest.plugin.version.clone(),
            load_order: resolved.len(),
            depends_on,
        });

        // Collect and sort next nodes for determinism.
        let mut next_nodes: Vec<&str> = Vec::new();
        if let Some(downstream) = dependents.get(name) {
            for &dependent in downstream {
                if let Some(count) = in_degree.get_mut(dependent) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        next_nodes.push(dependent);
                    }
                }
            }
        }
        next_nodes.sort_unstable();
        queue.extend(next_nodes);
    }

    // If we didn't resolve all plugins, there's a cycle.
    if resolved.len() < by_name.len() {
        let unresolved: Vec<String> = by_name
            .keys()
            .filter(|name| !visited.contains(**name))
            .map(|name| (*name).to_string())
            .collect();
        return Err(RokoError::invalid(format!(
            "dependency cycle detected among plugins: {}",
            unresolved.join(", ")
        )));
    }

    Ok(resolved)
}

// ─── detect_duplicate_plugins ─────────────────────────────────────────────

/// Detect plugin names that appear in multiple manifests.
///
/// Returns a [`DuplicateReport`] for each name that appears more than once,
/// listing all discovered versions and the total count.
pub fn detect_duplicate_plugins(manifests: &[PluginManifestFile]) -> Vec<DuplicateReport> {
    let mut groups: HashMap<&str, Vec<&str>> = HashMap::new();
    for manifest in manifests {
        groups
            .entry(manifest.plugin.name.as_str())
            .or_default()
            .push(manifest.plugin.version.as_str());
    }

    let mut reports: Vec<DuplicateReport> = groups
        .into_iter()
        .filter(|(_, versions)| versions.len() > 1)
        .map(|(name, versions)| DuplicateReport {
            name: name.to_string(),
            count: versions.len(),
            versions: versions.into_iter().map(String::from).collect(),
        })
        .collect();

    // Sort for deterministic output.
    reports.sort_by(|a, b| a.name.cmp(&b.name));
    reports
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{PluginDependency, PluginMeta};

    /// Helper: build a minimal manifest with the given name, version, and deps.
    fn make_manifest(
        name: &str,
        version: &str,
        deps: Vec<(&str, Option<&str>)>,
    ) -> PluginManifestFile {
        PluginManifestFile {
            plugin: PluginMeta {
                name: name.to_string(),
                version: version.to_string(),
                description: None,
                author: None,
                license: None,
                tier: Default::default(),
                enabled: true,
            },
            prompts: Vec::new(),
            profiles: Vec::new(),
            tools: Vec::new(),
            triggers: Vec::new(),
            sandbox: None,
            dependencies: deps
                .into_iter()
                .map(|(dep_name, dep_version)| PluginDependency {
                    name: dep_name.to_string(),
                    version: dep_version.map(String::from),
                })
                .collect(),
        }
    }

    // ── version helpers ──────────────────────────────────────────────────

    #[test]
    fn dependency_version_satisfies_equal() {
        assert!(version_satisfies("1.0.0", "1.0.0"));
    }

    #[test]
    fn dependency_version_satisfies_newer() {
        assert!(version_satisfies("2.1.0", "1.0.0"));
        assert!(version_satisfies("1.1.0", "1.0.0"));
        assert!(version_satisfies("1.0.1", "1.0.0"));
    }

    #[test]
    fn dependency_version_rejects_older() {
        assert!(!version_satisfies("0.9.0", "1.0.0"));
        assert!(!version_satisfies("1.0.0", "1.0.1"));
    }

    #[test]
    fn dependency_version_numeric_ordering() {
        // 1.10.0 > 1.9.0 — numeric, not lexicographic.
        assert!(version_satisfies("1.10.0", "1.9.0"));
        assert!(!version_satisfies("1.9.0", "1.10.0"));
    }

    // ── resolve: no dependencies ─────────────────────────────────────────

    #[test]
    fn dependency_resolve_empty_input() {
        let result = resolve_plugin_dependencies(&[]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn dependency_resolve_single_plugin_no_deps() {
        let manifests = vec![make_manifest("alpha", "1.0.0", vec![])];
        let resolved = resolve_plugin_dependencies(&manifests).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "alpha");
        assert_eq!(resolved[0].load_order, 0);
        assert!(resolved[0].depends_on.is_empty());
    }

    #[test]
    fn dependency_resolve_multiple_independent_plugins() {
        let manifests = vec![
            make_manifest("charlie", "1.0.0", vec![]),
            make_manifest("alpha", "1.0.0", vec![]),
            make_manifest("bravo", "1.0.0", vec![]),
        ];
        let resolved = resolve_plugin_dependencies(&manifests).unwrap();
        assert_eq!(resolved.len(), 3);
        // All independent — sorted alphabetically for determinism.
        assert_eq!(resolved[0].name, "alpha");
        assert_eq!(resolved[1].name, "bravo");
        assert_eq!(resolved[2].name, "charlie");
    }

    // ── resolve: linear dependency chain ──────────────────────────────────

    #[test]
    fn dependency_resolve_linear_chain() {
        // c depends on b, b depends on a.
        let manifests = vec![
            make_manifest("c", "1.0.0", vec![("b", None)]),
            make_manifest("b", "1.0.0", vec![("a", None)]),
            make_manifest("a", "1.0.0", vec![]),
        ];
        let resolved = resolve_plugin_dependencies(&manifests).unwrap();
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0].name, "a");
        assert_eq!(resolved[0].load_order, 0);
        assert_eq!(resolved[1].name, "b");
        assert_eq!(resolved[1].load_order, 1);
        assert_eq!(resolved[2].name, "c");
        assert_eq!(resolved[2].load_order, 2);
    }

    // ── resolve: diamond dependency ──────────────────────────────────────

    #[test]
    fn dependency_resolve_diamond() {
        //   d
        //  / \
        // b   c
        //  \ /
        //   a
        let manifests = vec![
            make_manifest("d", "1.0.0", vec![("b", None), ("c", None)]),
            make_manifest("b", "1.0.0", vec![("a", None)]),
            make_manifest("c", "1.0.0", vec![("a", None)]),
            make_manifest("a", "1.0.0", vec![]),
        ];
        let resolved = resolve_plugin_dependencies(&manifests).unwrap();
        assert_eq!(resolved.len(), 4);
        // a must come first, then b and c in alphabetical order, then d.
        assert_eq!(resolved[0].name, "a");
        assert_eq!(resolved[3].name, "d");
        // b and c can be in either order but both before d.
        let middle_names: Vec<&str> = resolved[1..3].iter().map(|r| r.name.as_str()).collect();
        assert!(middle_names.contains(&"b"));
        assert!(middle_names.contains(&"c"));
    }

    // ── resolve: version constraints ─────────────────────────────────────

    #[test]
    fn dependency_resolve_version_constraint_satisfied() {
        let manifests = vec![
            make_manifest("consumer", "1.0.0", vec![("provider", Some("1.0.0"))]),
            make_manifest("provider", "1.2.0", vec![]),
        ];
        let resolved = resolve_plugin_dependencies(&manifests).unwrap();
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].name, "provider");
        assert_eq!(resolved[1].name, "consumer");
    }

    #[test]
    fn dependency_resolve_version_constraint_violated() {
        let manifests = vec![
            make_manifest("consumer", "1.0.0", vec![("provider", Some("2.0.0"))]),
            make_manifest("provider", "1.0.0", vec![]),
        ];
        let err = resolve_plugin_dependencies(&manifests).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("consumer"),
            "error should name the consumer: {msg}"
        );
        assert!(
            msg.contains("provider"),
            "error should name the provider: {msg}"
        );
        assert!(
            msg.contains("2.0.0"),
            "error should mention required version: {msg}"
        );
    }

    // ── resolve: missing dependency ──────────────────────────────────────

    #[test]
    fn dependency_resolve_missing_dependency() {
        let manifests = vec![make_manifest(
            "lonely",
            "1.0.0",
            vec![("nonexistent", None)],
        )];
        let err = resolve_plugin_dependencies(&manifests).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("lonely"),
            "error should name the plugin: {msg}"
        );
        assert!(
            msg.contains("nonexistent"),
            "error should name the missing dep: {msg}"
        );
    }

    // ── resolve: cycle detection ─────────────────────────────────────────

    #[test]
    fn dependency_resolve_direct_cycle() {
        let manifests = vec![
            make_manifest("a", "1.0.0", vec![("b", None)]),
            make_manifest("b", "1.0.0", vec![("a", None)]),
        ];
        let err = resolve_plugin_dependencies(&manifests).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cycle"), "error should mention cycle: {msg}");
    }

    #[test]
    fn dependency_resolve_transitive_cycle() {
        let manifests = vec![
            make_manifest("a", "1.0.0", vec![("b", None)]),
            make_manifest("b", "1.0.0", vec![("c", None)]),
            make_manifest("c", "1.0.0", vec![("a", None)]),
        ];
        let err = resolve_plugin_dependencies(&manifests).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cycle"), "error should mention cycle: {msg}");
    }

    // ── resolve: depends_on populated ────────────────────────────────────

    #[test]
    fn dependency_resolve_populates_depends_on() {
        let manifests = vec![
            make_manifest("app", "1.0.0", vec![("lib-a", None), ("lib-b", None)]),
            make_manifest("lib-a", "1.0.0", vec![]),
            make_manifest("lib-b", "1.0.0", vec![]),
        ];
        let resolved = resolve_plugin_dependencies(&manifests).unwrap();
        let app = resolved.iter().find(|r| r.name == "app").unwrap();
        assert_eq!(app.depends_on.len(), 2);
        assert!(app.depends_on.contains(&"lib-a".to_string()));
        assert!(app.depends_on.contains(&"lib-b".to_string()));
    }

    // ── detect_duplicate_plugins ─────────────────────────────────────────

    #[test]
    fn dependency_detect_no_duplicates() {
        let manifests = vec![
            make_manifest("a", "1.0.0", vec![]),
            make_manifest("b", "2.0.0", vec![]),
        ];
        let dups = detect_duplicate_plugins(&manifests);
        assert!(dups.is_empty());
    }

    #[test]
    fn dependency_detect_exact_duplicates() {
        let manifests = vec![
            make_manifest("dup", "1.0.0", vec![]),
            make_manifest("dup", "1.0.0", vec![]),
        ];
        let dups = detect_duplicate_plugins(&manifests);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].name, "dup");
        assert_eq!(dups[0].count, 2);
        assert_eq!(dups[0].versions, vec!["1.0.0", "1.0.0"]);
    }

    #[test]
    fn dependency_detect_version_conflict_duplicates() {
        let manifests = vec![
            make_manifest("dup", "1.0.0", vec![]),
            make_manifest("dup", "2.0.0", vec![]),
        ];
        let dups = detect_duplicate_plugins(&manifests);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].name, "dup");
        assert_eq!(dups[0].count, 2);
        assert!(dups[0].versions.contains(&"1.0.0".to_string()));
        assert!(dups[0].versions.contains(&"2.0.0".to_string()));
    }

    #[test]
    fn dependency_detect_multiple_duplicated_names() {
        let manifests = vec![
            make_manifest("alpha", "1.0.0", vec![]),
            make_manifest("alpha", "1.1.0", vec![]),
            make_manifest("beta", "1.0.0", vec![]),
            make_manifest("beta", "2.0.0", vec![]),
            make_manifest("gamma", "1.0.0", vec![]),
        ];
        let dups = detect_duplicate_plugins(&manifests);
        assert_eq!(dups.len(), 2);
        // Sorted alphabetically.
        assert_eq!(dups[0].name, "alpha");
        assert_eq!(dups[1].name, "beta");
    }

    #[test]
    fn dependency_detect_triple_duplicate() {
        let manifests = vec![
            make_manifest("tri", "1.0.0", vec![]),
            make_manifest("tri", "1.1.0", vec![]),
            make_manifest("tri", "2.0.0", vec![]),
        ];
        let dups = detect_duplicate_plugins(&manifests);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].name, "tri");
        assert_eq!(dups[0].count, 3);
        assert_eq!(dups[0].versions.len(), 3);
    }

    #[test]
    fn dependency_detect_empty_input() {
        let dups = detect_duplicate_plugins(&[]);
        assert!(dups.is_empty());
    }

    // ── resolve with duplicates: first-wins ──────────────────────────────

    #[test]
    fn dependency_resolve_with_duplicates_takes_first() {
        // Two manifests with the same name — resolver picks the first one.
        let manifests = vec![
            make_manifest("dup", "1.0.0", vec![]),
            make_manifest("dup", "2.0.0", vec![]),
        ];
        let resolved = resolve_plugin_dependencies(&manifests).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "dup");
        assert_eq!(resolved[0].version, "1.0.0");
    }
}
