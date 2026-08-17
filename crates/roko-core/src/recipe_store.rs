//! Atomic TOML persistence for recipes.

use std::path::{Path, PathBuf};

use crate::error::{Result, RokoError};
use crate::io::atomic_write_str;
use crate::recipe::Recipe;

/// Directory-backed store with one TOML file per recipe.
#[derive(Debug, Clone)]
pub struct RecipeStore {
    directory: PathBuf,
}

impl RecipeStore {
    /// Create a recipe store rooted at `.roko/recipes` (or an equivalent path).
    #[must_use]
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    /// Root directory used by the store.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Load one recipe.
    pub fn load(&self, id: &str) -> Result<Recipe> {
        let path = self.path(id)?;
        let contents = std::fs::read_to_string(path)?;
        toml::from_str(&contents)
            .map_err(|error| RokoError::invalid(format!("invalid recipe TOML: {error}")))
    }

    /// Save a valid recipe atomically. Overwriting increments the version.
    pub fn save(&self, recipe: &Recipe) -> Result<Recipe> {
        let errors = recipe.validate();
        if !errors.is_empty() {
            return Err(RokoError::invalid(errors.join("; ")));
        }
        let mut persisted = recipe.clone();
        if let Ok(existing) = self.load(&recipe.id) {
            persisted.version = existing
                .version
                .saturating_add(1)
                .max(recipe.version.saturating_add(1));
        } else {
            persisted.version = persisted.version.max(1);
        }
        let path = self.path(&persisted.id)?;
        let contents = toml::to_string_pretty(&persisted)
            .map_err(|error| RokoError::invalid(format!("cannot encode recipe TOML: {error}")))?;
        atomic_write_str(&path, &contents)?;
        Ok(persisted)
    }

    /// Return recipe identifiers in stable order.
    pub fn list(&self) -> Result<Vec<String>> {
        if !self.directory.exists() {
            return Ok(Vec::new());
        }
        let mut ids = std::fs::read_dir(&self.directory)?
            .filter_map(std::result::Result::ok)
            .filter_map(|entry| {
                entry
                    .path()
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();
        ids.sort();
        Ok(ids)
    }

    /// Delete a recipe, returning whether it existed.
    pub fn delete(&self, id: &str) -> Result<bool> {
        let path = self.path(id)?;
        match std::fs::remove_file(path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn path(&self, id: &str) -> Result<PathBuf> {
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(RokoError::invalid(
                "recipe id must contain only letters, digits, '-' or '_'",
            ));
        }
        Ok(self.directory.join(format!("{id}.toml")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::{RecipeEdge, RecipeNode, ScoreOp};
    use std::collections::HashMap;

    fn recipe() -> Recipe {
        Recipe {
            id: "example".into(),
            name: "Example".into(),
            version: 1,
            nodes: vec![RecipeNode {
                id: "clamp".into(),
                operation: ScoreOp::Clamp,
                params: HashMap::new(),
            }],
            edges: vec![RecipeEdge {
                from: "input".into(),
                to: "clamp".into(),
                field: String::new(),
            }],
            input_feeds: vec!["input".into()],
            output_schema: None,
        }
    }

    #[test]
    fn round_trips_and_increments_version() {
        let directory = tempfile::tempdir().unwrap();
        let store = RecipeStore::new(directory.path());
        assert_eq!(store.save(&recipe()).unwrap().version, 1);
        assert_eq!(store.save(&recipe()).unwrap().version, 2);
        assert_eq!(store.list().unwrap(), vec!["example"]);
        assert!(store.delete("example").unwrap());
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(RecipeStore::new("recipes").load("../secret").is_err());
    }
}
