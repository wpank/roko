//! The hidden-suite store (S05 §4.4): hidden tests in the vault, out of
//! every agent's reach, and their lifecycle.
//!
//! A suite lives at `<vault>/<workspace_id>/hidden/<suite_id>/`: its body
//! (`suite.py`, `suite.rs`, …) and `meta.json`, directories 0700 and files
//! 0600. The store is opened on a resolved [`AuditVault`], which refuses a
//! vault inside the workspace. Each body carries
//! `ROKO-CANARY-<suite_id>-<32 hex digits>` in a comment, which the canary
//! scanner looks for wherever an agent's words go (7117).
//!
//! States move drafted → validated → active → exposed → retired, and
//! drafted or validated → rejected ([`SuiteState::can_move_to`]); any other
//! move is an error. An active suite retires after [`MAX_USES`] audits or
//! [`MAX_AGE_DAYS`] days, whichever comes first, and a suite shown to a fix
//! task is burned: exposed, then retired. Every move appends an
//! `audit.hidden_suite` event to the ledger, which never holds a body.
//! Authoring and validation belong to the audit worker (7124); this is
//! storage and state only.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use roko_core::audit_home::AuditVault;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::ledger::{AuditEvent, AuditLedger};
use super::policy::hex;
use super::random_hex;

/// Audits an active suite serves before it retires.
pub const MAX_USES: u32 = 20;
/// Days an active suite serves before it retires.
pub const MAX_AGE_DAYS: i64 = 30;
/// The canary prefix every suite carries.
pub const CANARY_PREFIX: &str = "ROKO-CANARY-";
/// A suite's metadata file.
pub const META_FILE: &str = "meta.json";

/// Where a suite is in its life (S05 §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuiteState {
    /// Written from the spec, not yet checked.
    Drafted,
    /// It compiles against the base, fails there and passes a reference.
    Validated,
    /// In use by audits.
    Active,
    /// Seen outside the vault: a canary hit, or shown to a fix task.
    Exposed,
    /// Out of use for good.
    Retired,
    /// Failed validation; B1 is null for its units.
    Rejected,
}

impl SuiteState {
    /// Whether a suite may move from `self` to `next`.
    #[must_use]
    pub const fn can_move_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Drafted, Self::Validated | Self::Rejected)
                | (Self::Validated, Self::Active | Self::Rejected)
                | (Self::Active, Self::Exposed | Self::Retired)
                | (Self::Exposed, Self::Retired)
        )
    }

    /// The state's name in the ledger.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Drafted => "drafted",
            Self::Validated => "validated",
            Self::Active => "active",
            Self::Exposed => "exposed",
            Self::Retired => "retired",
            Self::Rejected => "rejected",
        }
    }
}

/// A suite's `meta.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiteMeta {
    /// The suite.
    pub suite_id: String,
    /// The task it tests.
    pub task_id: String,
    /// The spec it was written from.
    pub spec_hash: String,
    /// "sha256:" + SHA-256 of the body, canary included.
    pub suite_hash: String,
    /// The model that wrote it.
    pub author_model: String,
    /// That model's family.
    pub author_family: String,
    /// The implementer's family, which the author's must differ from.
    pub implementer_family: String,
    /// Its state.
    pub state: SuiteState,
    /// Audits that used it.
    pub uses: u32,
    /// When it was drafted.
    pub created_at: DateTime<Utc>,
    /// Its canary.
    pub canary: String,
    /// The body's file name in the suite's directory.
    pub file_name: String,
}

/// A new suite, as the author wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteDraft {
    /// The task it tests.
    pub task_id: String,
    /// The spec it was written from.
    pub spec_hash: String,
    /// The model that wrote it.
    pub author_model: String,
    /// That model's family.
    pub author_family: String,
    /// The implementer's family.
    pub implementer_family: String,
    /// The body's file name, such as `suite.py`.
    pub file_name: String,
    /// What starts a line comment in the body's language: `#` or `//`.
    pub comment: String,
    /// The tests.
    pub body: String,
}

/// Why the store refused.
#[derive(Debug, thiserror::Error)]
pub enum HiddenError {
    /// A file could not be read or written.
    #[error("hidden-suite store: {0}")]
    Io(#[from] std::io::Error),
    /// A `meta.json` does not parse.
    #[error("hidden suite {0}: its meta.json does not parse: {1}")]
    BadMeta(String, String),
    /// No such suite.
    #[error("no hidden suite {0}")]
    Unknown(String),
    /// A move the lifecycle forbids.
    #[error("hidden suite {suite_id} cannot move from {from} to {to}")]
    Illegal {
        /// The suite.
        suite_id: String,
        /// Its state.
        from: &'static str,
        /// The state asked for.
        to: &'static str,
    },
    /// A file name that would leave the suite's directory.
    #[error("hidden suite file name {0:?} is not a plain file name")]
    BadFileName(String),
}

/// The clock the store reads: the wall clock, or a test's.
pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// The hidden suites of one workspace.
#[derive(Clone)]
pub struct HiddenStore {
    dir: PathBuf,
    clock: Clock,
}

impl std::fmt::Debug for HiddenStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HiddenStore")
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

impl HiddenStore {
    /// The store in `vault`'s `hidden/`.
    ///
    /// # Errors
    ///
    /// The directory cannot be created.
    pub fn open(vault: &AuditVault) -> std::io::Result<Self> {
        Self::open_dir(&vault.hidden_dir())
    }

    /// The store in `dir` (the vault's `hidden/` in production).
    ///
    /// # Errors
    ///
    /// The directory cannot be created.
    pub fn open_dir(dir: &Path) -> std::io::Result<Self> {
        private_dir(dir)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            clock: Arc::new(Utc::now),
        })
    }

    /// The store, reading `clock` for the time.
    #[must_use]
    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    /// Store `draft` as a drafted suite, with its canary in a comment on the
    /// first line, and log it.
    ///
    /// # Errors
    ///
    /// A bad file name, or the store or the ledger cannot be written.
    pub fn draft(
        &self,
        ledger: &mut AuditLedger,
        draft: SuiteDraft,
    ) -> Result<SuiteMeta, HiddenError> {
        let plain = Path::new(&draft.file_name)
            .file_name()
            .and_then(|name| name.to_str());
        if plain != Some(draft.file_name.as_str()) || draft.file_name == META_FILE {
            return Err(HiddenError::BadFileName(draft.file_name));
        }
        let suite_id = format!("hs-{}", random_hex(6)?);
        let canary = format!("{CANARY_PREFIX}{suite_id}-{}", random_hex(16)?);
        let body = format!("{} {canary}\n{}", draft.comment, draft.body);
        let suite_dir = self.dir.join(&suite_id);
        private_dir(&suite_dir)?;
        write_private(&suite_dir.join(&draft.file_name), body.as_bytes())?;
        let meta = SuiteMeta {
            suite_id,
            task_id: draft.task_id,
            spec_hash: draft.spec_hash,
            suite_hash: format!("sha256:{}", hex(&Sha256::digest(body.as_bytes()))),
            author_model: draft.author_model,
            author_family: draft.author_family,
            implementer_family: draft.implementer_family,
            state: SuiteState::Drafted,
            uses: 0,
            created_at: (self.clock)(),
            canary,
            file_name: draft.file_name,
        };
        self.save(&meta)?;
        log(ledger, &meta, None, "drafted")?;
        Ok(meta)
    }

    /// Move suite `suite_id` to `to`, and log why.
    ///
    /// # Errors
    ///
    /// An unknown suite, a move [`SuiteState::can_move_to`] forbids, or the
    /// store or the ledger cannot be written.
    pub fn transition(
        &self,
        ledger: &mut AuditLedger,
        suite_id: &str,
        to: SuiteState,
        reason: &str,
    ) -> Result<SuiteMeta, HiddenError> {
        let mut meta = self.meta(suite_id)?;
        let from = meta.state;
        if !from.can_move_to(to) {
            return Err(HiddenError::Illegal {
                suite_id: suite_id.to_string(),
                from: from.label(),
                to: to.label(),
            });
        }
        meta.state = to;
        self.save(&meta)?;
        log(ledger, &meta, Some(from), reason)?;
        Ok(meta)
    }

    /// Count one audit's use of an active suite; at [`MAX_USES`] it
    /// retires.
    ///
    /// # Errors
    ///
    /// As [`Self::transition`], for a suite that is not active too.
    pub fn record_use(
        &self,
        ledger: &mut AuditLedger,
        suite_id: &str,
    ) -> Result<SuiteMeta, HiddenError> {
        let mut meta = self.meta(suite_id)?;
        if meta.state != SuiteState::Active {
            return Err(HiddenError::Illegal {
                suite_id: suite_id.to_string(),
                from: meta.state.label(),
                to: "used",
            });
        }
        meta.uses += 1;
        self.save(&meta)?;
        if meta.uses >= MAX_USES {
            return self.transition(ledger, suite_id, SuiteState::Retired, "max_uses");
        }
        Ok(meta)
    }

    /// Retire every active suite that served [`MAX_USES`] audits or is
    /// [`MAX_AGE_DAYS`] days old; returns the suites retired.
    ///
    /// # Errors
    ///
    /// The store or the ledger cannot be read or written.
    pub fn rotate(&self, ledger: &mut AuditLedger) -> Result<Vec<SuiteMeta>, HiddenError> {
        let now = (self.clock)();
        let mut retired = Vec::new();
        for meta in self.all()? {
            if meta.state != SuiteState::Active {
                continue;
            }
            let reason = if meta.uses >= MAX_USES {
                "max_uses"
            } else if now - meta.created_at >= Duration::days(MAX_AGE_DAYS) {
                "age"
            } else {
                continue;
            };
            retired.push(self.transition(ledger, &meta.suite_id, SuiteState::Retired, reason)?);
        }
        Ok(retired)
    }

    /// Burn a suite a fix task was shown (`show_failing_hidden_tests`):
    /// exposed, then retired.
    ///
    /// # Errors
    ///
    /// As [`Self::transition`].
    pub fn burn_shown(
        &self,
        ledger: &mut AuditLedger,
        suite_id: &str,
    ) -> Result<SuiteMeta, HiddenError> {
        if self.meta(suite_id)?.state == SuiteState::Active {
            self.transition(ledger, suite_id, SuiteState::Exposed, "shown_to_fixer")?;
        }
        self.transition(ledger, suite_id, SuiteState::Retired, "shown_to_fixer")
    }

    /// The active suite for `task_id`, the newest when there are several.
    ///
    /// # Errors
    ///
    /// The store cannot be read.
    pub fn active_for(&self, task_id: &str) -> Result<Option<SuiteMeta>, HiddenError> {
        Ok(self
            .all()?
            .into_iter()
            .filter(|meta| meta.state == SuiteState::Active && meta.task_id == task_id)
            .max_by_key(|meta| meta.created_at))
    }

    /// The suite whose canary is `canary`, if this store holds it.
    ///
    /// # Errors
    ///
    /// The store cannot be read.
    pub fn by_canary(&self, canary: &str) -> Result<Option<SuiteMeta>, HiddenError> {
        Ok(self
            .all()?
            .into_iter()
            .find(|meta| meta.canary.eq_ignore_ascii_case(canary)))
    }

    /// Suite `suite_id`'s metadata.
    ///
    /// # Errors
    ///
    /// An unknown suite, or a `meta.json` that does not parse.
    pub fn meta(&self, suite_id: &str) -> Result<SuiteMeta, HiddenError> {
        let plain = Path::new(suite_id)
            .file_name()
            .and_then(|name| name.to_str());
        if plain != Some(suite_id) {
            return Err(HiddenError::Unknown(suite_id.to_string()));
        }
        let text = match std::fs::read_to_string(self.dir.join(suite_id).join(META_FILE)) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(HiddenError::Unknown(suite_id.to_string()));
            }
            Err(error) => return Err(error.into()),
        };
        serde_json::from_str(&text)
            .map_err(|error| HiddenError::BadMeta(suite_id.to_string(), error.to_string()))
    }

    /// The path of a suite's body.
    #[must_use]
    pub fn body_path(&self, meta: &SuiteMeta) -> PathBuf {
        self.dir.join(&meta.suite_id).join(&meta.file_name)
    }

    /// Every suite's metadata.
    ///
    /// # Errors
    ///
    /// The store cannot be read, or a `meta.json` does not parse.
    pub fn all(&self) -> Result<Vec<SuiteMeta>, HiddenError> {
        let mut suites = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                suites.push(self.meta(&name)?);
            }
        }
        suites.sort_by(|left, right| left.suite_id.cmp(&right.suite_id));
        Ok(suites)
    }

    fn save(&self, meta: &SuiteMeta) -> Result<(), HiddenError> {
        let text = serde_json::to_string_pretty(meta)
            .map_err(|error| HiddenError::BadMeta(meta.suite_id.clone(), error.to_string()))?;
        let dir = self.dir.join(&meta.suite_id);
        let staged = dir.join(format!(".{META_FILE}.tmp"));
        write_private(&staged, text.as_bytes())?;
        std::fs::rename(&staged, dir.join(META_FILE))?;
        Ok(())
    }
}

/// Append the `audit.hidden_suite` event of a move; never the body.
fn log(
    ledger: &mut AuditLedger,
    meta: &SuiteMeta,
    from: Option<SuiteState>,
    reason: &str,
) -> std::io::Result<()> {
    ledger.append(AuditEvent::HiddenSuite {
        suite_id: meta.suite_id.clone(),
        task_id: meta.task_id.clone(),
        spec_hash: meta.spec_hash.clone(),
        suite_hash: meta.suite_hash.clone(),
        author_model: meta.author_model.clone(),
        author_family: meta.author_family.clone(),
        implementer_family: meta.implementer_family.clone(),
        from: from.map(|state| state.label().to_string()),
        to: meta.state.label().to_string(),
        reason: reason.to_string(),
        uses: meta.uses,
    })?;
    Ok(())
}

/// Create `dir` and its parents, mode 0700.
pub(super) fn private_dir(dir: &Path) -> std::io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(dir)
}

/// Write `bytes` to `path`, mode 0600, and sync it.
pub(super) fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::audit::ledger::verify_chain;

    pub(crate) fn draft(task_id: &str) -> SuiteDraft {
        SuiteDraft {
            task_id: task_id.to_string(),
            spec_hash: "sha256:spec".into(),
            author_model: "glm-4.7".into(),
            author_family: "zhipu".into(),
            implementer_family: "openai".into(),
            file_name: "suite.py".into(),
            comment: "#".into(),
            body: "def test_upper_bound_stays_hidden():\n    assert True\n".into(),
        }
    }

    fn fixture() -> (tempfile::TempDir, HiddenStore, AuditLedger) {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = HiddenStore::open_dir(&temp.path().join("hidden")).expect("a store");
        let ledger = AuditLedger::open_dir(&temp.path().join("ledger")).expect("a ledger");
        (temp, store, ledger)
    }

    #[test]
    fn hidden_suite_lifecycle_rejects_illegal_transitions() {
        use SuiteState::{Active, Drafted, Exposed, Rejected, Retired, Validated};
        let (temp, store, mut ledger) = fixture();
        let meta = store.draft(&mut ledger, draft("T1")).expect("a draft");
        let id = meta.suite_id.as_str();
        assert_eq!(meta.state, Drafted);
        let body = std::fs::read_to_string(store.body_path(&meta)).expect("the body");
        assert!(body.starts_with(&format!("# {}", meta.canary)), "{body}");
        assert!(meta.canary.starts_with(&format!("{CANARY_PREFIX}{id}-")));
        assert_eq!(meta.canary.len(), CANARY_PREFIX.len() + id.len() + 1 + 32);
        let digest = format!("sha256:{}", hex(&Sha256::digest(body.as_bytes())));
        assert_eq!(meta.suite_hash, digest);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |path: &Path| {
                let metadata = std::fs::metadata(path).expect("metadata");
                metadata.permissions().mode() & 0o777
            };
            assert_eq!(mode(&temp.path().join("hidden").join(id)), 0o700);
            assert_eq!(mode(&store.body_path(&meta)), 0o600);
        }

        let illegal = |store: &HiddenStore, ledger: &mut AuditLedger, to| {
            let error = store
                .transition(ledger, id, to, "test")
                .expect_err("illegal");
            assert!(matches!(error, HiddenError::Illegal { .. }), "{error}");
        };
        illegal(&store, &mut ledger, Active);
        illegal(&store, &mut ledger, Exposed);
        store
            .transition(&mut ledger, id, Validated, "validated")
            .expect("validate");
        illegal(&store, &mut ledger, Drafted);
        store
            .transition(&mut ledger, id, Active, "activated")
            .expect("activate");
        illegal(&store, &mut ledger, Validated);
        store
            .transition(&mut ledger, id, Exposed, "canary_hit")
            .expect("expose");
        illegal(&store, &mut ledger, Active);
        store
            .transition(&mut ledger, id, Retired, "canary_hit")
            .expect("retire");
        for to in [Drafted, Validated, Active, Exposed, Rejected] {
            illegal(&store, &mut ledger, to);
        }
        let other = store
            .draft(&mut ledger, draft("T2"))
            .expect("a second draft");
        let other_id = other.suite_id.as_str();
        store
            .transition(&mut ledger, other_id, Rejected, "fails on base")
            .expect("reject");
        let error = store
            .transition(&mut ledger, other_id, Validated, "again")
            .expect_err("rejected is terminal");
        assert!(matches!(error, HiddenError::Illegal { .. }), "{error}");
        assert!(matches!(
            store.meta("hs-none"),
            Err(HiddenError::Unknown(_))
        ));
        assert!(matches!(store.meta("../x"), Err(HiddenError::Unknown(_))));

        // Every move is logged; no body is.
        let ledger_dir = temp.path().join("ledger");
        assert_eq!(verify_chain(&ledger_dir), Ok(7));
        for file in std::fs::read_dir(&ledger_dir).expect("ledger files") {
            let text = std::fs::read_to_string(file.expect("entry").path()).unwrap_or_default();
            assert!(!text.contains("test_upper_bound_stays_hidden"));
        }
    }

    #[test]
    fn suites_retire_after_20_uses_or_30_days() {
        let (_temp, store, mut ledger) = fixture();
        let start = Utc::now();
        let now = Arc::new(Mutex::new(start));
        let clock_now = Arc::clone(&now);
        let clock: Clock = Arc::new(move || *clock_now.lock().expect("the clock"));
        let store = store.with_clock(clock);
        let activate = |store: &HiddenStore, ledger: &mut AuditLedger, task: &str| {
            let meta = store.draft(ledger, draft(task)).expect("a draft");
            let id = meta.suite_id;
            store
                .transition(ledger, &id, SuiteState::Validated, "ok")
                .expect("validate");
            store
                .transition(ledger, &id, SuiteState::Active, "ok")
                .expect("activate");
            id
        };

        // 20 uses.
        let used = activate(&store, &mut ledger, "T-used");
        for _ in 1..MAX_USES {
            let meta = store.record_use(&mut ledger, &used).expect("a use");
            assert_eq!(meta.state, SuiteState::Active);
        }
        let last = store.record_use(&mut ledger, &used).expect("the 20th use");
        assert_eq!((last.state, last.uses), (SuiteState::Retired, MAX_USES));
        assert!(store.record_use(&mut ledger, &used).is_err());

        // 30 days.
        let aged = activate(&store, &mut ledger, "T-aged");
        let found = store.active_for("T-aged").expect("lookup");
        assert_eq!(found.map(|meta| meta.suite_id), Some(aged.clone()));
        *now.lock().expect("the clock") = start + Duration::days(29);
        assert!(store.rotate(&mut ledger).expect("rotate").is_empty());
        *now.lock().expect("the clock") = start + Duration::days(MAX_AGE_DAYS);
        let retired = store.rotate(&mut ledger).expect("rotate");
        assert_eq!(retired.len(), 1);
        assert_eq!(retired[0].suite_id, aged);
        assert_eq!(store.active_for("T-aged").expect("lookup"), None);

        // A suite shown to a fix task burns: exposed, then retired.
        let shown = activate(&store, &mut ledger, "T-shown");
        let burned = store.burn_shown(&mut ledger, &shown).expect("burn");
        assert_eq!(burned.state, SuiteState::Retired);
        let canary = burned.canary.to_ascii_lowercase();
        let found = store.by_canary(&canary).expect("lookup");
        assert_eq!(found.map(|meta| meta.suite_id), Some(shown));
    }
}
