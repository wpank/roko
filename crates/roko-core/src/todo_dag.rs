//! Todo/subagent DAG model — tracks hierarchical todo items produced by
//! operators, providers, and the system during agent runs.
//!
//! A [`TodoDag`] holds [`TodoItem`] nodes in a parent-child + dependency
//! graph. Items are immutably snapshotted via [`TodoSnapshot`] and
//! incrementally updated via [`TodoDelta`]. The DAG enforces acyclicity
//! on both the parent and dependency edges.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

// ─── TodoStatus ──────────────────────────────────────────────────────────

/// Lifecycle status of a single todo item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TodoStatus {
    /// Not yet started.
    Pending,
    /// Currently being worked on.
    Active,
    /// Waiting on unresolved dependencies.
    Blocked,
    /// Successfully completed.
    Done,
    /// Terminated with an error.
    Failed,
    /// Removed before completion.
    Cancelled,
}

impl TodoStatus {
    /// Whether the item is in a terminal state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled)
    }

    /// Single-char icon for TUI display.
    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Pending => "·",
            Self::Active => "►",
            Self::Blocked => "⊘",
            Self::Done => "✓",
            Self::Failed => "✗",
            Self::Cancelled => "–",
        }
    }

    /// Lowercase label matching serde serialization.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl Default for TodoStatus {
    fn default() -> Self {
        Self::Pending
    }
}

// ─── TodoSource ──────────────────────────────────────────────────────────

/// Origin of a todo item.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum TodoSource {
    /// Created by the human operator.
    Operator,
    /// Created by an LLM provider during a turn.
    Provider {
        /// Name of the provider that created this item.
        provider_name: String,
    },
    /// Created by the system (e.g. automatic dependency tracking).
    System,
}

impl Default for TodoSource {
    fn default() -> Self {
        Self::System
    }
}

// ─── TodoItem ────────────────────────────────────────────────────────────

/// A single node in the todo DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoItem {
    /// Unique identifier for this todo.
    pub todo_id: String,
    /// Parent todo ID (forms the tree hierarchy).
    pub parent_id: Option<String>,
    /// Short human-readable summary.
    pub title: String,
    /// Extended description or implementation notes.
    pub details: Option<String>,
    /// Current lifecycle status.
    pub status: TodoStatus,
    /// Priority (lower = higher priority, 0 is highest).
    pub priority: u32,
    /// Agent currently responsible for this item.
    pub owner_agent_id: Option<String>,
    /// IDs of other todos that must complete before this one starts.
    #[serde(default)]
    pub dependency_ids: Vec<String>,
    /// Unix timestamp (millis) when this item was created.
    pub created_at: i64,
    /// Unix timestamp (millis) of the last update.
    pub updated_at: i64,
    /// Completion progress from 0.0 to 1.0.
    pub progress: Option<f64>,
    /// Result of the verification step, if any.
    pub verification_result: Option<String>,
    /// Where this todo originated.
    pub source: TodoSource,
}

impl TodoItem {
    /// Construct a minimal todo item with defaults.
    #[must_use]
    pub fn new(todo_id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        Self {
            todo_id: todo_id.into(),
            parent_id: None,
            title: title.into(),
            details: None,
            status: TodoStatus::Pending,
            priority: 100,
            owner_agent_id: None,
            dependency_ids: Vec::new(),
            created_at: now,
            updated_at: now,
            progress: None,
            verification_result: None,
            source: TodoSource::System,
        }
    }
}

// ─── TodoSnapshot ────────────────────────────────────────────────────────

/// Immutable snapshot of the full todo state at a point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoSnapshot {
    /// Run that produced this snapshot.
    pub run_id: String,
    /// Turn within the run.
    pub turn_id: String,
    /// Monotonic sequence number within the run.
    pub sequence: u64,
    /// Unix timestamp (millis).
    pub timestamp: i64,
    /// All items at the time of the snapshot.
    pub items: Vec<TodoItem>,
}

// ─── TodoChanges ─────────────────────────────────────────────────────────

/// The set of field-level changes applied in a single delta.
///
/// Each `Option` field, when `Some`, indicates that the field changed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TodoChanges {
    /// New status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TodoStatus>,
    /// New title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// New details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Option<String>>,
    /// New priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
    /// New owner agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_agent_id: Option<Option<String>>,
    /// New parent (reparenting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Option<String>>,
    /// Dependencies to add.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add_dependencies: Option<Vec<String>>,
    /// Dependencies to remove.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove_dependencies: Option<Vec<String>>,
    /// New progress value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<Option<f64>>,
    /// New verification result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_result: Option<Option<String>>,
}

impl TodoChanges {
    /// Returns true if no fields are set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.title.is_none()
            && self.details.is_none()
            && self.priority.is_none()
            && self.owner_agent_id.is_none()
            && self.parent_id.is_none()
            && self.add_dependencies.is_none()
            && self.remove_dependencies.is_none()
            && self.progress.is_none()
            && self.verification_result.is_none()
    }
}

// ─── TodoDelta ───────────────────────────────────────────────────────────

/// Incremental update event for a single todo item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoDelta {
    /// Run that produced this delta.
    pub run_id: String,
    /// Turn within the run.
    pub turn_id: String,
    /// Monotonic sequence number within the run.
    pub sequence: u64,
    /// Unix timestamp (millis).
    pub timestamp: i64,
    /// The todo item being changed.
    pub todo_id: String,
    /// The field-level changes.
    pub changes: TodoChanges,
}

// ─── TodoDagError ────────────────────────────────────────────────────────

/// Errors returned by [`TodoDag`] operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TodoDagError {
    /// The referenced todo ID was not found.
    #[error("todo not found: {0}")]
    NotFound(String),
    /// Inserting a duplicate todo ID.
    #[error("duplicate todo id: {0}")]
    DuplicateId(String),
    /// Adding the edge would create a cycle.
    #[error("cycle detected involving: {0}")]
    CycleDetected(String),
    /// The referenced parent ID does not exist.
    #[error("parent not found: {0}")]
    ParentNotFound(String),
    /// A referenced dependency ID does not exist.
    #[error("dependency not found: {0}")]
    DependencyNotFound(String),
}

// ─── TodoDag ─────────────────────────────────────────────────────────────

/// In-memory DAG of todo items with concurrent-safe interior mutability.
///
/// The DAG enforces two kinds of edges:
/// - **Parent edges**: form a tree (each item has at most one parent).
/// - **Dependency edges**: form a DAG (items can depend on multiple others).
///
/// Both edge sets are validated for acyclicity on mutation.
#[derive(Debug, Clone)]
pub struct TodoDag {
    inner: Arc<RwLock<DagInner>>,
}

#[derive(Debug, Clone, Default)]
struct DagInner {
    /// All items keyed by todo_id.
    items: HashMap<String, TodoItem>,
    /// Children keyed by parent_id.
    children: HashMap<String, HashSet<String>>,
}

impl Default for TodoDag {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoDag {
    /// Create an empty DAG.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(DagInner::default())),
        }
    }

    /// Number of items in the DAG.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.read().items.len()
    }

    /// Whether the DAG is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.read().items.is_empty()
    }

    /// Insert a new todo item. Returns error if the ID already exists
    /// or the parent/dependency references would create a cycle.
    pub fn insert(&self, item: TodoItem) -> Result<(), TodoDagError> {
        let mut dag = self.inner.write();

        if dag.items.contains_key(&item.todo_id) {
            return Err(TodoDagError::DuplicateId(item.todo_id.clone()));
        }

        // Validate parent exists if specified.
        if let Some(ref pid) = item.parent_id
            && !dag.items.contains_key(pid)
        {
            return Err(TodoDagError::ParentNotFound(pid.clone()));
            // Check that setting this parent wouldn't create a cycle
            // (the new item has no children yet, so only need to check
            // if the parent is somehow already a descendant — impossible
            // for a fresh insert, but validate anyway for safety).
        }

        // Validate dependency references exist.
        for dep_id in &item.dependency_ids {
            if !dag.items.contains_key(dep_id) {
                return Err(TodoDagError::DependencyNotFound(dep_id.clone()));
            }
        }

        // Record in children index.
        if let Some(ref pid) = item.parent_id {
            dag.children
                .entry(pid.clone())
                .or_default()
                .insert(item.todo_id.clone());
        }

        dag.items.insert(item.todo_id.clone(), item);
        Ok(())
    }

    /// Get a clone of the item with the given ID.
    #[must_use]
    pub fn get(&self, todo_id: &str) -> Option<TodoItem> {
        self.inner.read().items.get(todo_id).cloned()
    }

    /// Apply a delta to an existing item. Validates that structural
    /// changes (parent, dependencies) preserve acyclicity.
    pub fn apply_delta(&self, delta: &TodoDelta) -> Result<(), TodoDagError> {
        let mut dag = self.inner.write();

        // Verify the item exists.
        if !dag.items.contains_key(&delta.todo_id) {
            return Err(TodoDagError::NotFound(delta.todo_id.clone()));
        }

        let changes = &delta.changes;

        // Apply simple field changes.
        {
            let item = dag
                .items
                .get_mut(&delta.todo_id)
                .expect("checked existence above");
            if let Some(status) = changes.status {
                item.status = status;
            }
            if let Some(ref title) = changes.title {
                item.title = title.clone();
            }
            if let Some(ref details) = changes.details {
                item.details = details.clone();
            }
            if let Some(priority) = changes.priority {
                item.priority = priority;
            }
            if let Some(ref owner) = changes.owner_agent_id {
                item.owner_agent_id = owner.clone();
            }
            if let Some(ref progress) = changes.progress {
                item.progress = *progress;
            }
            if let Some(ref vr) = changes.verification_result {
                item.verification_result = vr.clone();
            }
        }
        // `item` borrow is now dropped — safe to borrow dag for structural checks.

        // Handle dependency additions.
        if let Some(ref adds) = changes.add_dependencies {
            for dep_id in adds {
                if !dag.items.contains_key(dep_id) {
                    return Err(TodoDagError::DependencyNotFound(dep_id.clone()));
                }
                if has_dependency_path(&dag.items, dep_id, &delta.todo_id) {
                    return Err(TodoDagError::CycleDetected(format!(
                        "{} -> {}",
                        delta.todo_id, dep_id
                    )));
                }
            }
            let item = dag
                .items
                .get_mut(&delta.todo_id)
                .expect("checked existence above");
            for dep_id in adds {
                if !item.dependency_ids.contains(dep_id) {
                    item.dependency_ids.push(dep_id.clone());
                }
            }
        }

        // Handle dependency removals.
        if let Some(ref removes) = changes.remove_dependencies {
            let item = dag
                .items
                .get_mut(&delta.todo_id)
                .expect("checked existence above");
            item.dependency_ids.retain(|d| !removes.contains(d));
        }

        // Handle reparenting.
        if let Some(ref new_parent) = changes.parent_id {
            let old_parent = dag.items[&delta.todo_id].parent_id.clone();
            let todo_id = delta.todo_id.clone();

            // Validate new parent exists.
            if let Some(pid) = new_parent {
                if !dag.items.contains_key(pid) {
                    return Err(TodoDagError::ParentNotFound(pid.clone()));
                }
                if is_descendant(&dag.children, &todo_id, pid) {
                    return Err(TodoDagError::CycleDetected(format!(
                        "reparent {} under descendant {}",
                        todo_id, pid
                    )));
                }
            }

            // Update children index.
            if let Some(ref old_pid) = old_parent
                && let Some(siblings) = dag.children.get_mut(old_pid)
            {
                siblings.remove(&todo_id);
                if siblings.is_empty() {
                    dag.children.remove(old_pid);
                }
            }
            if let Some(new_pid) = new_parent {
                dag.children
                    .entry(new_pid.clone())
                    .or_default()
                    .insert(todo_id);
            }

            let item = dag
                .items
                .get_mut(&delta.todo_id)
                .expect("checked existence above");
            item.parent_id = new_parent.clone();
            item.updated_at = delta.timestamp;
        } else {
            let item = dag
                .items
                .get_mut(&delta.todo_id)
                .expect("checked existence above");
            item.updated_at = delta.timestamp;
        }

        Ok(())
    }

    /// Remove an item (and recursively all its children) from the DAG.
    /// Returns the removed items, or error if not found.
    pub fn remove(&self, todo_id: &str) -> Result<Vec<TodoItem>, TodoDagError> {
        let mut dag = self.inner.write();

        if !dag.items.contains_key(todo_id) {
            return Err(TodoDagError::NotFound(todo_id.to_string()));
        }

        // Collect the subtree (BFS).
        let mut to_remove = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(todo_id.to_string());

        while let Some(id) = queue.pop_front() {
            to_remove.push(id.clone());
            if let Some(child_ids) = dag.children.get(&id) {
                for child_id in child_ids {
                    queue.push_back(child_id.clone());
                }
            }
        }

        // Remove all collected items.
        let mut removed = Vec::with_capacity(to_remove.len());
        for id in &to_remove {
            if let Some(item) = dag.items.remove(id) {
                // Clean up parent's children set.
                if let Some(ref pid) = item.parent_id
                    && let Some(siblings) = dag.children.get_mut(pid)
                {
                    siblings.remove(id);
                    if siblings.is_empty() {
                        dag.children.remove(pid);
                    }
                }
                removed.push(item);
            }
            dag.children.remove(id);
        }

        // Clean up dependency references from remaining items.
        let removed_set: HashSet<&str> = to_remove.iter().map(|s| s.as_str()).collect();
        for item in dag.items.values_mut() {
            item.dependency_ids
                .retain(|d| !removed_set.contains(d.as_str()));
        }

        Ok(removed)
    }

    /// Query items by status.
    #[must_use]
    pub fn by_status(&self, status: TodoStatus) -> Vec<TodoItem> {
        self.inner
            .read()
            .items
            .values()
            .filter(|item| item.status == status)
            .cloned()
            .collect()
    }

    /// Query items owned by a specific agent.
    #[must_use]
    pub fn by_owner(&self, agent_id: &str) -> Vec<TodoItem> {
        self.inner
            .read()
            .items
            .values()
            .filter(|item| item.owner_agent_id.as_deref() == Some(agent_id))
            .cloned()
            .collect()
    }

    /// Get all direct children of a given item.
    #[must_use]
    pub fn children_of(&self, parent_id: &str) -> Vec<TodoItem> {
        let dag = self.inner.read();
        dag.children
            .get(parent_id)
            .map(|child_ids| {
                child_ids
                    .iter()
                    .filter_map(|id| dag.items.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the full subtree (all descendants) rooted at a given item,
    /// returned in BFS order. The root itself is included.
    #[must_use]
    pub fn subtree(&self, root_id: &str) -> Vec<TodoItem> {
        let dag = self.inner.read();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        if dag.items.contains_key(root_id) {
            queue.push_back(root_id.to_string());
        }

        while let Some(id) = queue.pop_front() {
            if let Some(item) = dag.items.get(&id) {
                result.push(item.clone());
            }
            if let Some(child_ids) = dag.children.get(&id) {
                for child_id in child_ids {
                    queue.push_back(child_id.clone());
                }
            }
        }

        result
    }

    /// Get all root items (those without a parent).
    #[must_use]
    pub fn roots(&self) -> Vec<TodoItem> {
        self.inner
            .read()
            .items
            .values()
            .filter(|item| item.parent_id.is_none())
            .cloned()
            .collect()
    }

    /// Get items that depend on the given item.
    #[must_use]
    pub fn dependents_of(&self, todo_id: &str) -> Vec<TodoItem> {
        self.inner
            .read()
            .items
            .values()
            .filter(|item| item.dependency_ids.iter().any(|d| d == todo_id))
            .cloned()
            .collect()
    }

    /// Create a full snapshot of the current state.
    #[must_use]
    pub fn snapshot(&self, run_id: String, turn_id: String, sequence: u64) -> TodoSnapshot {
        let dag = self.inner.read();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        TodoSnapshot {
            run_id,
            turn_id,
            sequence,
            timestamp: now,
            items: dag.items.values().cloned().collect(),
        }
    }

    /// Rebuild the DAG from a snapshot. Replaces all current state.
    pub fn restore_from_snapshot(&self, snapshot: &TodoSnapshot) -> Result<(), TodoDagError> {
        let mut dag = self.inner.write();
        dag.items.clear();
        dag.children.clear();

        // Insert all items.
        for item in &snapshot.items {
            dag.items.insert(item.todo_id.clone(), item.clone());
            if let Some(ref pid) = item.parent_id {
                dag.children
                    .entry(pid.clone())
                    .or_default()
                    .insert(item.todo_id.clone());
            }
        }

        // Validate no cycles in parent edges.
        if has_parent_cycle(&dag.items) {
            dag.items.clear();
            dag.children.clear();
            return Err(TodoDagError::CycleDetected(
                "parent cycle in snapshot".to_string(),
            ));
        }

        // Validate no cycles in dependency edges.
        if has_dep_cycle(&dag.items) {
            dag.items.clear();
            dag.children.clear();
            return Err(TodoDagError::CycleDetected(
                "dependency cycle in snapshot".to_string(),
            ));
        }

        Ok(())
    }

    /// Replay a sequence of deltas onto the DAG (e.g. from a persisted log).
    pub fn replay_deltas(&self, deltas: &[TodoDelta]) -> Result<(), TodoDagError> {
        for delta in deltas {
            self.apply_delta(delta)?;
        }
        Ok(())
    }

    /// Return all items as a vec (unordered).
    #[must_use]
    pub fn all_items(&self) -> Vec<TodoItem> {
        self.inner.read().items.values().cloned().collect()
    }

    /// Validate the entire DAG: no cycles in parent or dependency edges,
    /// and all referenced parents/dependencies exist.
    #[must_use]
    pub fn validate(&self) -> Vec<TodoDagError> {
        let dag = self.inner.read();
        let mut errors = Vec::new();

        for item in dag.items.values() {
            // Check parent reference.
            if let Some(ref pid) = item.parent_id
                && !dag.items.contains_key(pid)
            {
                errors.push(TodoDagError::ParentNotFound(pid.clone()));
            }
            // Check dependency references.
            for dep_id in &item.dependency_ids {
                if !dag.items.contains_key(dep_id) {
                    errors.push(TodoDagError::DependencyNotFound(dep_id.clone()));
                }
            }
        }

        if has_parent_cycle(&dag.items) {
            errors.push(TodoDagError::CycleDetected("parent edge cycle".to_string()));
        }
        if has_dep_cycle(&dag.items) {
            errors.push(TodoDagError::CycleDetected(
                "dependency edge cycle".to_string(),
            ));
        }

        errors
    }
}

// ─── Cycle detection helpers ─────────────────────────────────────────────

/// Check whether `target` is reachable from `start` via dependency edges.
fn has_dependency_path(items: &HashMap<String, TodoItem>, start: &str, target: &str) -> bool {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start.to_string());

    while let Some(current) = queue.pop_front() {
        if current == target {
            return true;
        }
        if !visited.insert(current.clone()) {
            continue;
        }
        if let Some(item) = items.get(&current) {
            for dep_id in &item.dependency_ids {
                queue.push_back(dep_id.clone());
            }
        }
    }
    false
}

/// Check whether `candidate` is a descendant of `ancestor` in the parent tree.
fn is_descendant(
    children: &HashMap<String, HashSet<String>>,
    ancestor: &str,
    candidate: &str,
) -> bool {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(ancestor.to_string());

    while let Some(current) = queue.pop_front() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if let Some(child_ids) = children.get(&current) {
            for child_id in child_ids {
                if child_id == candidate {
                    return true;
                }
                queue.push_back(child_id.clone());
            }
        }
    }
    false
}

/// Detect cycles in the parent edges using iterative ancestor walking.
fn has_parent_cycle(items: &HashMap<String, TodoItem>) -> bool {
    for item in items.values() {
        let mut visited = HashSet::new();
        let mut current_id = Some(item.todo_id.as_str());
        while let Some(id) = current_id {
            if !visited.insert(id) {
                return true;
            }
            current_id = items.get(id).and_then(|i| i.parent_id.as_deref());
        }
    }
    false
}

/// Detect cycles in the dependency edges using Kahn's algorithm.
fn has_dep_cycle(items: &HashMap<String, TodoItem>) -> bool {
    // Build in-degree map.
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for id in items.keys() {
        in_degree.entry(id.as_str()).or_insert(0);
    }
    for item in items.values() {
        for dep_id in &item.dependency_ids {
            if items.contains_key(dep_id) {
                *in_degree.entry(item.todo_id.as_str()).or_insert(0) += 1;
            }
        }
    }

    // Start with zero in-degree nodes.
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut visited = 0usize;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        // Find items that depend on `node` and decrement their in-degree.
        for item in items.values() {
            if item.dependency_ids.iter().any(|d| d == node) {
                let deg = in_degree
                    .get_mut(item.todo_id.as_str())
                    .expect("all items present in in_degree map");
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(item.todo_id.as_str());
                }
            }
        }
    }

    visited < items.len()
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: &str, title: &str) -> TodoItem {
        TodoItem {
            todo_id: id.to_string(),
            parent_id: None,
            title: title.to_string(),
            details: None,
            status: TodoStatus::Pending,
            priority: 100,
            owner_agent_id: None,
            dependency_ids: Vec::new(),
            created_at: 1000,
            updated_at: 1000,
            progress: None,
            verification_result: None,
            source: TodoSource::System,
        }
    }

    fn make_delta(todo_id: &str, changes: TodoChanges) -> TodoDelta {
        TodoDelta {
            run_id: "run-1".to_string(),
            turn_id: "turn-1".to_string(),
            sequence: 1,
            timestamp: 2000,
            todo_id: todo_id.to_string(),
            changes,
        }
    }

    // ── Basic CRUD ───────────────────────────────────────────────────

    #[test]
    fn insert_and_get() {
        let dag = TodoDag::new();
        let item = make_item("t1", "first");
        dag.insert(item.clone()).unwrap();
        assert_eq!(dag.len(), 1);
        assert!(!dag.is_empty());

        let got = dag.get("t1").unwrap();
        assert_eq!(got.title, "first");
        assert!(dag.get("t999").is_none());
    }

    #[test]
    fn insert_duplicate_fails() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "a")).unwrap();
        let err = dag.insert(make_item("t1", "b")).unwrap_err();
        assert!(matches!(err, TodoDagError::DuplicateId(ref id) if id == "t1"));
    }

    #[test]
    fn remove_item() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "a")).unwrap();
        let removed = dag.remove("t1").unwrap();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].todo_id, "t1");
        assert!(dag.is_empty());
    }

    #[test]
    fn remove_not_found() {
        let dag = TodoDag::new();
        let err = dag.remove("t1").unwrap_err();
        assert!(matches!(err, TodoDagError::NotFound(_)));
    }

    #[test]
    fn remove_cascades_to_children() {
        let dag = TodoDag::new();
        dag.insert(make_item("root", "root")).unwrap();
        let mut child = make_item("child", "child");
        child.parent_id = Some("root".to_string());
        dag.insert(child).unwrap();
        let mut grandchild = make_item("grandchild", "gc");
        grandchild.parent_id = Some("child".to_string());
        dag.insert(grandchild).unwrap();

        let removed = dag.remove("root").unwrap();
        assert_eq!(removed.len(), 3);
        assert!(dag.is_empty());
    }

    // ── Parent hierarchy ─────────────────────────────────────────────

    #[test]
    fn parent_child_relationship() {
        let dag = TodoDag::new();
        dag.insert(make_item("parent", "p")).unwrap();
        let mut child = make_item("child1", "c1");
        child.parent_id = Some("parent".to_string());
        dag.insert(child).unwrap();

        let children = dag.children_of("parent");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].todo_id, "child1");
    }

    #[test]
    fn insert_with_missing_parent_fails() {
        let dag = TodoDag::new();
        let mut item = make_item("child", "c");
        item.parent_id = Some("nonexistent".to_string());
        let err = dag.insert(item).unwrap_err();
        assert!(matches!(err, TodoDagError::ParentNotFound(_)));
    }

    #[test]
    fn subtree_returns_bfs_order() {
        let dag = TodoDag::new();
        dag.insert(make_item("root", "r")).unwrap();
        let mut c1 = make_item("c1", "child 1");
        c1.parent_id = Some("root".to_string());
        dag.insert(c1).unwrap();
        let mut c2 = make_item("c2", "child 2");
        c2.parent_id = Some("root".to_string());
        dag.insert(c2).unwrap();
        let mut gc = make_item("gc1", "grandchild");
        gc.parent_id = Some("c1".to_string());
        dag.insert(gc).unwrap();

        let tree = dag.subtree("root");
        assert_eq!(tree.len(), 4);
        assert_eq!(tree[0].todo_id, "root");
    }

    #[test]
    fn roots_returns_parentless_items() {
        let dag = TodoDag::new();
        dag.insert(make_item("r1", "root 1")).unwrap();
        dag.insert(make_item("r2", "root 2")).unwrap();
        let mut child = make_item("c1", "child");
        child.parent_id = Some("r1".to_string());
        dag.insert(child).unwrap();

        let roots = dag.roots();
        assert_eq!(roots.len(), 2);
    }

    // ── Dependencies ─────────────────────────────────────────────────

    #[test]
    fn dependency_references() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "first")).unwrap();
        let mut t2 = make_item("t2", "second");
        t2.dependency_ids = vec!["t1".to_string()];
        dag.insert(t2).unwrap();

        let deps = dag.dependents_of("t1");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].todo_id, "t2");
    }

    #[test]
    fn insert_with_missing_dependency_fails() {
        let dag = TodoDag::new();
        let mut item = make_item("t1", "first");
        item.dependency_ids = vec!["nonexistent".to_string()];
        let err = dag.insert(item).unwrap_err();
        assert!(matches!(err, TodoDagError::DependencyNotFound(_)));
    }

    // ── Cycle detection ──────────────────────────────────────────────

    #[test]
    fn dependency_cycle_rejected() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "a")).unwrap();
        let mut t2 = make_item("t2", "b");
        t2.dependency_ids = vec!["t1".to_string()];
        dag.insert(t2).unwrap();

        // Try to add t1 -> t2 dependency (creating a cycle).
        let delta = make_delta(
            "t1",
            TodoChanges {
                add_dependencies: Some(vec!["t2".to_string()]),
                ..Default::default()
            },
        );
        let err = dag.apply_delta(&delta).unwrap_err();
        assert!(matches!(err, TodoDagError::CycleDetected(_)));
    }

    #[test]
    fn reparent_cycle_rejected() {
        let dag = TodoDag::new();
        dag.insert(make_item("parent", "p")).unwrap();
        let mut child = make_item("child", "c");
        child.parent_id = Some("parent".to_string());
        dag.insert(child).unwrap();

        // Try to reparent "parent" under "child" (cycle).
        let delta = make_delta(
            "parent",
            TodoChanges {
                parent_id: Some(Some("child".to_string())),
                ..Default::default()
            },
        );
        let err = dag.apply_delta(&delta).unwrap_err();
        assert!(matches!(err, TodoDagError::CycleDetected(_)));
    }

    // ── Delta application ────────────────────────────────────────────

    #[test]
    fn apply_status_change() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "a")).unwrap();

        let delta = make_delta(
            "t1",
            TodoChanges {
                status: Some(TodoStatus::Active),
                ..Default::default()
            },
        );
        dag.apply_delta(&delta).unwrap();
        assert_eq!(dag.get("t1").unwrap().status, TodoStatus::Active);
    }

    #[test]
    fn apply_ownership_change() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "a")).unwrap();

        let delta = make_delta(
            "t1",
            TodoChanges {
                owner_agent_id: Some(Some("agent-42".to_string())),
                ..Default::default()
            },
        );
        dag.apply_delta(&delta).unwrap();

        let item = dag.get("t1").unwrap();
        assert_eq!(item.owner_agent_id.as_deref(), Some("agent-42"));

        let owned = dag.by_owner("agent-42");
        assert_eq!(owned.len(), 1);
    }

    #[test]
    fn apply_add_and_remove_dependencies() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "a")).unwrap();
        dag.insert(make_item("t2", "b")).unwrap();
        dag.insert(make_item("t3", "c")).unwrap();

        // Add t1 as dependency of t3.
        let delta = make_delta(
            "t3",
            TodoChanges {
                add_dependencies: Some(vec!["t1".to_string(), "t2".to_string()]),
                ..Default::default()
            },
        );
        dag.apply_delta(&delta).unwrap();
        assert_eq!(dag.get("t3").unwrap().dependency_ids.len(), 2);

        // Remove t1.
        let delta2 = make_delta(
            "t3",
            TodoChanges {
                remove_dependencies: Some(vec!["t1".to_string()]),
                ..Default::default()
            },
        );
        dag.apply_delta(&delta2).unwrap();
        assert_eq!(dag.get("t3").unwrap().dependency_ids, vec!["t2"]);
    }

    #[test]
    fn apply_delta_to_missing_item_fails() {
        let dag = TodoDag::new();
        let delta = make_delta(
            "t1",
            TodoChanges {
                status: Some(TodoStatus::Done),
                ..Default::default()
            },
        );
        let err = dag.apply_delta(&delta).unwrap_err();
        assert!(matches!(err, TodoDagError::NotFound(_)));
    }

    #[test]
    fn apply_reparent() {
        let dag = TodoDag::new();
        dag.insert(make_item("p1", "parent 1")).unwrap();
        dag.insert(make_item("p2", "parent 2")).unwrap();
        let mut child = make_item("c1", "child");
        child.parent_id = Some("p1".to_string());
        dag.insert(child).unwrap();

        assert_eq!(dag.children_of("p1").len(), 1);
        assert_eq!(dag.children_of("p2").len(), 0);

        let delta = make_delta(
            "c1",
            TodoChanges {
                parent_id: Some(Some("p2".to_string())),
                ..Default::default()
            },
        );
        dag.apply_delta(&delta).unwrap();

        assert_eq!(dag.children_of("p1").len(), 0);
        assert_eq!(dag.children_of("p2").len(), 1);
        assert_eq!(dag.get("c1").unwrap().parent_id.as_deref(), Some("p2"));
    }

    // ── Query helpers ────────────────────────────────────────────────

    #[test]
    fn by_status_filters_correctly() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "a")).unwrap();
        dag.insert(make_item("t2", "b")).unwrap();

        let delta = make_delta(
            "t1",
            TodoChanges {
                status: Some(TodoStatus::Done),
                ..Default::default()
            },
        );
        dag.apply_delta(&delta).unwrap();

        assert_eq!(dag.by_status(TodoStatus::Done).len(), 1);
        assert_eq!(dag.by_status(TodoStatus::Pending).len(), 1);
        assert_eq!(dag.by_status(TodoStatus::Active).len(), 0);
    }

    // ── Serde round-trip ─────────────────────────────────────────────

    #[test]
    fn todo_item_serde_roundtrip() {
        let item = TodoItem {
            todo_id: "t1".to_string(),
            parent_id: Some("root".to_string()),
            title: "test item".to_string(),
            details: Some("some details".to_string()),
            status: TodoStatus::Active,
            priority: 50,
            owner_agent_id: Some("agent-1".to_string()),
            dependency_ids: vec!["t0".to_string()],
            created_at: 1000,
            updated_at: 2000,
            progress: Some(0.5),
            verification_result: Some("ok".to_string()),
            source: TodoSource::Provider {
                provider_name: "claude".to_string(),
            },
        };
        let json = serde_json::to_string(&item).unwrap();
        let decoded: TodoItem = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, item);
    }

    #[test]
    fn todo_status_serde_all_variants() {
        let variants = [
            TodoStatus::Pending,
            TodoStatus::Active,
            TodoStatus::Blocked,
            TodoStatus::Done,
            TodoStatus::Failed,
            TodoStatus::Cancelled,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: TodoStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v, "roundtrip failed for {v:?}");
            assert_eq!(json, format!("\"{}\"", v.as_str()));
        }
    }

    #[test]
    fn todo_source_serde_roundtrip() {
        let sources = [
            TodoSource::Operator,
            TodoSource::Provider {
                provider_name: "claude".to_string(),
            },
            TodoSource::System,
        ];
        for s in &sources {
            let json = serde_json::to_string(s).unwrap();
            let back: TodoSource = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, s, "roundtrip failed for {s:?}");
        }
    }

    #[test]
    fn todo_snapshot_serde_roundtrip() {
        let snap = TodoSnapshot {
            run_id: "run-1".to_string(),
            turn_id: "turn-1".to_string(),
            sequence: 42,
            timestamp: 5000,
            items: vec![make_item("t1", "a")],
        };
        let json = serde_json::to_string(&snap).unwrap();
        let decoded: TodoSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, snap);
    }

    #[test]
    fn todo_delta_serde_roundtrip() {
        let delta = TodoDelta {
            run_id: "run-1".to_string(),
            turn_id: "turn-1".to_string(),
            sequence: 1,
            timestamp: 2000,
            todo_id: "t1".to_string(),
            changes: TodoChanges {
                status: Some(TodoStatus::Done),
                title: Some("new title".to_string()),
                progress: Some(Some(1.0)),
                ..Default::default()
            },
        };
        let json = serde_json::to_string(&delta).unwrap();
        let decoded: TodoDelta = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, delta);
    }

    #[test]
    fn todo_changes_empty_check() {
        assert!(TodoChanges::default().is_empty());
        let non_empty = TodoChanges {
            status: Some(TodoStatus::Active),
            ..Default::default()
        };
        assert!(!non_empty.is_empty());
    }

    // ── Snapshot + restore ───────────────────────────────────────────

    #[test]
    fn snapshot_and_restore() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "a")).unwrap();
        dag.insert(make_item("t2", "b")).unwrap();
        let mut child = make_item("c1", "child");
        child.parent_id = Some("t1".to_string());
        dag.insert(child).unwrap();

        let snap = dag.snapshot("run-1".into(), "turn-1".into(), 1);
        assert_eq!(snap.items.len(), 3);

        // Restore into a fresh DAG.
        let dag2 = TodoDag::new();
        dag2.restore_from_snapshot(&snap).unwrap();
        assert_eq!(dag2.len(), 3);
        assert_eq!(dag2.children_of("t1").len(), 1);
    }

    #[test]
    fn restore_rejects_cycle_in_snapshot() {
        // Build a snapshot with a parent cycle (t1 -> t2 -> t1).
        let snap = TodoSnapshot {
            run_id: "r".to_string(),
            turn_id: "t".to_string(),
            sequence: 0,
            timestamp: 0,
            items: vec![
                TodoItem {
                    todo_id: "t1".to_string(),
                    parent_id: Some("t2".to_string()),
                    title: "a".to_string(),
                    details: None,
                    status: TodoStatus::Pending,
                    priority: 0,
                    owner_agent_id: None,
                    dependency_ids: vec![],
                    created_at: 0,
                    updated_at: 0,
                    progress: None,
                    verification_result: None,
                    source: TodoSource::System,
                },
                TodoItem {
                    todo_id: "t2".to_string(),
                    parent_id: Some("t1".to_string()),
                    title: "b".to_string(),
                    details: None,
                    status: TodoStatus::Pending,
                    priority: 0,
                    owner_agent_id: None,
                    dependency_ids: vec![],
                    created_at: 0,
                    updated_at: 0,
                    progress: None,
                    verification_result: None,
                    source: TodoSource::System,
                },
            ],
        };
        let dag = TodoDag::new();
        let err = dag.restore_from_snapshot(&snap).unwrap_err();
        assert!(matches!(err, TodoDagError::CycleDetected(_)));
        assert!(dag.is_empty());
    }

    // ── Replay from deltas ───────────────────────────────────────────

    #[test]
    fn replay_deltas_sequence() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "first")).unwrap();
        dag.insert(make_item("t2", "second")).unwrap();

        let deltas = vec![
            make_delta(
                "t1",
                TodoChanges {
                    status: Some(TodoStatus::Active),
                    owner_agent_id: Some(Some("agent-1".to_string())),
                    ..Default::default()
                },
            ),
            make_delta(
                "t2",
                TodoChanges {
                    add_dependencies: Some(vec!["t1".to_string()]),
                    ..Default::default()
                },
            ),
            make_delta(
                "t1",
                TodoChanges {
                    status: Some(TodoStatus::Done),
                    progress: Some(Some(1.0)),
                    ..Default::default()
                },
            ),
        ];

        dag.replay_deltas(&deltas).unwrap();

        let t1 = dag.get("t1").unwrap();
        assert_eq!(t1.status, TodoStatus::Done);
        assert_eq!(t1.progress, Some(1.0));
        assert_eq!(t1.owner_agent_id.as_deref(), Some("agent-1"));

        let t2 = dag.get("t2").unwrap();
        assert_eq!(t2.dependency_ids, vec!["t1"]);
    }

    // ── Concurrent access ────────────────────────────────────────────

    #[test]
    fn concurrent_reads_and_writes() {
        use std::sync::Arc;

        let dag = Arc::new(TodoDag::new());
        dag.insert(make_item("t1", "shared")).unwrap();

        let handles: Vec<_> = (0..8)
            .map(|i| {
                let dag = Arc::clone(&dag);
                std::thread::spawn(move || {
                    // Mix reads and writes.
                    if i % 2 == 0 {
                        let _ = dag.get("t1");
                        let _ = dag.by_status(TodoStatus::Pending);
                        let _ = dag.roots();
                    } else {
                        let delta = TodoDelta {
                            run_id: format!("run-{i}"),
                            turn_id: format!("turn-{i}"),
                            sequence: i as u64,
                            timestamp: 1000 + i as i64,
                            todo_id: "t1".to_string(),
                            changes: TodoChanges {
                                priority: Some(i as u32),
                                ..Default::default()
                            },
                        };
                        let _ = dag.apply_delta(&delta);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // DAG should still be consistent.
        assert_eq!(dag.len(), 1);
        assert!(dag.validate().is_empty());
    }

    // ── TodoStatus helpers ───────────────────────────────────────────

    #[test]
    fn status_is_terminal() {
        assert!(!TodoStatus::Pending.is_terminal());
        assert!(!TodoStatus::Active.is_terminal());
        assert!(!TodoStatus::Blocked.is_terminal());
        assert!(TodoStatus::Done.is_terminal());
        assert!(TodoStatus::Failed.is_terminal());
        assert!(TodoStatus::Cancelled.is_terminal());
    }

    #[test]
    fn status_icons_distinct() {
        let icons: Vec<_> = [
            TodoStatus::Pending,
            TodoStatus::Active,
            TodoStatus::Blocked,
            TodoStatus::Done,
            TodoStatus::Failed,
            TodoStatus::Cancelled,
        ]
        .iter()
        .map(|s| s.icon())
        .collect();
        let unique: std::collections::HashSet<_> = icons.iter().copied().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // ── Validate ─────────────────────────────────────────────────────

    #[test]
    fn validate_clean_dag() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "a")).unwrap();
        dag.insert(make_item("t2", "b")).unwrap();
        assert!(dag.validate().is_empty());
    }

    // ── Remove cleans dependency refs ────────────────────────────────

    #[test]
    fn remove_cleans_dependency_references() {
        let dag = TodoDag::new();
        dag.insert(make_item("t1", "a")).unwrap();
        let mut t2 = make_item("t2", "b");
        t2.dependency_ids = vec!["t1".to_string()];
        dag.insert(t2).unwrap();

        dag.remove("t1").unwrap();

        let t2 = dag.get("t2").unwrap();
        assert!(
            t2.dependency_ids.is_empty(),
            "stale dep ref should be cleaned"
        );
    }
}
