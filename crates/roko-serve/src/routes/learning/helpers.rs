//! Shared helper functions for learning submodules.

use roko_learn::prompt_experiment::ExperimentStore;

use crate::error::ApiError;

/// Read and parse the persisted experiment store.
pub async fn read_experiment_store(path: &std::path::Path) -> Result<ExperimentStore, ApiError> {
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ExperimentStore::new());
        }
        Err(e) => {
            return Err(ApiError::internal(format!("read {}: {e}", path.display())));
        }
    };

    serde_json::from_str(&content)
        .map_err(|e| ApiError::internal(format!("parse {}: {e}", path.display())))
}
