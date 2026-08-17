//! Persistent authenticated extension registry for the relay.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::Engine as _;
use roko_plugin::registry::{
    RegistryPackage, ResolvedRegistryGraph, package_dependency_requirements,
    parse_registry_public_key, validate_registry_name, validate_registry_publisher,
    validate_signed_package,
};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Maximum accepted JSON publish request, including base64 expansion.
pub const MAX_PUBLISH_BODY_BYTES: usize = 96 * 1024 * 1024;

/// One publisher identity authorized by relay configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryPublisherConfig {
    pub id: String,
    /// Lowercase SHA-256 hex digest of the bearer token.
    pub token_sha256: String,
    /// Standard-base64 Ed25519 public key (32 bytes).
    pub public_key: String,
}

/// Publisher configuration file accepted by the relay executable.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryPublishersFile {
    #[serde(default)]
    pub publishers: Vec<RegistryPublisherConfig>,
}

#[derive(Debug)]
pub enum RegistryError {
    Unauthorized(String),
    Invalid(String),
    MissingDependency(String),
    NotFound(String),
    Conflict(String),
    Storage(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized(message)
            | Self::Invalid(message)
            | Self::MissingDependency(message)
            | Self::NotFound(message)
            | Self::Conflict(message)
            | Self::Storage(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for RegistryError {}

pub struct PublishOutcome {
    pub created: bool,
    pub package: RegistryPackage,
}

/// File-backed immutable registry with a configured publisher allowlist.
pub struct RegistryStore {
    root: PathBuf,
    publishers: BTreeMap<String, RegistryPublisherConfig>,
    write_lock: Mutex<()>,
}

impl RegistryStore {
    pub fn open(
        root: impl Into<PathBuf>,
        publishers: Vec<RegistryPublisherConfig>,
    ) -> Result<Self, RegistryError> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(storage)?;
        let mut configured = BTreeMap::new();
        for publisher in publishers {
            validate_publisher_config(&publisher)?;
            let id = publisher.id.clone();
            if configured.insert(id.clone(), publisher).is_some() {
                return Err(RegistryError::Invalid(format!(
                    "duplicate registry publisher `{id}`"
                )));
            }
        }
        Ok(Self {
            root,
            publishers: configured,
            write_lock: Mutex::new(()),
        })
    }

    #[must_use]
    pub fn can_publish(&self) -> bool {
        !self.publishers.is_empty()
    }

    /// Whether packages can be authenticated against configured publisher keys.
    #[must_use]
    pub fn can_read(&self) -> bool {
        !self.publishers.is_empty()
    }

    pub fn publish(
        &self,
        package: RegistryPackage,
        bearer_token: &str,
    ) -> Result<PublishOutcome, RegistryError> {
        let publisher = self.publishers.get(&package.publisher).ok_or_else(|| {
            RegistryError::Unauthorized("publisher identity is not authorized".to_string())
        })?;
        let supplied_hash = format!("{:x}", Sha256::digest(bearer_token.as_bytes()));
        if !constant_time_eq(supplied_hash.as_bytes(), publisher.token_sha256.as_bytes()) {
            return Err(RegistryError::Unauthorized(
                "publisher bearer token is invalid".to_string(),
            ));
        }
        if !constant_time_eq(
            package.publisher_public_key.as_bytes(),
            publisher.public_key.as_bytes(),
        ) {
            return Err(RegistryError::Unauthorized(
                "package signing key is not authorized for publisher".to_string(),
            ));
        }
        let validated = validate_signed_package(&package).map_err(RegistryError::Invalid)?;
        if base64::engine::general_purpose::STANDARD.encode(validated.verifying_key.as_bytes())
            != publisher.public_key
        {
            return Err(RegistryError::Unauthorized(
                "verified signing key does not match publisher configuration".to_string(),
            ));
        }

        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| RegistryError::Storage("registry write lock is poisoned".to_string()))?;
        if let Err(error) = self.solve_fixed_graph(package.clone(), Some(&package))? {
            return Err(match error {
                ResolutionFailure::Missing {
                    package: requiring,
                    dependency,
                    requirement,
                } => RegistryError::MissingDependency(format!(
                    "extension `{requiring}` requires unpublished dependency `{dependency}` matching `{requirement}`"
                )),
                error => RegistryError::Conflict(format!(
                    "extension {}@{} has an unresolvable dependency graph: {error}",
                    package.name, package.version
                )),
            });
        }
        let directory = self.root.join(&package.name);
        std::fs::create_dir_all(&directory).map_err(storage)?;
        let target = directory.join(format!("{}.json", package.version));
        let encoded = serde_json::to_vec(&package)
            .map_err(|error| RegistryError::Storage(format!("encode registry package: {error}")))?;
        if target.exists() {
            let existing = std::fs::read(&target).map_err(storage)?;
            if constant_time_eq(&existing, &encoded) {
                return Ok(PublishOutcome {
                    created: false,
                    package,
                });
            }
            return Err(RegistryError::Conflict(format!(
                "extension {}@{} is immutable and already published",
                package.name, package.version
            )));
        }
        let mut temporary = tempfile::NamedTempFile::new_in(&directory).map_err(storage)?;
        temporary.write_all(&encoded).map_err(storage)?;
        temporary.as_file().sync_all().map_err(storage)?;
        match temporary.persist_noclobber(&target) {
            Ok(file) => file.sync_all().map_err(storage)?,
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = std::fs::read(&target).map_err(storage)?;
                if constant_time_eq(&existing, &encoded) {
                    return Ok(PublishOutcome {
                        created: false,
                        package,
                    });
                }
                return Err(RegistryError::Conflict(format!(
                    "extension {}@{} is immutable and already published",
                    package.name, package.version
                )));
            }
            Err(error) => return Err(storage(error.error)),
        }
        Ok(PublishOutcome {
            created: true,
            package,
        })
    }

    pub fn resolve_latest(&self, name: &str) -> Result<RegistryPackage, RegistryError> {
        self.resolve(name, "*")
    }

    /// Resolve the highest version matching a semver requirement whose full
    /// dependency graph is conflict-free.
    pub fn resolve(&self, name: &str, requirement: &str) -> Result<RegistryPackage, RegistryError> {
        let graph = self.resolve_graph(name, requirement)?;
        graph.packages.last().cloned().ok_or_else(|| {
            RegistryError::Storage("registry resolver returned an empty graph".to_string())
        })
    }

    /// Resolve and return the complete dependency-first installation graph.
    pub fn resolve_graph(
        &self,
        name: &str,
        requirement: &str,
    ) -> Result<ResolvedRegistryGraph, RegistryError> {
        validate_registry_name(name).map_err(RegistryError::Invalid)?;
        let requirement = parse_requirement(requirement)?;
        let candidates = self.candidate_packages(name, &requirement, None)?;
        if candidates.is_empty() {
            return Err(RegistryError::NotFound(format!(
                "registry extension `{name}` has no version matching `{requirement}`"
            )));
        }
        let mut last_failure = None;
        for candidate in candidates {
            match self.solve_fixed_graph(candidate, None)? {
                Ok(graph) => return Ok(graph),
                Err(error) => last_failure = Some(error),
            }
        }
        Err(RegistryError::Conflict(format!(
            "registry extension `{name}` has no conflict-free version matching `{requirement}`: {}",
            last_failure
                .map(|error| error.to_string())
                .unwrap_or_else(|| "dependency resolution failed".to_string())
        )))
    }

    pub fn get(&self, name: &str, version: &str) -> Result<RegistryPackage, RegistryError> {
        validate_registry_name(name).map_err(RegistryError::Invalid)?;
        Version::parse(version).map_err(|error| {
            RegistryError::Invalid(format!("invalid version `{version}`: {error}"))
        })?;
        let path = self.root.join(name).join(format!("{version}.json"));
        let package = self.read_package_authenticated(&path)?;
        if package.name != name || package.version != version {
            return Err(RegistryError::Storage(format!(
                "stored package {} does not match requested identity {name}@{version}",
                path.display()
            )));
        }
        match self.solve_fixed_graph(package.clone(), None)? {
            Ok(_) => Ok(package),
            Err(error) => Err(RegistryError::Conflict(format!(
                "stored package {name}@{version} has an unresolvable dependency graph: {error}"
            ))),
        }
    }

    fn solve_fixed_graph(
        &self,
        root: RegistryPackage,
        injected: Option<&RegistryPackage>,
    ) -> Result<Result<ResolvedRegistryGraph, ResolutionFailure>, RegistryError> {
        let root_name = root.name.clone();
        let root_version = root.version.clone();
        let mut selected = BTreeMap::new();
        selected.insert(root_name.clone(), root);
        let mut stack = vec![root_name.clone()];
        let selected =
            match self.resolve_dependencies(&root_name, 0, selected, &mut stack, injected) {
                Ok(selected) => selected,
                Err(SolveError::Resolution(error)) => return Ok(Err(error)),
                Err(SolveError::Registry(error)) => return Err(error),
            };
        let packages = topological_packages(&root_name, &selected)?;
        Ok(Ok(ResolvedRegistryGraph {
            root: root_name,
            root_version,
            packages,
        }))
    }

    fn resolve_dependencies(
        &self,
        package_name: &str,
        dependency_index: usize,
        selected: BTreeMap<String, RegistryPackage>,
        stack: &mut Vec<String>,
        injected: Option<&RegistryPackage>,
    ) -> Result<BTreeMap<String, RegistryPackage>, SolveError> {
        let package = selected.get(package_name).cloned().ok_or_else(|| {
            SolveError::Registry(RegistryError::Storage(format!(
                "resolver lost selected package `{package_name}`"
            )))
        })?;
        let dependencies = package_dependency_requirements(&package)
            .map_err(|error| SolveError::Registry(RegistryError::Storage(error)))?
            .into_iter()
            .collect::<Vec<_>>();
        let Some((dependency, requirement)) = dependencies.get(dependency_index) else {
            return Ok(selected);
        };
        let requirement = VersionReq::parse(requirement).map_err(|error| {
            SolveError::Registry(RegistryError::Storage(format!(
                "stored dependency requirement is invalid: {error}"
            )))
        })?;

        if let Some(existing) = selected.get(dependency) {
            let selected_version = Version::parse(&existing.version).map_err(|error| {
                SolveError::Registry(RegistryError::Storage(format!(
                    "selected dependency version is invalid: {error}"
                )))
            })?;
            if !requirement.matches(&selected_version) {
                return Err(SolveError::Resolution(ResolutionFailure::Conflict {
                    package: package.name,
                    dependency: dependency.clone(),
                    requirement: requirement.to_string(),
                    selected: existing.version.clone(),
                }));
            }
            if stack.contains(dependency) {
                let mut cycle = stack.clone();
                cycle.push(dependency.clone());
                return Err(SolveError::Resolution(ResolutionFailure::Cycle(cycle)));
            }
            return self.resolve_dependencies(
                package_name,
                dependency_index + 1,
                selected,
                stack,
                injected,
            );
        }

        let candidates = self
            .candidate_packages(dependency, &requirement, injected)
            .map_err(SolveError::Registry)?;
        if candidates.is_empty() {
            return Err(SolveError::Resolution(ResolutionFailure::Missing {
                package: package.name,
                dependency: dependency.clone(),
                requirement: requirement.to_string(),
            }));
        }

        let mut last_failure = None;
        for candidate in candidates {
            let mut branch = selected.clone();
            branch.insert(dependency.clone(), candidate);
            stack.push(dependency.clone());
            let dependency_result =
                self.resolve_dependencies(dependency, 0, branch, stack, injected);
            stack.pop();
            let branch = match dependency_result {
                Ok(branch) => branch,
                Err(SolveError::Resolution(error)) => {
                    last_failure = Some(error);
                    continue;
                }
                Err(error) => return Err(error),
            };
            match self.resolve_dependencies(
                package_name,
                dependency_index + 1,
                branch,
                stack,
                injected,
            ) {
                Ok(branch) => return Ok(branch),
                Err(SolveError::Resolution(error)) => last_failure = Some(error),
                Err(error) => return Err(error),
            }
        }
        Err(SolveError::Resolution(last_failure.unwrap_or_else(|| {
            ResolutionFailure::Missing {
                package: package.name,
                dependency: dependency.clone(),
                requirement: requirement.to_string(),
            }
        })))
    }

    fn candidate_packages(
        &self,
        name: &str,
        requirement: &VersionReq,
        injected: Option<&RegistryPackage>,
    ) -> Result<Vec<RegistryPackage>, RegistryError> {
        validate_registry_name(name).map_err(RegistryError::Invalid)?;
        let directory = self.root.join(name);
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => Some(entries),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(storage(error)),
        };
        let mut candidates = BTreeMap::new();
        if let Some(entries) = entries {
            for entry in entries {
                let entry = entry.map_err(storage)?;
                let path = entry.path();
                if path.extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                    continue;
                };
                let Ok(version) = Version::parse(stem) else {
                    continue;
                };
                if !requirement.matches(&version) {
                    continue;
                }
                let package = self.read_package_authenticated(&path)?;
                if package.name != name || package.version != version.to_string() {
                    return Err(RegistryError::Storage(format!(
                        "stored package {} does not match filename identity {name}@{version}",
                        path.display()
                    )));
                }
                candidates.insert(version, package);
            }
        }
        if let Some(package) = injected
            && package.name == name
        {
            let version = Version::parse(&package.version).map_err(|error| {
                RegistryError::Invalid(format!(
                    "candidate package has invalid version `{}`: {error}",
                    package.version
                ))
            })?;
            if requirement.matches(&version) {
                candidates.insert(version, package.clone());
            }
        }
        Ok(candidates
            .into_iter()
            .rev()
            .map(|(_, package)| package)
            .collect())
    }

    fn read_package_authenticated(&self, path: &Path) -> Result<RegistryPackage, RegistryError> {
        let encoded = std::fs::read(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                RegistryError::NotFound(format!(
                    "registry package {} was not found",
                    path.display()
                ))
            } else {
                storage(error)
            }
        })?;
        let package: RegistryPackage = serde_json::from_slice(&encoded).map_err(|error| {
            RegistryError::Storage(format!("decode {}: {error}", path.display()))
        })?;
        let validated = validate_signed_package(&package).map_err(|error| {
            RegistryError::Storage(format!(
                "stored package {} is invalid: {error}",
                path.display()
            ))
        })?;
        let publisher = self.publishers.get(&package.publisher).ok_or_else(|| {
            RegistryError::Storage(format!(
                "stored package {} names unconfigured publisher `{}`",
                path.display(),
                package.publisher
            ))
        })?;
        let verified_key =
            base64::engine::general_purpose::STANDARD.encode(validated.verifying_key.as_bytes());
        if !constant_time_eq(verified_key.as_bytes(), publisher.public_key.as_bytes()) {
            return Err(RegistryError::Storage(format!(
                "stored package {} is not signed by the configured key for publisher `{}`",
                path.display(),
                package.publisher
            )));
        }
        Ok(package)
    }
}

#[derive(Debug)]
enum SolveError {
    Registry(RegistryError),
    Resolution(ResolutionFailure),
}

#[derive(Debug)]
enum ResolutionFailure {
    Missing {
        package: String,
        dependency: String,
        requirement: String,
    },
    Conflict {
        package: String,
        dependency: String,
        requirement: String,
        selected: String,
    },
    Cycle(Vec<String>),
}

impl std::fmt::Display for ResolutionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing {
                package,
                dependency,
                requirement,
            } => write!(
                formatter,
                "extension `{package}` requires unpublished dependency `{dependency}` matching `{requirement}`"
            ),
            Self::Conflict {
                package,
                dependency,
                requirement,
                selected,
            } => write!(
                formatter,
                "extension `{package}` requires `{dependency}` {requirement}, but version {selected} is already selected"
            ),
            Self::Cycle(path) => write!(
                formatter,
                "dependency cycle detected: {}",
                path.join(" -> ")
            ),
        }
    }
}

fn parse_requirement(requirement: &str) -> Result<VersionReq, RegistryError> {
    if requirement.trim().is_empty() {
        return Err(RegistryError::Invalid(
            "dependency version requirement cannot be empty".to_string(),
        ));
    }
    VersionReq::parse(requirement.trim()).map_err(|error| {
        RegistryError::Invalid(format!(
            "invalid dependency version requirement `{requirement}`: {error}"
        ))
    })
}

fn topological_packages(
    root: &str,
    selected: &BTreeMap<String, RegistryPackage>,
) -> Result<Vec<RegistryPackage>, RegistryError> {
    fn visit(
        name: &str,
        selected: &BTreeMap<String, RegistryPackage>,
        visited: &mut BTreeSet<String>,
        ordered: &mut Vec<RegistryPackage>,
    ) -> Result<(), RegistryError> {
        if !visited.insert(name.to_string()) {
            return Ok(());
        }
        let package = selected.get(name).ok_or_else(|| {
            RegistryError::Storage(format!("resolver omitted selected package `{name}`"))
        })?;
        for dependency in package_dependency_requirements(package)
            .map_err(RegistryError::Storage)?
            .keys()
        {
            visit(dependency, selected, visited, ordered)?;
        }
        ordered.push(package.clone());
        Ok(())
    }

    let mut visited = BTreeSet::new();
    let mut ordered = Vec::new();
    visit(root, selected, &mut visited, &mut ordered)?;
    if ordered.len() != selected.len() {
        return Err(RegistryError::Storage(
            "resolver selected unreachable packages".to_string(),
        ));
    }
    Ok(ordered)
}

fn validate_publisher_config(config: &RegistryPublisherConfig) -> Result<(), RegistryError> {
    validate_registry_publisher(&config.id).map_err(RegistryError::Invalid)?;
    if config.token_sha256.len() != 64 {
        return Err(RegistryError::Invalid(
            "publisher token_sha256 must contain 64 characters".to_string(),
        ));
    }
    if !config
        .token_sha256
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(RegistryError::Invalid(
            "publisher token_sha256 must be lowercase hexadecimal".to_string(),
        ));
    }
    parse_registry_public_key(&config.public_key).map_err(RegistryError::Invalid)?;
    Ok(())
}

fn storage(error: std::io::Error) -> RegistryError {
    RegistryError::Storage(error.to_string())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}
