//! Copy-on-write helpers for speculative execution and scenario branching.

use std::{collections::HashMap, num::NonZeroUsize, sync::Arc};

use alloy_primitives::{Address, B256, U256};
use lru::LruCache;
use parking_lot::Mutex;

use crate::Bytecode;

/// Shared copy-on-write storage overlay.
#[derive(Debug, Clone, Default)]
pub struct CowState {
    baseline: Arc<HashMap<(Address, U256), U256>>,
    overlay: HashMap<(Address, U256), U256>,
}

impl CowState {
    /// Creates a branch from a frozen baseline map.
    #[must_use]
    pub fn branch(baseline: Arc<HashMap<(Address, U256), U256>>) -> Self {
        Self {
            baseline,
            overlay: HashMap::new(),
        }
    }

    /// Reads a storage slot from the overlay or baseline.
    #[must_use]
    pub fn read(&self, address: Address, slot: U256) -> Option<U256> {
        self.overlay
            .get(&(address, slot))
            .or_else(|| self.baseline.get(&(address, slot)))
            .copied()
    }

    /// Writes a storage slot into the overlay.
    pub fn write(&mut self, address: Address, slot: U256, value: U256) {
        self.overlay.insert((address, slot), value);
    }

    /// Returns the number of modified slots in this branch.
    #[must_use]
    pub fn overlay_size(&self) -> usize {
        self.overlay.len()
    }

    /// Returns a reference to the overlay (speculative writes only).
    #[must_use]
    pub fn overlay_ref(&self) -> &HashMap<(Address, U256), U256> {
        &self.overlay
    }

    /// Returns the shared baseline.
    #[must_use]
    pub fn baseline_arc(&self) -> &Arc<HashMap<(Address, U256), U256>> {
        &self.baseline
    }
}

/// Shared bytecode cache keyed by code hash.
#[derive(Debug)]
pub struct BytecodeCache {
    cache: LruCache<B256, Bytecode>,
}

impl BytecodeCache {
    /// Creates a new bytecode cache.
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            cache: LruCache::new(capacity),
        }
    }

    /// Retrieves bytecode from the cache.
    #[must_use]
    pub fn get(&mut self, code_hash: &B256) -> Option<Bytecode> {
        self.cache.get(code_hash).cloned()
    }

    /// Inserts bytecode into the cache.
    pub fn insert(&mut self, code_hash: B256, bytecode: Bytecode) {
        self.cache.put(code_hash, bytecode);
    }

    /// Returns the current entry count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

/// Per-slot multi-version storage used by the simplified Block-STM test harness.
#[derive(Debug, Default)]
pub struct MultiVersionStore {
    /// Slot versions indexed by `(address, slot)`.
    pub versions: dashmap::DashMap<(Address, U256), Vec<VersionEntry>>,
}

impl MultiVersionStore {
    /// Records a new version entry.
    pub fn record(&self, address: Address, slot: U256, entry: VersionEntry) {
        self.versions
            .entry((address, slot))
            .or_default()
            .push(entry);
    }

    /// Materializes the latest value for each tracked slot.
    #[must_use]
    pub fn materialize(&self) -> HashMap<(Address, U256), U256> {
        self.versions
            .iter()
            .filter_map(|entry| {
                entry
                    .value()
                    .iter()
                    .max_by_key(|version| (version.tx_index, version.incarnation))
                    .map(|version| (*entry.key(), version.value))
            })
            .collect()
    }
}

/// One version of a slot written during optimistic execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionEntry {
    /// Transaction index within the block.
    pub tx_index: usize,
    /// Value written by that execution incarnation.
    pub value: U256,
    /// Re-execution counter.
    pub incarnation: u32,
}

/// Shared bytecode cache handle type.
pub(crate) type SharedBytecodeCache = Arc<Mutex<BytecodeCache>>;

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, num::NonZeroUsize, sync::Arc};

    use alloy_primitives::{U256, address, bytes};

    use super::{BytecodeCache, CowState};
    use crate::Bytecode;

    #[test]
    fn cow_branches_share_baseline() {
        let address = address!("0x1000000000000000000000000000000000000000");
        let slot = U256::from(1);
        let baseline = Arc::new(HashMap::from([((address, slot), U256::from(7))]));
        let left = CowState::branch(Arc::clone(&baseline));
        let right = CowState::branch(baseline);

        assert_eq!(left.read(address, slot), Some(U256::from(7)));
        assert_eq!(right.read(address, slot), Some(U256::from(7)));
    }

    #[test]
    fn cow_branches_are_isolated() {
        let address = address!("0x2000000000000000000000000000000000000000");
        let slot = U256::from(1);
        let baseline = Arc::new(HashMap::new());
        let mut left = CowState::branch(Arc::clone(&baseline));
        let right = CowState::branch(baseline);

        left.write(address, slot, U256::from(9));

        assert_eq!(left.read(address, slot), Some(U256::from(9)));
        assert_eq!(right.read(address, slot), None);
    }

    #[test]
    fn cow_overlay_wins_over_baseline() {
        let address = address!("0x2100000000000000000000000000000000000000");
        let slot = U256::from(7);
        let baseline = Arc::new(HashMap::from([((address, slot), U256::from(3))]));
        let mut branch = CowState::branch(baseline);

        branch.write(address, slot, U256::from(11));

        assert_eq!(branch.read(address, slot), Some(U256::from(11)));
        assert_eq!(branch.overlay_size(), 1);
    }

    #[test]
    fn cow_memory_scales_with_overlays() {
        let address = address!("0x3000000000000000000000000000000000000000");
        let baseline = Arc::new(
            (0..50_000)
                .map(|index| ((address, U256::from(index)), U256::from(index)))
                .collect::<HashMap<_, _>>(),
        );
        let mut branch = CowState::branch(baseline);
        for index in 0..200 {
            branch.write(address, U256::from(index), U256::from(index + 1));
        }

        assert_eq!(branch.overlay_size(), 200);
    }

    #[test]
    fn bytecode_cache_no_ttl() {
        let code = Bytecode::new_raw(bytes!("6001600055"));
        let hash = code.hash_slow();
        let mut cache = BytecodeCache::new(NonZeroUsize::MIN);
        cache.insert(hash, code.clone());

        assert_eq!(cache.get(&hash), Some(code));
    }
}
