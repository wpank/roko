//! `CellRegistry` -- maps cell type names to factory functions that produce `Cell` instances.
//!
//! Each registered cell type has both a [`CellDescriptor`] (side-effect-free metadata used
//! for edge validation) and a [`CellFactory`] (closure that constructs a live Cell).

use std::collections::HashMap;

use roko_core::TypeSchema;

use crate::cell::{Cell, CellVersion};
use crate::types::GraphError;

/// A factory function that takes a TOML config and produces a boxed Cell.
pub type CellFactory = Box<dyn Fn(toml::Value) -> Box<dyn Cell> + Send + Sync>;

/// Side-effect-free metadata for a registered cell type.
///
/// Used by [`Graph::validate_edges`] to check edge type compatibility without
/// constructing live Cell instances. Every production registration must provide
/// a descriptor; test-only registrations may use [`CellDescriptor::test_stub`].
#[derive(Debug, Clone, PartialEq)]
pub struct CellDescriptor {
    /// Cell type name (matches the registry key).
    pub id: String,
    /// Semantic version of the cell implementation.
    pub version: CellVersion,
    /// Input type schema the cell expects. `None` means untyped (Any).
    pub input_schema: Option<TypeSchema>,
    /// Output type schema the cell produces. `None` means untyped (Any).
    pub output_schema: Option<TypeSchema>,
    /// Whether this is a test-only stub. Production starts reject graphs
    /// containing stub descriptors.
    pub is_stub: bool,
}

impl CellDescriptor {
    /// Create a descriptor for a production cell type.
    pub fn new(
        id: impl Into<String>,
        version: CellVersion,
        input_schema: Option<TypeSchema>,
        output_schema: Option<TypeSchema>,
    ) -> Self {
        Self {
            id: id.into(),
            version,
            input_schema,
            output_schema,
            is_stub: false,
        }
    }

    /// Create a test-only stub descriptor. Production starts reject graphs
    /// containing stub descriptors.
    pub fn test_stub(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: (0, 0, 0),
            input_schema: None,
            output_schema: None,
            is_stub: true,
        }
    }
}

/// Internal storage: a descriptor paired with its factory.
struct CellEntry {
    descriptor: CellDescriptor,
    factory: CellFactory,
}

/// Registry that maps cell type name strings to factory functions.
///
/// When the graph engine encounters a node with `cell_type = "gate.compile"`,
/// it looks up "gate.compile" in this registry to obtain a factory, then calls
/// it with the node's config to instantiate the cell.
pub struct CellRegistry {
    entries: HashMap<String, CellEntry>,
}

impl CellRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register a factory function for a cell type name.
    ///
    /// An auto-generated untyped (Any) descriptor is created. Prefer
    /// [`CellRegistry::register_with_descriptor`] for production cells
    /// that declare typed schemas.
    ///
    /// If a factory was already registered for this name, it is replaced.
    pub fn register<F>(&mut self, cell_type: &str, factory: F)
    where
        F: Fn(toml::Value) -> Box<dyn Cell> + Send + Sync + 'static,
    {
        let descriptor = CellDescriptor {
            id: cell_type.to_string(),
            version: (0, 1, 0),
            input_schema: None,
            output_schema: None,
            is_stub: false,
        };
        self.entries.insert(
            cell_type.to_string(),
            CellEntry {
                descriptor,
                factory: Box::new(factory),
            },
        );
    }

    /// Register a factory function with an explicit descriptor.
    ///
    /// The descriptor provides side-effect-free schema introspection for edge
    /// validation without constructing a live Cell.
    pub fn register_with_descriptor<F>(
        &mut self,
        cell_type: &str,
        descriptor: CellDescriptor,
        factory: F,
    ) where
        F: Fn(toml::Value) -> Box<dyn Cell> + Send + Sync + 'static,
    {
        self.entries.insert(
            cell_type.to_string(),
            CellEntry {
                descriptor,
                factory: Box::new(factory),
            },
        );
    }

    /// Look up a factory by cell type name and instantiate a Cell with the given config.
    ///
    /// # Errors
    /// Returns `GraphError::UnknownCellType` if no factory is registered for the name.
    pub fn create(
        &self,
        cell_type: &str,
        config: toml::Value,
    ) -> Result<Box<dyn Cell>, GraphError> {
        let entry = self
            .entries
            .get(cell_type)
            .ok_or_else(|| GraphError::UnknownCellType(cell_type.to_string()))?;
        Ok((entry.factory)(config))
    }

    /// Look up the descriptor for a cell type without constructing a Cell.
    ///
    /// This is side-effect-free and used by edge validation.
    #[must_use]
    pub fn descriptor(&self, cell_type: &str) -> Option<&CellDescriptor> {
        self.entries.get(cell_type).map(|e| &e.descriptor)
    }

    /// Check if a cell type is registered.
    #[must_use]
    pub fn contains(&self, cell_type: &str) -> bool {
        self.entries.contains_key(cell_type)
    }

    /// Return the number of registered cell types.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return an iterator over registered cell type names.
    pub fn cell_types(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}

impl Default for CellRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CellRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellRegistry")
            .field(
                "registered_types",
                &self.entries.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use roko_core::Signal;

    use crate::cell::{CellContext, CellVersion};

    use super::*;

    /// A trivial Cell implementation for testing the registry.
    struct NoopCell {
        id: String,
    }

    #[async_trait::async_trait]
    impl Cell for NoopCell {
        fn cell_id(&self) -> &str {
            &self.id
        }
        fn cell_name(&self) -> &str {
            "noop"
        }
        fn cell_version(&self) -> CellVersion {
            (0, 1, 0)
        }
        fn protocols(&self) -> Vec<roko_core::ProtocolId> {
            Vec::new()
        }
        fn estimated_cost(&self) -> Option<f64> {
            None
        }
        fn estimated_duration(&self) -> Option<Duration> {
            None
        }
        async fn execute(
            &self,
            input: Vec<Signal>,
            _ctx: &CellContext,
        ) -> roko_core::error::Result<Vec<Signal>> {
            Ok(input)
        }
    }

    #[test]
    fn register_and_create() {
        let mut registry = CellRegistry::new();
        registry.register("noop", |_config| {
            Box::new(NoopCell {
                id: "noop-1".to_string(),
            })
        });

        assert!(registry.contains("noop"));
        assert_eq!(registry.len(), 1);

        let cell = registry
            .create("noop", toml::Value::Table(toml::map::Map::new()))
            .unwrap();
        assert_eq!(cell.cell_id(), "noop-1");
        assert_eq!(cell.cell_name(), "noop");
    }

    #[test]
    fn unknown_cell_type_errors() {
        let registry = CellRegistry::new();
        let result = registry.create("nonexistent", toml::Value::Table(toml::map::Map::new()));
        assert!(matches!(
            result,
            Err(GraphError::UnknownCellType(ref t)) if t == "nonexistent"
        ));
    }

    #[test]
    fn empty_registry() {
        let registry = CellRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(!registry.contains("anything"));
    }

    #[test]
    fn register_replaces_existing() {
        let mut registry = CellRegistry::new();
        registry.register("test", |_| {
            Box::new(NoopCell {
                id: "first".to_string(),
            })
        });
        registry.register("test", |_| {
            Box::new(NoopCell {
                id: "second".to_string(),
            })
        });

        let cell = registry
            .create("test", toml::Value::Table(toml::map::Map::new()))
            .unwrap();
        assert_eq!(cell.cell_id(), "second");
    }

    #[test]
    fn cell_types_iterator() {
        let mut registry = CellRegistry::new();
        registry.register("alpha", |_| {
            Box::new(NoopCell {
                id: "a".to_string(),
            })
        });
        registry.register("beta", |_| {
            Box::new(NoopCell {
                id: "b".to_string(),
            })
        });

        let mut types: Vec<&str> = registry.cell_types().collect();
        types.sort();
        assert_eq!(types, vec!["alpha", "beta"]);
    }

    #[test]
    fn descriptor_returns_auto_generated() {
        let mut registry = CellRegistry::new();
        registry.register("noop", |_| {
            Box::new(NoopCell {
                id: "n".to_string(),
            })
        });

        let desc = registry.descriptor("noop").expect("descriptor must exist");
        assert_eq!(desc.id, "noop");
        assert_eq!(desc.version, (0, 1, 0));
        assert!(desc.input_schema.is_none());
        assert!(desc.output_schema.is_none());
        assert!(!desc.is_stub);
    }

    #[test]
    fn descriptor_explicit_registration() {
        let mut registry = CellRegistry::new();
        let desc = CellDescriptor::new(
            "typed-cell",
            (1, 2, 0),
            Some(TypeSchema::OfKind(roko_core::Kind::Task)),
            Some(TypeSchema::OfKind(roko_core::Kind::Episode)),
        );
        registry.register_with_descriptor("typed-cell", desc, |_| {
            Box::new(NoopCell {
                id: "t".to_string(),
            })
        });

        let d = registry
            .descriptor("typed-cell")
            .expect("descriptor must exist");
        assert_eq!(d.id, "typed-cell");
        assert_eq!(d.version, (1, 2, 0));
        assert_eq!(d.input_schema, Some(TypeSchema::OfKind(roko_core::Kind::Task)));
        assert_eq!(
            d.output_schema,
            Some(TypeSchema::OfKind(roko_core::Kind::Episode))
        );
        assert!(!d.is_stub);
    }

    #[test]
    fn test_stub_descriptor() {
        let desc = CellDescriptor::test_stub("my-stub");
        assert_eq!(desc.id, "my-stub");
        assert!(desc.is_stub);
        assert_eq!(desc.version, (0, 0, 0));
        assert!(desc.input_schema.is_none());
        assert!(desc.output_schema.is_none());
    }

    #[test]
    fn descriptor_not_found_returns_none() {
        let registry = CellRegistry::new();
        assert!(registry.descriptor("nonexistent").is_none());
    }
}
