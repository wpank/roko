//! Runtime component registry for hot-swapping cognitive components.
//!
//! [`CognitiveRegistry`] holds named components as type-erased `Arc<dyn Any +
//! Send + Sync>` values behind a shared `RwLock`. Callers can register, look
//! up, and atomically swap any component without restarting the process.
//!
//! # Usage
//!
//! ```rust
//! use std::sync::Arc;
//! use roko_core::cognitive_registry::CognitiveRegistry;
//!
//! let registry = CognitiveRegistry::default();
//!
//! // Register a scorer implementation.
//! registry.register("scorer", Arc::new(42u32));
//!
//! // Retrieve it later (returns None on type mismatch).
//! let val: Option<Arc<u32>> = registry.get("scorer");
//! assert_eq!(val.as_deref().copied(), Some(42));
//!
//! // Swap to a new implementation without a restart.
//! let prev: Option<Arc<u32>> = registry.swap("scorer", Arc::new(99u32));
//! assert_eq!(prev.as_deref().copied(), Some(42));
//! assert_eq!(registry.get::<u32>("scorer").as_deref().copied(), Some(99));
//! ```

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe, type-erased registry of named runtime components.
///
/// Components are stored as `Arc<dyn Any + Send + Sync>` so the registry
/// itself stays `Send + Sync` and callers can cheaply clone handles without
/// copying the underlying value.
///
/// Locking strategy: a single `RwLock<HashMap<...>>` guards all entries.
/// Reads are concurrent; writes (`register` and `swap`) are exclusive but
/// momentary — callers hold the write lock only long enough to update the map.
#[derive(Debug, Default)]
pub struct CognitiveRegistry {
    inner: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,
}

impl CognitiveRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `component` under `name`.
    ///
    /// If a component is already registered under that name it is silently
    /// replaced. Use [`swap`](Self::swap) when you need the previous value.
    pub fn register<T: Any + Send + Sync>(&self, name: impl Into<String>, component: Arc<T>) {
        let erased: Arc<dyn Any + Send + Sync> = component;
        self.inner
            .write()
            .expect("CognitiveRegistry write lock poisoned")
            .insert(name.into(), erased);
    }

    /// Look up a component by name and downcast it to `Arc<T>`.
    ///
    /// Returns `None` when the name is not registered **or** the stored value
    /// is not of type `T`.
    #[must_use]
    pub fn get<T: Any + Send + Sync>(&self, name: &str) -> Option<Arc<T>> {
        let map = self
            .inner
            .read()
            .expect("CognitiveRegistry read lock poisoned");
        let erased = map.get(name)?;
        Arc::clone(erased).downcast::<T>().ok()
    }

    /// Atomically swap the component at `name` with `new_component`.
    ///
    /// Returns the previously stored `Arc<T>` (downcasted) or `None` when
    /// the name was not registered or the old component was a different type.
    /// The new component is always stored regardless of type compatibility.
    pub fn swap<T: Any + Send + Sync>(
        &self,
        name: impl Into<String>,
        new_component: Arc<T>,
    ) -> Option<Arc<T>> {
        let name = name.into();
        let new_erased: Arc<dyn Any + Send + Sync> = new_component;
        let old_erased = self
            .inner
            .write()
            .expect("CognitiveRegistry write lock poisoned")
            .insert(name, new_erased)?;
        Arc::clone(&old_erased).downcast::<T>().ok()
    }

    /// Remove a component by name and return it (downcasted), or `None` when
    /// absent or mistyped.
    pub fn remove<T: Any + Send + Sync>(&self, name: &str) -> Option<Arc<T>> {
        let erased = self
            .inner
            .write()
            .expect("CognitiveRegistry write lock poisoned")
            .remove(name)?;
        Arc::clone(&erased).downcast::<T>().ok()
    }

    /// Return the names of all currently registered components.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.inner
            .read()
            .expect("CognitiveRegistry read lock poisoned")
            .keys()
            .cloned()
            .collect()
    }

    /// Return `true` if a component is registered under `name`.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.inner
            .read()
            .expect("CognitiveRegistry read lock poisoned")
            .contains_key(name)
    }

    /// Return the number of registered components.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner
            .read()
            .expect("CognitiveRegistry read lock poisoned")
            .len()
    }

    /// Return `true` when the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_get_same_type() {
        let reg = CognitiveRegistry::new();
        reg.register("scorer", Arc::new(42u32));
        let val = reg.get::<u32>("scorer");
        assert_eq!(val.as_deref().copied(), Some(42));
    }

    #[test]
    fn get_missing_name_returns_none() {
        let reg = CognitiveRegistry::new();
        assert!(reg.get::<u32>("missing").is_none());
    }

    #[test]
    fn get_wrong_type_returns_none() {
        let reg = CognitiveRegistry::new();
        reg.register("scorer", Arc::new(42u32));
        // Requesting as u64 should fail the downcast.
        assert!(reg.get::<u64>("scorer").is_none());
    }

    #[test]
    fn register_overwrites_existing() {
        let reg = CognitiveRegistry::new();
        reg.register("scorer", Arc::new(1u32));
        reg.register("scorer", Arc::new(2u32));
        assert_eq!(reg.get::<u32>("scorer").as_deref().copied(), Some(2));
    }

    #[test]
    fn swap_returns_old_value() {
        let reg = CognitiveRegistry::new();
        reg.register("scorer", Arc::new(10u32));
        let prev = reg.swap("scorer", Arc::new(99u32));
        assert_eq!(prev.as_deref().copied(), Some(10));
        assert_eq!(reg.get::<u32>("scorer").as_deref().copied(), Some(99));
    }

    #[test]
    fn swap_on_empty_name_returns_none_but_stores_new() {
        let reg = CognitiveRegistry::new();
        let prev = reg.swap::<u32>("scorer", Arc::new(5u32));
        assert!(prev.is_none());
        assert_eq!(reg.get::<u32>("scorer").as_deref().copied(), Some(5));
    }

    #[test]
    fn remove_returns_value() {
        let reg = CognitiveRegistry::new();
        reg.register("scorer", Arc::new(7u32));
        let val = reg.remove::<u32>("scorer");
        assert_eq!(val.as_deref().copied(), Some(7));
        assert!(!reg.contains("scorer"));
    }

    #[test]
    fn remove_missing_returns_none() {
        let reg = CognitiveRegistry::new();
        assert!(reg.remove::<u32>("nope").is_none());
    }

    #[test]
    fn names_reflects_registered_keys() {
        let reg = CognitiveRegistry::new();
        reg.register("scorer", Arc::new(1u32));
        reg.register("router", Arc::new(2u32));
        let mut names = reg.names();
        names.sort();
        assert_eq!(names, vec!["router", "scorer"]);
    }

    #[test]
    fn contains_and_len_and_is_empty() {
        let reg = CognitiveRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(!reg.contains("scorer"));

        reg.register("scorer", Arc::new(1u32));
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);
        assert!(reg.contains("scorer"));
    }

    #[test]
    fn multiple_types_coexist() {
        let reg = CognitiveRegistry::new();
        reg.register("scorer", Arc::new(42u32));
        reg.register("router", Arc::new("cascade".to_string()));

        assert_eq!(reg.get::<u32>("scorer").as_deref().copied(), Some(42));
        assert_eq!(
            reg.get::<String>("router").as_deref().map(String::as_str),
            Some("cascade")
        );
    }

    #[test]
    fn send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CognitiveRegistry>();
    }

    #[test]
    fn concurrent_reads_do_not_panic() {
        use std::thread;

        let reg = Arc::new(CognitiveRegistry::new());
        reg.register("scorer", Arc::new(1u32));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let r = Arc::clone(&reg);
                thread::spawn(move || {
                    let val = r.get::<u32>("scorer");
                    assert_eq!(val.as_deref().copied(), Some(1));
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread panicked");
        }
    }
}
