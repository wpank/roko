//! API key and agent token management routes.
//!
//! ## API key routes
//! - `POST   /api/api-keys`               — create a new named API key
//! - `GET    /api/api-keys`               — list all keys (metadata only, never the key itself)
//! - `DELETE /api/api-keys/:name`         — revoke a key by name
//! - `POST   /api/api-keys/:name/rotate`  — rotate: new key, 5-min grace for old
//!
//! ## Agent token routes (T02)
//! - `POST   /api/agent-tokens`           — issue a scoped agent bearer token
//! - `GET    /api/agent-tokens`           — list active tokens (metadata only)
//! - `DELETE /api/agent-tokens/:token_id` — revoke an agent token
//!
//! ## Relay token routes (E35-T06)
//! - `POST   /api/relay-tokens`           — issue a narrowed, parent-linked delegation
//! - `DELETE /api/relay-tokens/:token_id` — revoke a delegation and its descendants
//!
//! Keys are stored as SHA-256 hashes in `.roko/api-keys.json`.
//! Agent tokens are stored in `.roko/agent-tokens.json`.
//! Relay tokens are stored in `.roko/relay-tokens.json`.
//! Other processes (a second server on the same workspace, a key-management
//! CLI) may change these files while the server runs, so every writer must
//! hold the file's sibling `.lock` file across a read-merge-write of its one
//! change; `roko_fs::with_locked_json_transaction` does exactly that.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::http::header::{AUTHORIZATION, HeaderMap};
use axum::routing::{delete, get, post};
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use roko_core::config::ApiKeyEntry;
use roko_fs::with_locked_json_transaction;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::auth_audit::{AuthAuditAction, AuthAuditEvent, AuthOutcome};

use super::middleware::AuthContext;
use super::middleware::hash_api_key;
use crate::error::ApiError;
use crate::state::AppState;

// ─── API key types ─────────────────────────────────────────────────────────

/// Request payload for creating a new API key.
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    /// Human-readable name for the key (must be unique).
    pub name: String,
    /// Permission scope: "admin", "agent:write", "read", etc.
    #[serde(default = "default_scope")]
    pub scope: String,
    /// Optional ISO 8601 expiry timestamp.
    #[serde(default)]
    pub expires_at: Option<String>,
}

fn default_scope() -> String {
    "admin".into()
}

fn validate_api_key_scope(scope: &str) -> Result<(), ApiError> {
    const VALID_SCOPES: &[&str] = &[
        "admin",
        "write",
        "read",
        "agent:write",
        "plan:write",
        "terminal:write",
    ];
    if VALID_SCOPES.contains(&scope) {
        Ok(())
    } else {
        Err(ApiError::bad_request(format!(
            "scope must be one of: {}",
            VALID_SCOPES.join(", ")
        )))
    }
}

/// Response returned when a new key is created or rotated.
/// The plaintext key is returned **once** and never stored.
#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub name: String,
    pub key: String,
    pub scope: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Summary of a stored key (no secret material).
#[derive(Debug, Serialize)]
pub struct ApiKeySummary {
    pub name: String,
    pub scope: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
}

// ─── Agent token types (T02) ────────────────────────────────────────────────

/// Capability granted to an agent token.
///
/// Each variant maps to a coarse scope used by the auth middleware to decide
/// whether a bearer request is allowed to access a given route.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AgentCapability {
    /// Submit inference requests to the LLM backends.
    Inference,
    /// Invoke registered tools on behalf of an agent.
    Tools,
    /// Publish messages to the internal event bus.
    BusPublish,
    /// Write to the knowledge / neuro store.
    StoreWrite,
    /// Read from the knowledge / neuro store.
    StoreRead,
}

/// Persisted agent token record (secrets stored as SHA-256 hashes only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToken {
    /// Unique identifier for the token.
    pub token_id: String,
    /// The agent this token was issued for.
    pub agent_id: String,
    /// Capabilities scoped to this token.
    pub capabilities: Vec<AgentCapability>,
    /// ISO 8601 issuance timestamp.
    pub issued_at: String,
    /// ISO 8601 expiry timestamp.
    pub expires_at: DateTime<Utc>,
    /// Whether the token has been explicitly revoked.
    pub revoked: bool,
    /// SHA-256 hex hash of the plaintext token (never stored in plaintext).
    pub token_hash: String,
}

/// Request payload for issuing a new agent token.
#[derive(Debug, Deserialize)]
pub struct IssueAgentTokenRequest {
    /// The agent ID this token is scoped to.
    pub agent_id: String,
    /// Capabilities granted to this token.
    pub capabilities: Vec<AgentCapability>,
    /// How many seconds until the token expires (default: 86400 = 24 h).
    #[serde(default = "default_token_ttl_secs")]
    pub ttl_secs: i64,
}

fn default_token_ttl_secs() -> i64 {
    86400
}

/// Response returned when a new agent token is issued.
/// The plaintext secret is shown **once**.
#[derive(Debug, Serialize)]
pub struct IssueAgentTokenResponse {
    pub token_id: String,
    pub token_secret: String,
    pub agent_id: String,
    pub capabilities: Vec<AgentCapability>,
    pub expires_at: String,
}

/// Summary of an agent token (no secret material).
#[derive(Debug, Serialize)]
pub struct AgentTokenSummary {
    pub token_id: String,
    pub agent_id: String,
    pub capabilities: Vec<AgentCapability>,
    pub issued_at: String,
    pub expires_at: String,
    pub revoked: bool,
}

// ─── Shared credential registry ─────────────────────────────────────────────

/// A key's `last_used_at` is persisted at most this often. Every API-key
/// request records a use; rewriting the key file each time would queue all
/// API-key traffic behind the file lock and its fsyncs.
const API_KEY_USE_PERSIST_INTERVAL_SECS: i64 = 60;

/// In-memory credential registry backed by atomically replaced JSON files.
///
/// All read-modify-write operations hold the corresponding lock until the
/// replacement file is durable, preventing `last_used_at` updates from
/// clobbering concurrent rotations or revocations.
///
/// The files may also change on disk while the server runs. Each write is a
/// locked read-merge-write of one change against the file as it is now
/// (`update_api_keys`, `RegistryCache::update`), and lookups re-read a file
/// when it has changed, so a credential another process adds works at once
/// and one it revokes stops working at once.
pub(crate) struct AuthRegistry {
    workdir: PathBuf,
    api_keys: RwLock<ApiKeyState>,
    agent_tokens: RwLock<RegistryCache<AgentToken>>,
    relay_tokens: RwLock<RegistryCache<RelayToken>>,
}

/// This process's cached copy of a token registry file.
struct RegistryCache<T> {
    path: PathBuf,
    entries: Vec<T>,
    /// The file version `entries` was read from (`Some(None)`: there was no
    /// file). `None` until the first lookup reads the file, and again after
    /// this process writes it.
    read_from: Option<Option<FileStamp>>,
}

impl<T> RegistryCache<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    fn new(path: PathBuf, entries: Vec<T>) -> Self {
        Self {
            path,
            entries,
            read_from: None,
        }
    }

    /// Apply one change to the file as a locked read-merge-write, then cache
    /// the file's new contents. `change` sees the file as it is on disk now,
    /// so tokens and revocations written by another process are kept.
    async fn update<R, F>(&mut self, change: F) -> Result<R, ApiError>
    where
        R: Send + 'static,
        F: FnOnce(&mut Vec<T>) -> Result<R, ApiError> + Send + 'static,
    {
        let (result, entries) = run_registry_change(self.path.clone(), change).await?;
        self.entries = entries;
        self.read_from = None;
        Ok(result)
    }
}

/// Re-read a token file into its cache if another process replaced it since
/// this process last read or wrote it. Writers replace the file by rename, so
/// a plain read sees one whole version and needs no lock.
async fn refresh_registry<T>(lock: &RwLock<RegistryCache<T>>) -> Result<(), ApiError>
where
    T: for<'de> Deserialize<'de>,
{
    {
        let cache = lock.read().await;
        if cache.read_from == Some(registry_file_stamp(&cache.path)?) {
            return Ok(());
        }
    }
    let mut cache = lock.write().await;
    let (entries, stamp) = read_registry_file(&cache.path)?;
    cache.entries = entries;
    cache.read_from = Some(stamp);
    Ok(())
}

/// This process's cached view of `.roko/api-keys.json`.
struct ApiKeyState {
    /// The file's keys, then `unseeded`.
    entries: Vec<ApiKeyEntry>,
    /// Configured `[serve.auth.api_keys]` entries the file does not have yet.
    /// The next write adds them, as the old whole-list writes did. Once this
    /// process sees a key in the file, only the file decides whether it
    /// exists, so a later revocation is not undone from configuration.
    unseeded: Vec<ApiKeyEntry>,
    /// The file version `entries` was read from (`Some(None)`: there was no
    /// file). `None` after this process writes, so the next lookup re-reads.
    read_from: Option<Option<FileStamp>>,
}

impl ApiKeyState {
    fn new(
        file_keys: Vec<ApiKeyEntry>,
        stamp: Option<FileStamp>,
        configured_keys: &[ApiKeyEntry],
    ) -> Self {
        let mut entries = file_keys;
        let on_disk = entries.len();
        seed_configured_keys(&mut entries, configured_keys);
        let unseeded = entries[on_disk..].to_vec();
        Self {
            entries,
            unseeded,
            read_from: Some(stamp),
        }
    }

    /// Replace the cached keys with `keys`, the file's current contents.
    fn absorb(&mut self, keys: Vec<ApiKeyEntry>, read_from: Option<Option<FileStamp>>) {
        // Once the file has a configured key, the file alone decides its fate.
        self.unseeded.retain(|key| !has_key(&keys, &key.name));
        let mut entries = keys;
        seed_configured_keys(&mut entries, &self.unseeded);
        self.entries = entries;
        self.read_from = read_from;
    }
}

impl AuthRegistry {
    /// Load persisted credentials and merge configured named keys that are not
    /// already present on disk. Invalid registry JSON fails server startup
    /// rather than silently disabling every credential.
    pub(crate) fn load(workdir: &Path, configured_keys: &[ApiKeyEntry]) -> anyhow::Result<Self> {
        let (api_keys, stamp) = read_registry_file(&api_keys_path(workdir))?;
        let agents_path = agent_tokens_path(workdir);
        let relays_path = relay_tokens_path(workdir);
        let agent_tokens = load_registry_file(&agents_path)?;
        let relay_tokens = load_relay_registry(workdir)?;
        Ok(Self {
            workdir: workdir.to_path_buf(),
            api_keys: RwLock::new(ApiKeyState::new(api_keys, stamp, configured_keys)),
            agent_tokens: RwLock::new(RegistryCache::new(agents_path, agent_tokens)),
            relay_tokens: RwLock::new(RegistryCache::new(relays_path, relay_tokens)),
        })
    }

    /// Keys to authenticate against, re-read first if the file has changed.
    ///
    /// A key file that cannot be read or parsed cannot vouch for any named
    /// key, so this fails closed with an empty list.
    pub(crate) async fn api_keys_snapshot(&self) -> Vec<ApiKeyEntry> {
        match self.current_api_keys().await {
            Ok(keys) => keys,
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "API-key registry is unreadable; rejecting all named API keys"
                );
                Vec::new()
            }
        }
    }

    /// The current API keys: the cached copy while the file is unchanged,
    /// otherwise a fresh read.
    ///
    /// Writers replace the file by rename, so a plain read sees one whole
    /// version and needs no lock.
    async fn current_api_keys(&self) -> Result<Vec<ApiKeyEntry>, ApiError> {
        let path = api_keys_path(&self.workdir);
        let on_disk = registry_file_stamp(&path)?;
        {
            let state = self.api_keys.read().await;
            if state.read_from == Some(on_disk) {
                return Ok(state.entries.clone());
            }
        }
        let mut state = self.api_keys.write().await;
        let (keys, stamp) = read_registry_file(&path)?;
        state.absorb(keys, Some(stamp));
        Ok(state.entries.clone())
    }

    /// Apply one change to the API-key file as a locked read-merge-write.
    ///
    /// `change` sees the file as it is on disk now, plus configured keys not
    /// written yet, never this process's cached copy, so keys another process
    /// added or revoked stay that way. The cache is replaced with the result.
    /// If the file cannot be read or parsed, or `change` fails, nothing is
    /// written.
    async fn update_api_keys<R, F>(&self, change: F) -> Result<R, ApiError>
    where
        R: Send + 'static,
        F: FnOnce(&mut Vec<ApiKeyEntry>) -> Result<R, ApiError> + Send + 'static,
    {
        // Held across the file transaction so this process's writes apply in order.
        let mut state = self.api_keys.write().await;
        let unseeded = state.unseeded.clone();
        let (result, keys) = run_registry_change(api_keys_path(&self.workdir), move |keys| {
            seed_configured_keys(keys, &unseeded);
            change(keys)
        })
        .await?;
        // The file now has every configured key that `change` kept.
        state.unseeded.clear();
        state.absorb(keys, None);
        Ok(result)
    }

    pub(crate) async fn insert_api_key(&self, entry: ApiKeyEntry) -> Result<(), ApiError> {
        self.update_api_keys(move |keys| {
            if has_key(keys, &entry.name) {
                return Err(ApiError::conflict(format!(
                    "API key with name '{}' already exists",
                    entry.name
                )));
            }
            keys.push(entry);
            Ok(())
        })
        .await
    }

    async fn remove_api_key(&self, name: &str) -> Result<bool, ApiError> {
        let name = name.to_string();
        self.update_api_keys(move |keys| {
            let before = keys.len();
            keys.retain(|entry| entry.name != name);
            Ok(keys.len() != before)
        })
        .await
    }

    async fn rotate_api_key(
        &self,
        name: &str,
        new_hash: String,
        created_at: String,
        grace_expires: String,
    ) -> Result<ApiKeyEntry, ApiError> {
        let name = name.to_string();
        self.update_api_keys(move |keys| {
            let entry = keys
                .iter_mut()
                .find(|entry| entry.name == name)
                .ok_or_else(|| {
                    ApiError::not_found(format!("API key with name '{name}' not found"))
                })?;

            let now = Utc::now();
            entry
                .previous_key_hashes
                .retain(|(_, expiry)| parse_rfc3339(expiry).is_some_and(|expiry| expiry > now));
            entry
                .previous_key_hashes
                .push((entry.key_hash.clone(), grace_expires));
            if entry.previous_key_hashes.len() > 2 {
                let excess = entry.previous_key_hashes.len() - 2;
                entry.previous_key_hashes.drain(..excess);
            }
            entry.key_hash = new_hash;
            entry.created_at = created_at;
            entry.last_used_at = None;
            Ok(entry.clone())
        })
        .await
    }

    pub(crate) async fn record_api_key_use(&self, name: &str) -> Result<(), ApiError> {
        let now = Utc::now();
        let interval = Duration::seconds(API_KEY_USE_PERSIST_INTERVAL_SECS);
        let recorded_recently = move |entry: &ApiKeyEntry| {
            entry
                .last_used_at
                .as_deref()
                .and_then(parse_rfc3339)
                .is_some_and(|last| now - last < interval)
        };
        let already_recorded = self
            .api_keys
            .read()
            .await
            .entries
            .iter()
            .any(|entry| entry.name == name && recorded_recently(entry));
        if already_recorded {
            return Ok(());
        }
        let name = name.to_string();
        self.update_api_keys(move |keys| {
            let unrecorded = keys
                .iter_mut()
                .find(|entry| entry.name == name && !recorded_recently(entry));
            if let Some(entry) = unrecorded {
                entry.last_used_at = Some(now.to_rfc3339());
            }
            Ok(())
        })
        .await
    }

    pub(crate) async fn insert_agent_token(&self, token: AgentToken) -> Result<(), ApiError> {
        let mut agents = self.agent_tokens.write().await;
        agents
            .update(move |tokens| {
                tokens.push(token);
                Ok(())
            })
            .await
    }

    /// The current agent tokens, re-read first if the file has changed.
    async fn current_agent_tokens(&self) -> Result<Vec<AgentToken>, ApiError> {
        refresh_registry(&self.agent_tokens).await?;
        Ok(self.agent_tokens.read().await.entries.clone())
    }

    #[cfg(test)]
    async fn agent_tokens_snapshot(&self) -> Vec<AgentToken> {
        self.current_agent_tokens()
            .await
            .expect("agent-token registry is readable")
    }

    async fn revoke_agent_token(&self, token_id: &str) -> Result<Option<String>, ApiError> {
        // Hold both caches, agent tokens first, so an issuance in this process
        // cannot interleave with the revocation and its cascade.
        let mut agents = self.agent_tokens.write().await;
        let mut relays = self.relay_tokens.write().await;
        let id = token_id.to_string();
        let agent_id = agents
            .update(move |tokens| {
                let Some(token) = tokens.iter_mut().find(|token| token.token_id == id) else {
                    return Ok(None);
                };
                token.revoked = true;
                Ok(Some(token.agent_id.clone()))
            })
            .await?;
        let Some(agent_id) = agent_id else {
            return Ok(None);
        };
        // The root revocation is durable and cached now. Full-chain validation
        // fails closed even if the best-effort materialized cascade below fails.
        let id = token_id.to_string();
        relays
            .update(move |tokens| {
                cascade_relay_revocation(tokens, &id);
                Ok(())
            })
            .await?;
        Ok(Some(agent_id))
    }
}

fn load_registry_file<T: for<'de> Deserialize<'de>>(path: &Path) -> anyhow::Result<Vec<T>> {
    read_registry_file(path).map(|(entries, _)| entries)
}

/// Read a registry file and the version read (`None`: there is no file).
///
/// The version comes from the open handle, so it describes these bytes even
/// if another process replaces the file meanwhile.
fn read_registry_file<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> anyhow::Result<(Vec<T>, Option<FileStamp>)> {
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Vec::new(), None));
        }
        Err(error) => return Err(anyhow::anyhow!("read {}: {error}", path.display())),
    };
    let stamp = file
        .metadata()
        .map(|metadata| FileStamp::of(&metadata))
        .map_err(|error| anyhow::anyhow!("read {}: {error}", path.display()))?;
    let mut data = String::new();
    file.read_to_string(&mut data)
        .map_err(|error| anyhow::anyhow!("read {}: {error}", path.display()))?;
    let entries = serde_json::from_str(&data)
        .map_err(|error| anyhow::anyhow!("parse {}: {error}", path.display()))?;
    Ok((entries, Some(stamp)))
}

/// The version of a registry file now on disk (`None`: there is no file).
fn registry_file_stamp(path: &Path) -> Result<Option<FileStamp>, ApiError> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(FileStamp::of(&metadata))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ApiError::internal(format!(
            "read {}: {error}",
            path.display()
        ))),
    }
}

/// Identifies one version of a registry file. Writers rename a new file over
/// the old one, so a new version also gets a new inode, not only a new mtime.
#[derive(Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    inode: u64,
}

impl FileStamp {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            inode: std::os::unix::fs::MetadataExt::ino(metadata),
        }
    }
}

fn has_key(keys: &[ApiKeyEntry], name: &str) -> bool {
    keys.iter().any(|entry| entry.name == name)
}

/// Add each configured key that `keys` lacks. On a name clash the entry
/// already in `keys` wins, as the file's entry does at startup.
fn seed_configured_keys(keys: &mut Vec<ApiKeyEntry>, configured: &[ApiKeyEntry]) {
    for entry in configured {
        if !has_key(keys, &entry.name) {
            keys.push(entry.clone());
        }
    }
}

/// Why a locked API-key file update did not apply.
enum RegistryUpdateError {
    /// The file could not be locked, read, parsed or written.
    Io(std::io::Error),
    /// The change itself was refused, for example a duplicate name.
    Rejected(ApiError),
}

impl From<std::io::Error> for RegistryUpdateError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Run `change` against the registry file at `path` as a locked
/// read-merge-write on the blocking pool. Returns the change's result and the
/// file's new contents.
async fn run_registry_change<T, R, F>(path: PathBuf, change: F) -> Result<(R, Vec<T>), ApiError>
where
    T: Clone + Serialize + for<'de> Deserialize<'de> + Send + 'static,
    R: Send + 'static,
    F: FnOnce(&mut Vec<T>) -> Result<R, ApiError> + Send + 'static,
{
    let task_path = path.clone();
    tokio::task::spawn_blocking(move || apply_registry_change(&task_path, change))
        .await
        .map_err(|error| ApiError::internal(format!("update {}: {error}", path.display())))?
}

/// Run one change against a registry file while holding its sibling lock.
/// Blocking: call it from `spawn_blocking`.
fn apply_registry_change<T, R>(
    path: &Path,
    change: impl FnOnce(&mut Vec<T>) -> Result<R, ApiError>,
) -> Result<(R, Vec<T>), ApiError>
where
    T: Clone + Serialize + for<'de> Deserialize<'de>,
{
    with_locked_json_transaction::<Vec<T>, _, RegistryUpdateError, _>(path, |entries| {
        let result = change(entries).map_err(RegistryUpdateError::Rejected)?;
        Ok((result, entries.clone()))
    })
    .map_err(|error| match error {
        RegistryUpdateError::Io(error) => {
            ApiError::internal(format!("update {}: {error}", path.display()))
        }
        RegistryUpdateError::Rejected(error) => error,
    })
}

pub(crate) fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn generate_secret(prefix: &str) -> String {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!(
        "{prefix}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

// Relay token delegation types (E35-T06)

/// Default relay token TTL: 5 minutes.
const DEFAULT_RELAY_TOKEN_TTL_SECS: i64 = 300;
const MAX_RELAY_MAX_DEPTH: u8 = 12;
const DEFAULT_RELAY_MAX_DEPTH: u8 = MAX_RELAY_MAX_DEPTH;

/// A persisted, parent-linked capability delegation. Secrets are hashed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelayToken {
    pub token_id: String,
    pub parent_token_id: String,
    pub issuer_agent_id: String,
    pub delegated_capabilities: Vec<AgentCapability>,
    pub target_agent_id: String,
    /// Absolute maximum chain depth. `max_depth - depth` is the remaining
    /// delegation budget and therefore decreases at every hop.
    pub max_depth: u8,
    /// This token's one-based depth below its root agent token.
    pub depth: u8,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub token_hash: String,
}

/// Authenticated agent or relay claims passed from middleware to handlers.
#[derive(Debug, Clone)]
pub(crate) struct AgentCredentialClaims {
    pub token_id: String,
    pub agent_id: String,
    pub capabilities: Vec<AgentCapability>,
    pub expires_at: DateTime<Utc>,
    pub depth: u8,
    pub max_depth: u8,
}

/// Request payload for issuing a relay token.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssueRelayTokenRequest {
    pub target_agent_id: String,
    pub delegated_capabilities: Vec<AgentCapability>,
    /// Optional stricter absolute chain-depth bound. A child cannot increase
    /// the bound inherited from its parent.
    #[serde(default)]
    pub max_depth: Option<u8>,
    #[serde(default = "default_relay_ttl_secs")]
    pub ttl_secs: i64,
}

fn default_relay_ttl_secs() -> i64 {
    DEFAULT_RELAY_TOKEN_TTL_SECS
}

/// Response returned when a relay token is issued.
/// The plaintext token secret is shown **once** and never stored.
#[derive(Debug, Serialize)]
pub struct IssueRelayTokenResponse {
    pub token_id: String,
    pub token_secret: String,
    pub parent_token_id: String,
    pub issuer_agent_id: String,
    pub delegated_capabilities: Vec<AgentCapability>,
    pub target_agent_id: String,
    pub max_depth: u8,
    pub depth: u8,
    pub expires_at: String,
}

// ─── Relay token storage helpers ─────────────────────────────────────────────

fn relay_tokens_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("relay-tokens.json")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyRelayToken {
    token_id: String,
    issuer_agent_id: String,
    target_scope: String,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    used: bool,
    token_hash: String,
}

/// Load the delegation registry. The one recognized pre-T06 format is
/// atomically invalidated because it has no verifiable parent edge. Any other
/// malformed format fails startup closed.
fn load_relay_registry(workdir: &Path) -> anyhow::Result<Vec<RelayToken>> {
    let path = relay_tokens_path(workdir);
    let data = match std::fs::read_to_string(&path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(anyhow::anyhow!("read {}: {error}", path.display())),
    };
    if let Ok(tokens) = serde_json::from_str(&data) {
        return Ok(tokens);
    }
    let legacy: Vec<LegacyRelayToken> = serde_json::from_str(&data)
        .map_err(|error| anyhow::anyhow!("parse {}: {error}", path.display()))?;
    // Force every field to be decoded before invalidation and keep this exact
    // shape check intentional rather than accepting arbitrary malformed JSON.
    for token in &legacy {
        let _ = (
            &token.token_id,
            &token.issuer_agent_id,
            &token.target_scope,
            token.issued_at,
            token.expires_at,
            token.used,
            &token.token_hash,
        );
    }
    tracing::warn!(
        count = legacy.len(),
        path = %path.display(),
        "invalidating legacy unlinked relay tokens during T06 migration"
    );
    atomic_replace_json_sync(&path, &Vec::<RelayToken>::new())?;
    Ok(Vec::new())
}

fn atomic_replace_json_sync<T: Serialize + ?Sized>(path: &Path, value: &T) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("registry path has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("relay-tokens"),
        Uuid::new_v4()
    ));
    let result = (|| -> anyhow::Result<()> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        file.write_all(&serde_json::to_vec_pretty(value)?)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn is_capability_subset(child: &[AgentCapability], parent: &[AgentCapability]) -> bool {
    child.iter().all(|capability| parent.contains(capability))
}

fn canonical_capabilities(capabilities: &[AgentCapability]) -> Vec<AgentCapability> {
    let mut capabilities = capabilities.to_vec();
    capabilities.sort_unstable();
    capabilities.dedup();
    capabilities
}

fn constant_time_hash_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let difference = left
        .as_bytes()
        .iter()
        .zip(right.as_bytes())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        });
    core::hint::black_box(difference) == 0
}

fn cascade_relay_revocation(tokens: &mut [RelayToken], parent_token_id: &str) -> usize {
    let mut revoked_ids = HashSet::from([parent_token_id.to_string()]);
    let mut changed = 0;
    loop {
        let mut advanced = false;
        for token in tokens.iter_mut() {
            if revoked_ids.contains(&token.parent_token_id)
                && revoked_ids.insert(token.token_id.clone())
            {
                if !token.revoked {
                    token.revoked = true;
                    changed += 1;
                }
                advanced = true;
            }
        }
        if !advanced {
            return changed;
        }
    }
}

// ─── Relay token core logic ───────────────────────────────────────────────────

fn root_agent_claims(
    token: &AgentToken,
    now: DateTime<Utc>,
) -> Result<AgentCredentialClaims, ApiError> {
    if token.revoked || token.expires_at <= now {
        return Err(ApiError::unauthorized("invalid delegated credential"));
    }
    Ok(AgentCredentialClaims {
        token_id: token.token_id.clone(),
        agent_id: token.agent_id.clone(),
        capabilities: canonical_capabilities(&token.capabilities),
        expires_at: token.expires_at,
        depth: 0,
        max_depth: MAX_RELAY_MAX_DEPTH,
    })
}

/// Validate every edge back to a live root agent token. All externally visible
/// failures are deliberately generic; the detailed invariant is not leaked.
fn validate_relay_chain(
    leaf: &RelayToken,
    relay_tokens: &[RelayToken],
    agent_tokens: &[AgentToken],
    now: DateTime<Utc>,
) -> Result<AgentCredentialClaims, ApiError> {
    let invalid = || ApiError::unauthorized("invalid delegated credential");
    let leaf_claims = AgentCredentialClaims {
        token_id: leaf.token_id.clone(),
        agent_id: leaf.target_agent_id.clone(),
        capabilities: leaf.delegated_capabilities.clone(),
        expires_at: leaf.expires_at,
        depth: leaf.depth,
        max_depth: leaf.max_depth,
    };
    let mut current = leaf;
    let mut visited = HashSet::new();

    loop {
        if !visited.insert(current.token_id.as_str())
            || current.revoked
            || current.expires_at <= now
            || current.depth == 0
            || current.depth > current.max_depth
            || current.max_depth > MAX_RELAY_MAX_DEPTH
            || current.delegated_capabilities.is_empty()
            || current.delegated_capabilities
                != canonical_capabilities(&current.delegated_capabilities)
        {
            return Err(invalid());
        }
        // Token IDs are graph identities and must be globally unique.
        if relay_tokens
            .iter()
            .filter(|token| token.token_id == current.token_id)
            .count()
            != 1
            || agent_tokens
                .iter()
                .any(|token| token.token_id == current.token_id)
        {
            return Err(invalid());
        }

        if let Some(parent) = relay_tokens
            .iter()
            .find(|token| token.token_id == current.parent_token_id)
        {
            if agent_tokens
                .iter()
                .any(|token| token.token_id == current.parent_token_id)
            {
                return Err(invalid());
            }
            if current.depth != parent.depth.checked_add(1).ok_or_else(invalid)?
                || current.max_depth > parent.max_depth
                || current.expires_at > parent.expires_at
                || current.issuer_agent_id != parent.target_agent_id
                || !is_capability_subset(
                    &current.delegated_capabilities,
                    &parent.delegated_capabilities,
                )
            {
                return Err(invalid());
            }
            current = parent;
            continue;
        }

        let root = agent_tokens
            .iter()
            .find(|token| token.token_id == current.parent_token_id)
            .ok_or_else(invalid)?;
        if agent_tokens
            .iter()
            .filter(|token| token.token_id == current.parent_token_id)
            .count()
            != 1
        {
            return Err(invalid());
        }
        let root_claims = root_agent_claims(root, now)?;
        if current.depth != 1
            || current.expires_at > root_claims.expires_at
            || current.issuer_agent_id != root_claims.agent_id
            || !is_capability_subset(&current.delegated_capabilities, &root_claims.capabilities)
        {
            return Err(invalid());
        }
        return Ok(leaf_claims);
    }
}

impl AuthRegistry {
    pub(crate) async fn authenticate_agent_secret(
        &self,
        plaintext: &str,
    ) -> Option<AgentCredentialClaims> {
        // An unreadable token file cannot vouch for any token: fail closed.
        if let Err(error) = refresh_registry(&self.agent_tokens).await {
            tracing::warn!(
                error = %error,
                "agent-token registry is unreadable; rejecting agent tokens"
            );
            return None;
        }
        let hash = hash_api_key(plaintext);
        let tokens = self.agent_tokens.read().await;
        tokens
            .entries
            .iter()
            .find(|token| constant_time_hash_eq(&token.token_hash, &hash))
            .and_then(|token| root_agent_claims(token, Utc::now()).ok())
    }

    pub(crate) async fn authenticate_relay_secret(
        &self,
        plaintext: &str,
    ) -> Result<AgentCredentialClaims, ApiError> {
        // A delegation is only as valid as every token file on its chain.
        if let Err(error) = self.refresh_token_registries().await {
            tracing::warn!(
                error = %error,
                "token registry is unreadable; rejecting delegated credentials"
            );
            return Err(error);
        }
        let agents = self.agent_tokens.read().await;
        let relays = self.relay_tokens.read().await;
        let hash = hash_api_key(plaintext);
        let leaf = relays
            .entries
            .iter()
            .find(|token| constant_time_hash_eq(&token.token_hash, &hash))
            .ok_or_else(|| ApiError::unauthorized("invalid delegated credential"))?;
        validate_relay_chain(leaf, &relays.entries, &agents.entries, Utc::now())
    }

    /// Re-read both token files if another process changed them, agent tokens
    /// first.
    async fn refresh_token_registries(&self) -> Result<(), ApiError> {
        refresh_registry(&self.agent_tokens).await?;
        refresh_registry(&self.relay_tokens).await
    }

    pub(crate) async fn issue_relay_token(
        &self,
        authenticated_parent: &AgentCredentialClaims,
        request: &IssueRelayTokenRequest,
    ) -> Result<(RelayToken, String), ApiError> {
        if request.target_agent_id.trim().is_empty() {
            return Err(ApiError::bad_request("target_agent_id must not be empty"));
        }
        if request.delegated_capabilities.is_empty() {
            return Err(ApiError::bad_request(
                "at least one delegated capability is required",
            ));
        }
        if request.ttl_secs <= 0 {
            return Err(ApiError::bad_request("ttl_secs must be positive"));
        }

        refresh_registry(&self.agent_tokens).await?;
        // Holding the agent-token cache until the relay write is done keeps a
        // root revocation in this process from interleaving with the issuance.
        let agent_cache = self.agent_tokens.read().await;
        let mut relays = self.relay_tokens.write().await;
        let agents = agent_cache.entries.clone();
        let parent = authenticated_parent.clone();
        let request = request.clone();
        // The chain is checked against the relay file as it is on disk now, so
        // a parent that another process revoked cannot delegate.
        relays
            .update(move |tokens| {
                let (token, secret) = Self::new_relay_token(&parent, &request, tokens, &agents)?;
                tokens.push(token.clone());
                Ok((token, secret))
            })
            .await
    }

    /// Validate a delegation from `authenticated_parent` against the token
    /// lists as they are now, and build the relay token and its secret.
    fn new_relay_token(
        authenticated_parent: &AgentCredentialClaims,
        request: &IssueRelayTokenRequest,
        relays: &[RelayToken],
        agents: &[AgentToken],
    ) -> Result<(RelayToken, String), ApiError> {
        let now = Utc::now();
        let current_parent = if let Some(parent) = relays
            .iter()
            .find(|token| token.token_id == authenticated_parent.token_id)
        {
            validate_relay_chain(parent, relays, agents, now)?
        } else {
            let parent = agents
                .iter()
                .find(|token| token.token_id == authenticated_parent.token_id)
                .ok_or_else(|| ApiError::unauthorized("invalid delegated credential"))?;
            root_agent_claims(parent, now)?
        };
        if current_parent.agent_id != authenticated_parent.agent_id
            || current_parent.capabilities != authenticated_parent.capabilities
        {
            return Err(ApiError::unauthorized("invalid delegated credential"));
        }
        let delegated_capabilities = canonical_capabilities(&request.delegated_capabilities);
        if !is_capability_subset(&delegated_capabilities, &current_parent.capabilities) {
            return Err(ApiError::bad_request(
                "delegated_capabilities must be a subset of the parent capabilities",
            ));
        }

        let depth = current_parent.depth.checked_add(1).ok_or_else(|| {
            ApiError::bad_request("relay delegation depth limit has been reached")
        })?;
        let inherited_max = if current_parent.depth == 0 {
            DEFAULT_RELAY_MAX_DEPTH
        } else {
            current_parent.max_depth
        };
        let max_depth = request.max_depth.unwrap_or(inherited_max);
        if max_depth == 0 || max_depth > MAX_RELAY_MAX_DEPTH {
            return Err(ApiError::bad_request(format!(
                "max_depth must be between 1 and {MAX_RELAY_MAX_DEPTH}",
            )));
        }
        if current_parent.depth > 0 && max_depth > current_parent.max_depth {
            return Err(ApiError::bad_request(
                "a relay token cannot increase its parent's max_depth",
            ));
        }
        if depth > max_depth {
            return Err(ApiError::bad_request(
                "relay delegation depth limit has been reached",
            ));
        }

        let ttl = Duration::try_seconds(request.ttl_secs)
            .ok_or_else(|| ApiError::bad_request("ttl_secs is out of range"))?;
        let requested_expiry = now
            .checked_add_signed(ttl)
            .ok_or_else(|| ApiError::bad_request("ttl_secs is out of range"))?;
        let expires_at = requested_expiry.min(current_parent.expires_at);
        let plaintext = generate_secret("roko_relay_");
        let token = RelayToken {
            token_id: Uuid::new_v4().to_string(),
            parent_token_id: current_parent.token_id,
            issuer_agent_id: current_parent.agent_id,
            delegated_capabilities,
            target_agent_id: request.target_agent_id.trim().to_string(),
            max_depth,
            depth,
            issued_at: now,
            expires_at,
            revoked: false,
            token_hash: hash_api_key(&plaintext),
        };
        Ok((token, plaintext))
    }

    async fn revoke_relay_token(&self, token_id: &str) -> Result<Option<String>, ApiError> {
        let mut relays = self.relay_tokens.write().await;
        let id = token_id.to_string();
        relays
            .update(move |tokens| {
                let Some(token) = tokens.iter_mut().find(|token| token.token_id == id) else {
                    return Ok(None);
                };
                token.revoked = true;
                let target_agent_id = token.target_agent_id.clone();
                cascade_relay_revocation(tokens, &id);
                Ok(Some(target_agent_id))
            })
            .await
    }

    #[cfg(test)]
    async fn relay_tokens_snapshot(&self) -> Vec<RelayToken> {
        refresh_registry(&self.relay_tokens)
            .await
            .expect("relay-token registry is readable");
        self.relay_tokens.read().await.entries.clone()
    }
}

// ─── Router ────────────────────────────────────────────────────────────────

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // API key routes (T01)
        .route("/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api-keys/{name}", delete(revoke_api_key))
        .route("/api-keys/{name}/rotate", post(rotate_api_key))
        // Agent token routes (T02)
        .route(
            "/agent-tokens",
            get(list_agent_tokens).post(issue_agent_token),
        )
        .route("/agent-tokens/{token_id}", delete(revoke_agent_token))
        // Relay token routes (E35-T06)
        .route("/relay-tokens", post(issue_relay_token_handler))
        .route("/relay-tokens/{token_id}", delete(revoke_relay_token))
        // Audit log query (E35-T04)
        .route("/auth/audit", get(query_auth_audit))
}

// ─── Audit helpers (E35-T04) ────────────────────────────────────────────────

/// Query parameters for `GET /api/auth/audit`.
#[derive(Debug, Deserialize)]
pub struct AuditQueryParams {
    /// ISO-8601 lower bound for the event timestamp (inclusive).
    pub from: Option<String>,
    /// ISO-8601 upper bound for the event timestamp (inclusive).
    pub to: Option<String>,
    /// Filter by actor identifier (exact match).
    pub actor: Option<String>,
    /// Filter by action name (case-sensitive serialised form, e.g. `"TokenIssued"`).
    pub action: Option<String>,
}

/// `GET /api/auth/audit` — query auth audit log entries.
async fn query_auth_audit(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AuditQueryParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(log) = state.auth_audit.as_ref() else {
        return Ok(Json(json!({ "events": [], "count": 0 })));
    };

    let from = params
        .from
        .as_deref()
        .and_then(|s| s.parse::<DateTime<Utc>>().ok());
    let to = params
        .to
        .as_deref()
        .and_then(|s| s.parse::<DateTime<Utc>>().ok());
    let action = params.action.as_deref().and_then(|s| {
        serde_json::from_value::<AuthAuditAction>(serde_json::Value::String(s.to_string())).ok()
    });

    let events = log
        .query(from, to, params.actor.as_deref(), action.as_ref())
        .map_err(|e| ApiError::internal(format!("failed to query audit log: {e}")))?;

    let count = events.len();
    Ok(Json(json!({ "events": events, "count": count })))
}

/// Append an [`AuthAuditEvent`] to the audit log in a best-effort, fire-and-forget manner.
///
/// Opens the log file on each call (cheap — the underlying file is small).
/// Errors are logged via `tracing::warn!` and never propagated to callers.
fn append_audit_event(state: &AppState, event: AuthAuditEvent) {
    if let Some(log) = state.auth_audit.as_ref() {
        log.append(&event);
    }
}

fn audit_actor(context: Option<&Extension<AuthContext>>) -> String {
    context
        .and_then(|Extension(context)| context.user_id.clone())
        .unwrap_or_else(|| "local-api".to_string())
}

// ─── API key storage helpers ────────────────────────────────────────────────

fn api_keys_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("api-keys.json")
}

// ─── Agent token storage helpers ────────────────────────────────────────────

fn agent_tokens_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("agent-tokens.json")
}

// ─── API key handlers ───────────────────────────────────────────────────────

/// `POST /api/api-keys` — generate a new API key, store its SHA-256 hash,
/// and return the plaintext key exactly once.
async fn create_api_key(
    State(state): State<Arc<AppState>>,
    context: Option<Extension<AuthContext>>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), ApiError> {
    if req.name.is_empty() {
        return Err(ApiError::bad_request("name must not be empty"));
    }
    crate::error::validate_path_segment(&req.name, "key name")?;
    validate_api_key_scope(&req.scope)?;

    if let Some(expires_at) = req.expires_at.as_deref() {
        if parse_rfc3339(expires_at).is_none() {
            return Err(ApiError::bad_request(
                "expires_at must be an RFC 3339 timestamp",
            ));
        }
    }

    let plaintext = generate_secret("roko_");
    let key_hash = hash_api_key(&plaintext);
    let created_at = Utc::now().to_rfc3339();

    let entry = ApiKeyEntry {
        name: req.name.clone(),
        key_hash,
        scope: req.scope.clone(),
        created_at: created_at.clone(),
        expires_at: req.expires_at.clone(),
        last_used_at: None,
        previous_key_hashes: Vec::new(),
    };

    state.auth_registry.insert_api_key(entry).await?;

    // Audit: TokenIssued
    append_audit_event(
        &state,
        AuthAuditEvent::new(
            audit_actor(context.as_ref()),
            AuthAuditAction::TokenIssued,
            req.name.clone(),
            AuthOutcome::Success,
        ),
    );

    Ok((
        StatusCode::CREATED,
        Json(CreateApiKeyResponse {
            name: req.name,
            key: plaintext,
            scope: req.scope,
            created_at,
            expires_at: req.expires_at,
        }),
    ))
}

/// `GET /api/api-keys` — list all stored API keys (metadata only).
async fn list_api_keys(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let keys = state.auth_registry.current_api_keys().await?;
    let summaries: Vec<ApiKeySummary> = keys
        .into_iter()
        .map(|k| ApiKeySummary {
            name: k.name,
            scope: k.scope,
            created_at: k.created_at,
            expires_at: k.expires_at,
            last_used_at: k.last_used_at,
        })
        .collect();
    Ok(Json(json!({ "keys": summaries })))
}

/// `DELETE /api/api-keys/:name` — revoke an API key by name.
async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    context: Option<Extension<AuthContext>>,
    AxumPath(name): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    if !state.auth_registry.remove_api_key(&name).await? {
        return Err(ApiError::not_found(format!(
            "API key with name '{name}' not found"
        )));
    }

    // Audit: TokenRevoked
    append_audit_event(
        &state,
        AuthAuditEvent::new(
            audit_actor(context.as_ref()),
            AuthAuditAction::TokenRevoked,
            name.clone(),
            AuthOutcome::Success,
        ),
    );

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/api-keys/:name/rotate` — generate a new key for an existing
/// named key entry. The old hash is retained in `previous_key_hashes` for a
/// 5-minute grace period so in-flight requests with the old key don't fail.
///
/// Returns the new plaintext key exactly once. The old key is invalidated
/// (only accepted for 5 minutes via grace period, then discarded).
async fn rotate_api_key(
    State(state): State<Arc<AppState>>,
    context: Option<Extension<AuthContext>>,
    AxumPath(name): AxumPath<String>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), ApiError> {
    let grace_expires = (Utc::now() + Duration::minutes(5)).to_rfc3339();
    let plaintext = generate_secret("roko_");
    let entry = state
        .auth_registry
        .rotate_api_key(
            &name,
            hash_api_key(&plaintext),
            Utc::now().to_rfc3339(),
            grace_expires.clone(),
        )
        .await?;

    let response = CreateApiKeyResponse {
        name: entry.name.clone(),
        key: plaintext,
        scope: entry.scope.clone(),
        created_at: entry.created_at.clone(),
        expires_at: entry.expires_at.clone(),
    };

    // Audit: TokenRotated
    append_audit_event(
        &state,
        AuthAuditEvent::new(
            audit_actor(context.as_ref()),
            AuthAuditAction::TokenRotated,
            response.name.clone(),
            AuthOutcome::Success,
        )
        .with_meta("grace_expires", &grace_expires),
    );

    Ok((StatusCode::OK, Json(response)))
}

// ─── Agent token handlers (T02) ─────────────────────────────────────────────

/// `POST /api/agent-tokens` — issue a scoped bearer token for a specific agent.
///
/// Requires admin-level auth (enforced by the scope middleware on this route).
/// Returns `{ token_id, token_secret }` where the secret is shown only once.
async fn issue_agent_token(
    State(state): State<Arc<AppState>>,
    context: Option<Extension<AuthContext>>,
    Json(req): Json<IssueAgentTokenRequest>,
) -> Result<(StatusCode, Json<IssueAgentTokenResponse>), ApiError> {
    if req.agent_id.is_empty() {
        return Err(ApiError::bad_request("agent_id must not be empty"));
    }
    if req.capabilities.is_empty() {
        return Err(ApiError::bad_request("at least one capability is required"));
    }
    if req.ttl_secs <= 0 {
        return Err(ApiError::bad_request("ttl_secs must be positive"));
    }

    let token_id = Uuid::new_v4().to_string();
    let plaintext = generate_secret("roko_agent_");
    let token_hash = hash_api_key(&plaintext);
    let now = Utc::now();
    let expires_at = now + Duration::seconds(req.ttl_secs);

    let token = AgentToken {
        token_id: token_id.clone(),
        agent_id: req.agent_id.clone(),
        capabilities: req.capabilities.clone(),
        issued_at: now.to_rfc3339(),
        expires_at,
        revoked: false,
        token_hash,
    };

    state.auth_registry.insert_agent_token(token).await?;

    // Audit: TokenIssued (agent token)
    append_audit_event(
        &state,
        AuthAuditEvent::new(
            audit_actor(context.as_ref()),
            AuthAuditAction::TokenIssued,
            token_id.clone(),
            AuthOutcome::Success,
        )
        .with_meta("expires_at", expires_at.to_rfc3339()),
    );

    Ok((
        StatusCode::CREATED,
        Json(IssueAgentTokenResponse {
            token_id,
            token_secret: plaintext,
            agent_id: req.agent_id,
            capabilities: req.capabilities,
            expires_at: expires_at.to_rfc3339(),
        }),
    ))
}

/// `GET /api/agent-tokens` — list active (non-revoked, non-expired) tokens.
/// Returns metadata only; token secrets are never returned after issuance.
async fn list_agent_tokens(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let tokens = state.auth_registry.current_agent_tokens().await?;
    let now = Utc::now();
    let summaries: Vec<AgentTokenSummary> = tokens
        .into_iter()
        .filter(|t| !t.revoked && t.expires_at > now)
        .map(|t| AgentTokenSummary {
            token_id: t.token_id,
            agent_id: t.agent_id,
            capabilities: t.capabilities,
            issued_at: t.issued_at,
            expires_at: t.expires_at.to_rfc3339(),
            revoked: t.revoked,
        })
        .collect();
    Ok(Json(json!({ "tokens": summaries })))
}

/// `DELETE /api/agent-tokens/:token_id` — revoke a token by setting
/// `revoked = true`. The token record is kept for audit purposes.
async fn revoke_agent_token(
    State(state): State<Arc<AppState>>,
    context: Option<Extension<AuthContext>>,
    AxumPath(token_id): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let agent_id = state
        .auth_registry
        .revoke_agent_token(&token_id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("agent token '{token_id}' not found")))?;

    // Audit: TokenRevoked
    append_audit_event(
        &state,
        AuthAuditEvent::new(
            audit_actor(context.as_ref()),
            AuthAuditAction::TokenRevoked,
            token_id.clone(),
            AuthOutcome::Success,
        )
        .with_meta("agent_id", agent_id),
    );

    Ok(StatusCode::NO_CONTENT)
}

// Relay token delegation handlers (E35-T06)

/// `POST /api/relay-tokens` — issue a parent-linked capability delegation.
///
/// The parent is always derived from the authenticated agent/relay extension;
/// callers cannot nominate a more privileged parent in the request body.
async fn issue_relay_token_handler(
    State(state): State<Arc<AppState>>,
    context: Option<Extension<AuthContext>>,
    credential: Option<Extension<AgentCredentialClaims>>,
    headers: HeaderMap,
    Json(req): Json<IssueRelayTokenRequest>,
) -> Result<(StatusCode, Json<IssueRelayTokenResponse>), ApiError> {
    let credential = if let Some(Extension(credential)) = credential {
        credential
    } else {
        // Agent delegation remains an always-authenticated protocol even when
        // global human/API auth is disabled. Validate the bearer locally in
        // that configuration; anonymous callers still fail closed.
        let plaintext = headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(super::middleware::extract_bearer_token)
            .ok_or_else(|| ApiError::unauthorized("a valid agent credential is required"))?;
        if plaintext.starts_with("roko_agent_") {
            state
                .auth_registry
                .authenticate_agent_secret(plaintext)
                .await
                .ok_or_else(|| ApiError::unauthorized("a valid agent credential is required"))?
        } else if plaintext.starts_with("roko_relay_") {
            state
                .auth_registry
                .authenticate_relay_secret(plaintext)
                .await
                .map_err(|_| ApiError::unauthorized("a valid agent credential is required"))?
        } else {
            return Err(ApiError::unauthorized(
                "a valid agent credential is required",
            ));
        }
    };
    let (token, plaintext) = state
        .auth_registry
        .issue_relay_token(&credential, &req)
        .await?;
    let actor = context.as_ref().map_or_else(
        || format!("agent:{}", credential.agent_id),
        |context| audit_actor(Some(context)),
    );
    let response = IssueRelayTokenResponse {
        token_id: token.token_id.clone(),
        token_secret: plaintext,
        parent_token_id: token.parent_token_id.clone(),
        issuer_agent_id: token.issuer_agent_id.clone(),
        delegated_capabilities: token.delegated_capabilities.clone(),
        target_agent_id: token.target_agent_id.clone(),
        max_depth: token.max_depth,
        depth: token.depth,
        expires_at: token.expires_at.to_rfc3339(),
    };
    append_audit_event(
        &state,
        AuthAuditEvent::new(
            actor,
            AuthAuditAction::TokenIssued,
            token.token_id,
            AuthOutcome::Success,
        )
        .with_meta("token_kind", "relay")
        .with_meta("issuer_agent_id", token.issuer_agent_id)
        .with_meta("parent_token_id", token.parent_token_id)
        .with_meta("target_agent_id", token.target_agent_id)
        .with_meta("depth", token.depth.to_string())
        .with_meta("max_depth", token.max_depth.to_string())
        .with_meta(
            "delegated_capabilities",
            format!("{:?}", token.delegated_capabilities),
        )
        .with_meta("expires_at", token.expires_at.to_rfc3339()),
    );
    Ok((StatusCode::CREATED, Json(response)))
}

async fn revoke_relay_token(
    State(state): State<Arc<AppState>>,
    context: Option<Extension<AuthContext>>,
    AxumPath(token_id): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let target_agent_id = state
        .auth_registry
        .revoke_relay_token(&token_id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("relay token '{token_id}' not found")))?;
    append_audit_event(
        &state,
        AuthAuditEvent::new(
            audit_actor(context.as_ref()),
            AuthAuditAction::TokenRevoked,
            token_id,
            AuthOutcome::Success,
        )
        .with_meta("token_kind", "relay")
        .with_meta("target_agent_id", target_agent_id),
    );
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_api_key_is_deterministic() {
        let hash1 = hash_api_key("test-key-123");
        let hash2 = hash_api_key("test-key-123");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn hash_api_key_differs_for_different_inputs() {
        let hash1 = hash_api_key("key-a");
        let hash2 = hash_api_key("key-b");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn agent_token_secret_is_32_bytes_base64url() {
        let plaintext = generate_secret("roko_agent_");
        assert!(plaintext.starts_with("roko_agent_"));
        let encoded = plaintext.trim_start_matches("roko_agent_");
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .expect("agent secret is base64url");
        assert_eq!(decoded.len(), 32);
        assert!(!encoded.contains('='));
    }

    fn test_api_key(name: &str, plaintext: &str) -> ApiKeyEntry {
        ApiKeyEntry {
            name: name.to_string(),
            key_hash: hash_api_key(plaintext),
            scope: "admin".to_string(),
            created_at: Utc::now().to_rfc3339(),
            expires_at: None,
            last_used_at: None,
            previous_key_hashes: Vec::new(),
        }
    }

    #[tokio::test]
    async fn registry_survives_restart_and_tracks_usage() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = AuthRegistry::load(dir.path(), &[]).expect("empty registry");
        registry
            .insert_api_key(test_api_key("restart-key", "secret-one"))
            .await
            .expect("insert key");
        registry
            .record_api_key_use("restart-key")
            .await
            .expect("record usage");
        drop(registry);

        let restarted = AuthRegistry::load(dir.path(), &[]).expect("reload registry");
        let keys = restarted.api_keys_snapshot().await;
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "restart-key");
        assert!(keys[0].last_used_at.is_some());
        assert!(matches!(
            match_api_key_entry_for_test("secret-one", &keys),
            Some(true)
        ));
    }

    // Keep the registry test independent from middleware's private match enum.
    fn match_api_key_entry_for_test(plaintext: &str, keys: &[ApiKeyEntry]) -> Option<bool> {
        let hash = hash_api_key(plaintext);
        keys.iter()
            .find(|entry| entry.key_hash == hash)
            .map(|entry| entry.expires_at.is_none())
    }

    #[tokio::test]
    async fn usage_update_cannot_clobber_rotation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = Arc::new(AuthRegistry::load(dir.path(), &[]).expect("empty registry"));
        registry
            .insert_api_key(test_api_key("race-key", "old-secret"))
            .await
            .expect("insert key");

        let usage_registry = Arc::clone(&registry);
        let rotate_registry = Arc::clone(&registry);
        let (usage, rotation) = tokio::join!(
            async move { usage_registry.record_api_key_use("race-key").await },
            async move {
                rotate_registry
                    .rotate_api_key(
                        "race-key",
                        hash_api_key("new-secret"),
                        Utc::now().to_rfc3339(),
                        (Utc::now() + Duration::minutes(5)).to_rfc3339(),
                    )
                    .await
            }
        );
        usage.expect("usage write");
        rotation.expect("rotation write");

        let restarted = AuthRegistry::load(dir.path(), &[]).expect("reload registry");
        let keys = restarted.api_keys_snapshot().await;
        assert_eq!(keys[0].key_hash, hash_api_key("new-secret"));
        assert_eq!(keys[0].previous_key_hashes.len(), 1);
        assert_eq!(keys[0].previous_key_hashes[0].0, hash_api_key("old-secret"));
    }

    #[tokio::test]
    async fn rotation_retains_only_two_previous_hashes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = AuthRegistry::load(dir.path(), &[]).expect("empty registry");
        registry
            .insert_api_key(test_api_key("rotation-key", "secret-0"))
            .await
            .expect("insert key");
        for index in 1..=3 {
            registry
                .rotate_api_key(
                    "rotation-key",
                    hash_api_key(&format!("secret-{index}")),
                    Utc::now().to_rfc3339(),
                    (Utc::now() + Duration::minutes(5)).to_rfc3339(),
                )
                .await
                .expect("rotate key");
        }
        let keys = registry.api_keys_snapshot().await;
        assert_eq!(keys[0].previous_key_hashes.len(), 2);
        assert_eq!(keys[0].previous_key_hashes[0].0, hash_api_key("secret-1"));
        assert_eq!(keys[0].previous_key_hashes[1].0, hash_api_key("secret-2"));
    }

    #[test]
    fn malformed_registry_fails_closed_on_startup() {
        let dir = tempfile::tempdir().expect("tempdir");
        let roko_dir = dir.path().join(".roko");
        std::fs::create_dir_all(&roko_dir).expect("create roko dir");
        std::fs::write(roko_dir.join("api-keys.json"), "not-json").expect("write fixture");
        let error = AuthRegistry::load(dir.path(), &[])
            .err()
            .expect("malformed registry must fail");
        assert!(error.to_string().contains("parse"));
    }

    // Out-of-band key file changes (bug-da5b41)

    /// Change the key file as another process would: under the shared lock,
    /// behind the back of any loaded registry.
    fn edit_key_file(workdir: &Path, edit: impl FnOnce(&mut Vec<ApiKeyEntry>)) {
        with_locked_json_transaction::<Vec<ApiKeyEntry>, _, std::io::Error, _>(
            &api_keys_path(workdir),
            |keys| {
                edit(keys);
                Ok(())
            },
        )
        .expect("edit key file out of band");
    }

    fn key_names_on_disk(workdir: &Path) -> Vec<String> {
        load_registry_file::<ApiKeyEntry>(&api_keys_path(workdir))
            .expect("read key file")
            .into_iter()
            .map(|entry| entry.name)
            .collect()
    }

    fn accepts(keys: &[ApiKeyEntry], secret: &str) -> bool {
        match_api_key_entry_for_test(secret, keys) == Some(true)
    }

    #[tokio::test]
    async fn out_of_band_api_key_survives_server_write() {
        let dir = tmp_workdir();
        let registry = AuthRegistry::load(dir.path(), &[]).expect("empty registry");
        registry
            .insert_api_key(test_api_key("server", "server-secret"))
            .await
            .expect("insert server key");
        // A CLI or a second server adds a key behind the loaded registry.
        edit_key_file(dir.path(), |keys| {
            keys.push(test_api_key("cli", "cli-secret"));
        });

        registry
            .record_api_key_use("server")
            .await
            .expect("record usage");
        assert_eq!(key_names_on_disk(dir.path()), ["server", "cli"]);
        registry
            .insert_api_key(test_api_key("temp", "temp-secret"))
            .await
            .expect("insert temporary key");
        assert_eq!(key_names_on_disk(dir.path()), ["server", "cli", "temp"]);
        registry
            .rotate_api_key(
                "server",
                hash_api_key("rotated-secret"),
                Utc::now().to_rfc3339(),
                (Utc::now() + Duration::minutes(5)).to_rfc3339(),
            )
            .await
            .expect("rotate server key");
        assert_eq!(key_names_on_disk(dir.path()), ["server", "cli", "temp"]);
        assert!(
            registry
                .remove_api_key("temp")
                .await
                .expect("remove the temporary key")
        );
        assert_eq!(key_names_on_disk(dir.path()), ["server", "cli"]);

        // The running server accepts the new key without a restart.
        let keys = registry.api_keys_snapshot().await;
        assert!(accepts(&keys, "cli-secret"));
        assert!(accepts(&keys, "rotated-secret"));
    }

    #[tokio::test]
    async fn out_of_band_revocation_is_not_undone_by_server_write() {
        let dir = tmp_workdir();
        let registry = AuthRegistry::load(dir.path(), &[]).expect("empty registry");
        for (name, secret) in [("kept", "kept-secret"), ("gone", "gone-secret")] {
            registry
                .insert_api_key(test_api_key(name, secret))
                .await
                .expect("insert key");
        }
        // Another process revokes a key that this registry still has cached.
        edit_key_file(dir.path(), |keys| {
            keys.retain(|key| key.name != "gone");
        });

        registry
            .record_api_key_use("kept")
            .await
            .expect("record usage");
        assert_eq!(key_names_on_disk(dir.path()), ["kept"]);
        let keys = registry.api_keys_snapshot().await;
        assert!(accepts(&keys, "kept-secret"));
        assert!(!accepts(&keys, "gone-secret"));
    }

    #[tokio::test]
    async fn configured_key_is_written_once_and_stays_revoked() {
        let dir = tmp_workdir();
        let configured = [test_api_key("configured", "cfg-secret")];
        let registry = AuthRegistry::load(dir.path(), &configured).expect("load registry");
        // The first write adds the configured key to the file, as before.
        registry
            .insert_api_key(test_api_key("server", "server-secret"))
            .await
            .expect("insert server key");
        assert_eq!(key_names_on_disk(dir.path()), ["configured", "server"]);

        // Once another process revokes it, neither the configuration nor the
        // registry's cached copy may bring it back.
        edit_key_file(dir.path(), |keys| {
            keys.retain(|key| key.name != "configured");
        });
        registry
            .record_api_key_use("server")
            .await
            .expect("record usage");
        assert_eq!(key_names_on_disk(dir.path()), ["server"]);
        let keys = registry.api_keys_snapshot().await;
        assert!(!accepts(&keys, "cfg-secret"));
    }

    #[tokio::test]
    async fn unreadable_key_file_fails_closed_while_running() {
        let dir = tmp_workdir();
        let registry = AuthRegistry::load(dir.path(), &[]).expect("empty registry");
        registry
            .insert_api_key(test_api_key("server", "server-secret"))
            .await
            .expect("insert server key");
        let path = api_keys_path(dir.path());
        std::fs::write(&path, "not-json").expect("corrupt key file");

        // No named key authenticates, and no write replaces the file from the
        // cached copy.
        assert!(registry.api_keys_snapshot().await.is_empty());
        let error = registry
            .insert_api_key(test_api_key("new", "new-secret"))
            .await
            .expect_err("merging into an unreadable file must fail");
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
        let on_disk = std::fs::read_to_string(&path).expect("read key file");
        assert_eq!(on_disk, "not-json");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_writers_on_one_workspace_keep_every_key() {
        let dir = tmp_workdir();
        // Two registries on one workspace stand in for two server processes.
        let first = Arc::new(AuthRegistry::load(dir.path(), &[]).expect("load first"));
        let second = Arc::new(AuthRegistry::load(dir.path(), &[]).expect("load second"));
        let mut writers = Vec::new();
        for index in 0..8 {
            for (label, registry) in [("first", &first), ("second", &second)] {
                let registry = Arc::clone(registry);
                let name = format!("{label}-{index}");
                writers.push(tokio::spawn(async move {
                    let entry = test_api_key(&name, &name);
                    registry.insert_api_key(entry).await
                }));
            }
        }
        for writer in writers {
            writer.await.expect("writer task").expect("insert key");
        }

        assert_eq!(key_names_on_disk(dir.path()).len(), 16);
        assert_eq!(first.api_keys_snapshot().await.len(), 16);
        assert_eq!(second.api_keys_snapshot().await.len(), 16);
    }

    #[tokio::test]
    async fn repeated_use_is_persisted_at_most_once_per_interval() {
        let dir = tmp_workdir();
        let registry = AuthRegistry::load(dir.path(), &[]).expect("empty registry");
        registry
            .insert_api_key(test_api_key("busy", "busy-secret"))
            .await
            .expect("insert key");
        let mut recorded = Vec::new();
        for _ in 0..2 {
            registry
                .record_api_key_use("busy")
                .await
                .expect("record the key's use");
            let keys = registry.api_keys_snapshot().await;
            recorded.push(keys[0].last_used_at.clone());
        }
        assert!(recorded[0].is_some());
        assert_eq!(recorded[0], recorded[1]);
    }

    // Relay token delegation tests (E35-T06)

    fn tmp_workdir() -> tempfile::TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    fn root_token(secret: &str, capabilities: Vec<AgentCapability>) -> AgentToken {
        AgentToken {
            token_id: "root-token".to_string(),
            agent_id: "root-agent".to_string(),
            capabilities,
            issued_at: Utc::now().to_rfc3339(),
            expires_at: Utc::now() + Duration::hours(1),
            revoked: false,
            token_hash: hash_api_key(secret),
        }
    }

    fn relay_request(
        target: &str,
        capabilities: Vec<AgentCapability>,
        max_depth: Option<u8>,
    ) -> IssueRelayTokenRequest {
        IssueRelayTokenRequest {
            target_agent_id: target.to_string(),
            delegated_capabilities: capabilities,
            max_depth,
            ttl_secs: DEFAULT_RELAY_TOKEN_TTL_SECS,
        }
    }

    async fn registry_with_root(
        dir: &tempfile::TempDir,
        capabilities: Vec<AgentCapability>,
    ) -> (AuthRegistry, AgentCredentialClaims) {
        let registry = AuthRegistry::load(dir.path(), &[]).expect("load registry");
        let secret = "roko_agent_root-secret";
        registry
            .insert_agent_token(root_token(secret, capabilities))
            .await
            .expect("insert root token");
        let claims = registry
            .authenticate_agent_secret(secret)
            .await
            .expect("authenticate root");
        (registry, claims)
    }

    #[tokio::test]
    async fn relay_issuance_links_parent_narrows_and_hashes_secret() {
        let dir = tmp_workdir();
        let (registry, root) = registry_with_root(
            &dir,
            vec![AgentCapability::StoreRead, AgentCapability::Inference],
        )
        .await;
        let (token, plaintext) = registry
            .issue_relay_token(
                &root,
                &relay_request(
                    "child-agent",
                    vec![AgentCapability::Inference, AgentCapability::Inference],
                    Some(4),
                ),
            )
            .await
            .expect("issue relay");

        assert!(plaintext.starts_with("roko_relay_"));
        assert_eq!(plaintext.len(), "roko_relay_".len() + 43);
        assert_eq!(token.parent_token_id, root.token_id);
        assert_eq!(token.issuer_agent_id, "root-agent");
        assert_eq!(token.target_agent_id, "child-agent");
        assert_eq!(
            token.delegated_capabilities,
            vec![AgentCapability::Inference]
        );
        assert_eq!((token.depth, token.max_depth), (1, 4));
        assert!(!token.revoked);
        assert_ne!(token.token_hash, plaintext);

        let persisted =
            std::fs::read_to_string(relay_tokens_path(dir.path())).expect("read relay registry");
        assert!(!persisted.contains(&plaintext));
        let reloaded = AuthRegistry::load(dir.path(), &[]).expect("reload registry");
        assert_eq!(reloaded.relay_tokens_snapshot().await.len(), 1);
        assert!(reloaded.authenticate_relay_secret(&plaintext).await.is_ok());
    }

    #[tokio::test]
    async fn relay_rejects_capability_widening_with_bad_request() {
        let dir = tmp_workdir();
        let (registry, root) = registry_with_root(&dir, vec![AgentCapability::Inference]).await;
        let error = registry
            .issue_relay_token(
                &root,
                &relay_request("child", vec![AgentCapability::Tools], None),
            )
            .await
            .expect_err("widening must fail");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert!(registry.relay_tokens_snapshot().await.is_empty());
    }

    #[tokio::test]
    async fn relay_depth_bound_is_inherited_and_enforced() {
        let dir = tmp_workdir();
        let (registry, root) = registry_with_root(&dir, vec![AgentCapability::Inference]).await;
        let (first, first_secret) = registry
            .issue_relay_token(
                &root,
                &relay_request("agent-b", vec![AgentCapability::Inference], Some(2)),
            )
            .await
            .expect("first delegation");
        let first_claims = registry
            .authenticate_relay_secret(&first_secret)
            .await
            .expect("first relay auth");
        let (second, second_secret) = registry
            .issue_relay_token(
                &first_claims,
                &relay_request("agent-c", vec![AgentCapability::Inference], None),
            )
            .await
            .expect("second delegation");
        assert_eq!(first.depth, 1);
        assert_eq!((second.depth, second.max_depth), (2, 2));
        let second_claims = registry
            .authenticate_relay_secret(&second_secret)
            .await
            .expect("second relay auth");
        let error = registry
            .issue_relay_token(
                &second_claims,
                &relay_request("agent-d", vec![AgentCapability::Inference], None),
            )
            .await
            .expect_err("third delegation exceeds bound");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn root_revocation_cascades_and_full_chain_validation_rejects_descendants() {
        let dir = tmp_workdir();
        let (registry, root) = registry_with_root(&dir, vec![AgentCapability::Inference]).await;
        let (first, first_secret) = registry
            .issue_relay_token(
                &root,
                &relay_request("agent-b", vec![AgentCapability::Inference], Some(3)),
            )
            .await
            .expect("first delegation");
        let first_claims = registry
            .authenticate_relay_secret(&first_secret)
            .await
            .expect("first relay auth");
        let (_second, second_secret) = registry
            .issue_relay_token(
                &first_claims,
                &relay_request("agent-c", vec![AgentCapability::Inference], None),
            )
            .await
            .expect("second delegation");
        registry
            .revoke_agent_token(&root.token_id)
            .await
            .expect("revoke root")
            .expect("root exists");
        assert!(
            registry
                .authenticate_relay_secret(&first_secret)
                .await
                .is_err()
        );
        assert!(
            registry
                .authenticate_relay_secret(&second_secret)
                .await
                .is_err()
        );
        let relays = registry.relay_tokens_snapshot().await;
        assert_eq!(relays.len(), 2);
        assert!(relays.iter().all(|token| token.revoked));
        assert_eq!(relays[0].token_id, first.token_id);
    }

    #[tokio::test]
    async fn relay_revocation_cascades_to_its_descendants() {
        let dir = tmp_workdir();
        let (registry, root) = registry_with_root(&dir, vec![AgentCapability::Inference]).await;
        let (first, first_secret) = registry
            .issue_relay_token(
                &root,
                &relay_request("agent-b", vec![AgentCapability::Inference], Some(3)),
            )
            .await
            .expect("first delegation");
        let first_claims = registry
            .authenticate_relay_secret(&first_secret)
            .await
            .expect("first relay auth");
        let (_second, second_secret) = registry
            .issue_relay_token(
                &first_claims,
                &relay_request("agent-c", vec![AgentCapability::Inference], None),
            )
            .await
            .expect("second delegation");
        let (_sibling, sibling_secret) = registry
            .issue_relay_token(
                &root,
                &relay_request("agent-sibling", vec![AgentCapability::Inference], Some(3)),
            )
            .await
            .expect("sibling delegation");

        registry
            .revoke_relay_token(&first.token_id)
            .await
            .expect("revoke relay")
            .expect("relay exists");
        assert!(
            registry
                .authenticate_relay_secret(&first_secret)
                .await
                .is_err()
        );
        assert!(
            registry
                .authenticate_relay_secret(&second_secret)
                .await
                .is_err()
        );
        assert!(
            registry
                .authenticate_relay_secret(&sibling_secret)
                .await
                .is_ok()
        );
        let relays = registry.relay_tokens_snapshot().await;
        assert_eq!(relays.iter().filter(|token| token.revoked).count(), 2);
        assert_eq!(relays.iter().filter(|token| !token.revoked).count(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_issuance_is_lossless_and_racing_revocation_fails_closed() {
        let dir = tmp_workdir();
        let (registry, root) = registry_with_root(&dir, vec![AgentCapability::Inference]).await;
        let registry = Arc::new(registry);

        let mut issuers = Vec::new();
        for index in 0..16 {
            let registry = Arc::clone(&registry);
            let root = root.clone();
            issuers.push(tokio::spawn(async move {
                registry
                    .issue_relay_token(
                        &root,
                        &relay_request(
                            &format!("child-{index}"),
                            vec![AgentCapability::Inference],
                            Some(2),
                        ),
                    )
                    .await
            }));
        }
        let mut secrets = Vec::new();
        for issuer in issuers {
            let (_token, secret) = issuer
                .await
                .expect("issuer task")
                .expect("concurrent issuance");
            secrets.push(secret);
        }
        assert_eq!(registry.relay_tokens_snapshot().await.len(), 16);

        let revoker = {
            let registry = Arc::clone(&registry);
            let root_id = root.token_id.clone();
            tokio::spawn(async move { registry.revoke_agent_token(&root_id).await })
        };
        let racing_issuer = {
            let registry = Arc::clone(&registry);
            let root = root.clone();
            tokio::spawn(async move {
                registry
                    .issue_relay_token(
                        &root,
                        &relay_request("racing-child", vec![AgentCapability::Inference], Some(2)),
                    )
                    .await
            })
        };
        revoker
            .await
            .expect("revoker task")
            .expect("revoke root")
            .expect("root exists");
        if let Ok((_token, secret)) = racing_issuer.await.expect("racing issuer task") {
            secrets.push(secret);
        }
        for secret in secrets {
            assert!(registry.authenticate_relay_secret(&secret).await.is_err());
        }
        assert!(
            registry
                .relay_tokens_snapshot()
                .await
                .iter()
                .all(|token| token.revoked)
        );
    }

    #[tokio::test]
    async fn relay_chain_rejects_orphan_cycle_depth_capability_and_id_tampering() {
        let dir = tmp_workdir();
        let (registry, root) = registry_with_root(
            &dir,
            vec![AgentCapability::Inference, AgentCapability::Tools],
        )
        .await;
        let (first, _) = registry
            .issue_relay_token(
                &root,
                &relay_request("agent-b", vec![AgentCapability::Inference], Some(3)),
            )
            .await
            .expect("first delegation");
        let agents = registry.agent_tokens_snapshot().await;
        let original = registry.relay_tokens_snapshot().await;

        let mut orphan = original.clone();
        orphan[0].parent_token_id = "missing".to_string();
        assert!(validate_relay_chain(&orphan[0], &orphan, &agents, Utc::now()).is_err());

        let mut cycle = original.clone();
        cycle[0].parent_token_id = first.token_id.clone();
        assert!(validate_relay_chain(&cycle[0], &cycle, &agents, Utc::now()).is_err());

        let mut bad_depth = original.clone();
        bad_depth[0].depth = 0;
        assert!(validate_relay_chain(&bad_depth[0], &bad_depth, &agents, Utc::now()).is_err());

        let mut widened = original.clone();
        widened[0].delegated_capabilities = vec![AgentCapability::StoreWrite];
        assert!(validate_relay_chain(&widened[0], &widened, &agents, Utc::now()).is_err());

        let mut collision_agents = agents.clone();
        collision_agents[0].token_id = first.token_id;
        assert!(
            validate_relay_chain(&original[0], &original, &collision_agents, Utc::now()).is_err()
        );
    }

    #[tokio::test]
    async fn child_expiry_is_capped_by_parent_and_huge_ttl_is_rejected() {
        let dir = tmp_workdir();
        let (registry, root) = registry_with_root(&dir, vec![AgentCapability::Inference]).await;
        let (first, _) = registry
            .issue_relay_token(
                &root,
                &IssueRelayTokenRequest {
                    ttl_secs: 7200,
                    ..relay_request("child", vec![AgentCapability::Inference], None)
                },
            )
            .await
            .expect("issue capped relay");
        assert!(first.expires_at <= root.expires_at);

        let error = registry
            .issue_relay_token(
                &root,
                &IssueRelayTokenRequest {
                    ttl_secs: i64::MAX,
                    ..relay_request("child", vec![AgentCapability::Inference], None)
                },
            )
            .await
            .expect_err("unrepresentable ttl must fail");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn legacy_relay_registry_is_invalidated_but_other_malformed_json_fails() {
        let dir = tmp_workdir();
        let path = relay_tokens_path(dir.path());
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create registry dir");
        std::fs::write(
            &path,
            serde_json::to_vec(&vec![json!({
                "token_id": "legacy",
                "issuer_agent_id": "agent-a",
                "target_scope": "inference",
                "issued_at": Utc::now(),
                "expires_at": Utc::now() + Duration::minutes(5),
                "used": false,
                "token_hash": hash_api_key("legacy-secret"),
            })])
            .expect("serialize legacy"),
        )
        .expect("write legacy registry");
        let registry = AuthRegistry::load(dir.path(), &[]).expect("legacy migration");
        assert!(registry.relay_tokens.blocking_read().entries.is_empty());
        assert_eq!(
            std::fs::read_to_string(&path)
                .expect("read migrated")
                .trim(),
            "[]"
        );

        std::fs::write(&path, r#"[{"token_id":"unknown-shape"}]"#)
            .expect("write malformed registry");
        assert!(AuthRegistry::load(dir.path(), &[]).is_err());
    }

    // Token files changed by another process (bug-39d54c)

    async fn agent_accepted(store: &AuthRegistry, secret: &str) -> bool {
        store.authenticate_agent_secret(secret).await.is_some()
    }

    async fn relay_accepted(store: &AuthRegistry, secret: &str) -> bool {
        store.authenticate_relay_secret(secret).await.is_ok()
    }

    fn agent_tokens_on_disk(workdir: &Path) -> Vec<AgentToken> {
        load_registry_file(&agent_tokens_path(workdir)).expect("read agent tokens")
    }

    fn relay_tokens_on_disk(workdir: &Path) -> Vec<RelayToken> {
        load_registry_file(&relay_tokens_path(workdir)).expect("read relay tokens")
    }

    #[tokio::test]
    async fn a_token_revoked_by_another_process_stays_revoked() {
        let dir = tmp_workdir();
        let (first, root) = registry_with_root(&dir, vec![AgentCapability::Inference]).await;
        let (_relay, relay_secret) = first
            .issue_relay_token(
                &root,
                &relay_request("child", vec![AgentCapability::Inference], None),
            )
            .await
            .expect("issue relay");
        // A second registry on the workspace stands in for a second server
        // process. It caches both tokens while they are still valid.
        let second = AuthRegistry::load(dir.path(), &[]).expect("load second registry");
        let root_secret = "roko_agent_root-secret";
        assert!(agent_accepted(&second, root_secret).await);
        assert!(relay_accepted(&second, &relay_secret).await);

        first
            .revoke_agent_token(&root.token_id)
            .await
            .expect("revoke root")
            .expect("root exists");
        // The second registry then issues a token of its own.
        let other_secret = "roko_agent_other-secret";
        second
            .insert_agent_token(AgentToken {
                token_id: "other-token".to_string(),
                ..root_token(other_secret, vec![AgentCapability::Inference])
            })
            .await
            .expect("insert another agent token");
        let error = second
            .issue_relay_token(
                &root,
                &relay_request("late-child", vec![AgentCapability::Inference], None),
            )
            .await
            .expect_err("a revoked root cannot delegate");
        assert_eq!(error.status, StatusCode::UNAUTHORIZED);

        // The revocation survived, and neither registry accepts the root or its
        // delegation any more.
        let agents = agent_tokens_on_disk(dir.path());
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].token_id, root.token_id);
        assert!(agents[0].revoked);
        let relays = relay_tokens_on_disk(dir.path());
        assert_eq!(relays.len(), 1);
        assert!(relays[0].revoked);
        for store in [&first, &second] {
            assert!(!agent_accepted(store, root_secret).await);
            assert!(!relay_accepted(store, &relay_secret).await);
        }
        assert!(agent_accepted(&second, other_secret).await);
    }

    #[tokio::test]
    async fn a_relay_revoked_by_another_process_stays_revoked() {
        let dir = tmp_workdir();
        let (first, root) = registry_with_root(&dir, vec![AgentCapability::Inference]).await;
        let (revoked, revoked_secret) = first
            .issue_relay_token(
                &root,
                &relay_request("child", vec![AgentCapability::Inference], None),
            )
            .await
            .expect("issue relay");
        let second = AuthRegistry::load(dir.path(), &[]).expect("load second registry");
        assert!(relay_accepted(&second, &revoked_secret).await);

        first
            .revoke_relay_token(&revoked.token_id)
            .await
            .expect("revoke relay")
            .expect("relay exists");
        // The second registry, whose cache still has the relay live, issues a
        // sibling.
        let (_sibling, sibling_secret) = second
            .issue_relay_token(
                &root,
                &relay_request("sibling", vec![AgentCapability::Inference], None),
            )
            .await
            .expect("issue sibling relay");

        let relays = relay_tokens_on_disk(dir.path());
        assert_eq!(relays.len(), 2);
        assert!(relays[0].revoked);
        assert!(!relays[1].revoked);
        for store in [&first, &second] {
            assert!(!relay_accepted(store, &revoked_secret).await);
            assert!(relay_accepted(store, &sibling_secret).await);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_token_writers_on_one_workspace_keep_every_token() {
        let dir = tmp_workdir();
        let (first, root) = registry_with_root(&dir, vec![AgentCapability::Inference]).await;
        let first = Arc::new(first);
        let second = Arc::new(AuthRegistry::load(dir.path(), &[]).expect("load second"));
        let mut writers = Vec::new();
        for index in 0..8 {
            for store in [&first, &second] {
                let store = Arc::clone(store);
                let root = root.clone();
                let target = format!("child-{index}");
                writers.push(tokio::spawn(async move {
                    let request = relay_request(&target, vec![AgentCapability::Inference], None);
                    store.issue_relay_token(&root, &request).await
                }));
            }
        }
        for writer in writers {
            writer.await.expect("writer task").expect("issue relay");
        }

        assert_eq!(relay_tokens_on_disk(dir.path()).len(), 16);
        assert_eq!(first.relay_tokens_snapshot().await.len(), 16);
        assert_eq!(second.relay_tokens_snapshot().await.len(), 16);
    }

    #[tokio::test]
    async fn unreadable_token_files_fail_closed_while_running() {
        let dir = tmp_workdir();
        let (registry, root) = registry_with_root(&dir, vec![AgentCapability::Inference]).await;
        let (_relay, relay_secret) = registry
            .issue_relay_token(
                &root,
                &relay_request("child", vec![AgentCapability::Inference], None),
            )
            .await
            .expect("issue relay");
        let relays_path = relay_tokens_path(dir.path());
        std::fs::write(&relays_path, "not-json").expect("corrupt relay tokens");

        // No delegation authenticates, and no write replaces the file from the
        // cached copy.
        assert!(!relay_accepted(&registry, &relay_secret).await);
        let error = registry
            .issue_relay_token(
                &root,
                &relay_request("late-child", vec![AgentCapability::Inference], None),
            )
            .await
            .expect_err("issuing into an unreadable file must fail");
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
        let on_disk = std::fs::read_to_string(&relays_path).expect("read relay tokens");
        assert_eq!(on_disk, "not-json");

        // The same holds for the agent-token file.
        let agents_path = agent_tokens_path(dir.path());
        std::fs::write(&agents_path, "not-json").expect("corrupt agent tokens");
        assert!(!agent_accepted(&registry, "roko_agent_root-secret").await);
        let error = registry
            .revoke_agent_token(&root.token_id)
            .await
            .expect_err("revoking in an unreadable file must fail");
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
        let on_disk = std::fs::read_to_string(&agents_path).expect("read agent tokens");
        assert_eq!(on_disk, "not-json");
    }

    #[tokio::test]
    async fn auth_disabled_relay_route_still_requires_and_accepts_agent_bearer() {
        use axum::body::Body;
        use axum::http::Request;
        use roko_core::config::RokoConfig;
        use tower::ServiceExt;

        use crate::deploy::manual::ManualBackend;
        use crate::runtime::NoOpRuntime;

        let dir = tmp_workdir();
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                RokoConfig::default(),
                Arc::new(ManualBackend::default()),
            )
            .expect("create state"),
        );
        let secret = "roko_agent_route-root";
        state
            .auth_registry
            .insert_agent_token(root_token(secret, vec![AgentCapability::Inference]))
            .await
            .expect("insert root token");
        let app = routes().with_state(Arc::clone(&state));
        let body = json!({
            "target_agent_id": "child",
            "delegated_capabilities": ["Inference"],
            "max_depth": 2,
        })
        .to_string();

        let anonymous = app
            .clone()
            .oneshot(
                Request::post("/relay-tokens")
                    .header("content-type", "application/json")
                    .body(Body::from(body.clone()))
                    .expect("anonymous request"),
            )
            .await
            .expect("anonymous response");
        assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

        let authenticated = app
            .oneshot(
                Request::post("/relay-tokens")
                    .header("content-type", "application/json")
                    .header(AUTHORIZATION, format!("Bearer {secret}"))
                    .body(Body::from(body))
                    .expect("authenticated request"),
            )
            .await
            .expect("authenticated response");
        assert_eq!(authenticated.status(), StatusCode::CREATED);
        assert_eq!(state.auth_registry.relay_tokens_snapshot().await.len(), 1);
        let events = state
            .auth_audit
            .as_ref()
            .expect("audit writer")
            .query(None, None, None, None)
            .expect("query audit");
        let issued = events.last().expect("relay issuance audit event");
        assert_eq!(issued.actor, "agent:root-agent");
        assert_eq!(
            issued.metadata.get("target_agent_id").map(String::as_str),
            Some("child")
        );
        assert_eq!(issued.metadata.get("depth").map(String::as_str), Some("1"));
        assert!(!issued.metadata.contains_key("token_hash_prefix"));
    }

    #[test]
    fn relay_defaults_are_bounded() {
        assert_eq!(DEFAULT_RELAY_TOKEN_TTL_SECS, 300);
        assert_eq!(default_relay_ttl_secs(), 300);
        assert!(DEFAULT_RELAY_MAX_DEPTH <= MAX_RELAY_MAX_DEPTH);
    }

    #[test]
    fn credential_hash_comparison_handles_equal_unequal_and_length_mismatch() {
        assert!(constant_time_hash_eq("abcdef", "abcdef"));
        assert!(!constant_time_hash_eq("abcdef", "abcdeg"));
        assert!(!constant_time_hash_eq("abcdef", "abc"));
    }
}
