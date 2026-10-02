//! Plan-set scheduling for Graph plan runs: the order a selected plan set
//! runs in, and which of its plans may share the working tree at once.
//!
//! Every plan of a set runs in the operator's working tree. In execution
//! order, a plan starts once its `depends_on_plan` prerequisites have
//! succeeded, a slot under `max_parallel_plans` is free, and its
//! [`PlanFootprint`] overlaps no running plan's. With one slot this is the
//! sequential order the runner has always used.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Bound::{Excluded, Unbounded};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use anyhow::{Result, bail};

use crate::graph_checkpoint::GraphCheckpointStatus;
use crate::runner::plan_loader::Plan;

// ── Execution order and prerequisites ────────────────────────────────────

/// Execution order and plan-level prerequisites of a selected plan set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanSetOrder {
    /// Plan IDs, prerequisites first and otherwise by ID.
    pub order: Vec<String>,
    /// Each plan's prerequisites inside the selected set.
    pub dependencies: BTreeMap<String, BTreeSet<String>>,
    /// Prerequisites outside the selected set that are already complete on
    /// disk, with the evidence that satisfied each.
    pub satisfied_outside: BTreeMap<String, String>,
}

/// State on disk of a prerequisite plan that is not in the selected set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutsidePlanStatus {
    /// Complete, with the evidence: a succeeded Graph checkpoint, or every
    /// task of its `tasks.toml` done.
    Complete(String),
    /// Present but not complete, with its status.
    Incomplete(String),
    /// No plan with that ID exists in the workspace.
    Missing,
}

/// Order a selected plan set.
///
/// A `depends_on_plan` that names a plan outside the set is resolved
/// against that plan's state on disk ([`outside_plan_status`]): a complete
/// plan satisfies it, and anything else rejects the whole set before a
/// provider starts, naming the plan and its status. Duplicate plan IDs,
/// self-dependencies and dependency cycles are rejected as well.
pub fn plan_set_order(workdir: &Path, plans_dir: &Path, plans: &[Plan]) -> Result<PlanSetOrder> {
    order_plan_set(plans, |plan_id| {
        outside_plan_status(workdir, plans_dir, plan_id)
    })
}

fn order_plan_set(
    plans: &[Plan],
    mut outside_status: impl FnMut(&str) -> OutsidePlanStatus,
) -> Result<PlanSetOrder> {
    let mut plan_ids = BTreeSet::new();
    for plan in plans {
        if !plan_ids.insert(plan.id.as_str()) {
            bail!(
                "Graph selected plan set contains duplicate plan ID '{}'",
                plan.id
            );
        }
    }

    let mut dependencies = BTreeMap::new();
    let mut satisfied_outside = BTreeMap::new();
    for plan in plans {
        let mut inside = BTreeSet::new();
        for dependency in plan
            .tasks
            .tasks
            .iter()
            .flat_map(|task| &task.depends_on_plan)
        {
            if plan_ids.contains(dependency.as_str()) {
                inside.insert(dependency.clone());
                continue;
            }
            if satisfied_outside.contains_key(dependency) {
                continue;
            }
            match outside_status(dependency) {
                OutsidePlanStatus::Complete(evidence) => {
                    satisfied_outside.insert(dependency.clone(), evidence);
                }
                OutsidePlanStatus::Incomplete(status) => bail!(
                    "Graph plan '{}' depends on plan '{dependency}', which is not in the selected \
                     plan set and is not complete (status: {status}); run '{dependency}' first, \
                     or select both plans together",
                    plan.id
                ),
                OutsidePlanStatus::Missing => bail!(
                    "Graph plan '{}' depends on unknown plan '{dependency}': it is not in the \
                     selected plan set and no plan with that ID exists in the workspace",
                    plan.id
                ),
            }
        }
        dependencies.insert(plan.id.clone(), inside);
    }

    let order = topological_order(&dependencies)?;
    Ok(PlanSetOrder {
        order,
        dependencies,
        satisfied_outside,
    })
}

/// Plans in dependency order, ties broken by plan ID.
fn topological_order(dependencies: &BTreeMap<String, BTreeSet<String>>) -> Result<Vec<String>> {
    let mut indegree = BTreeMap::new();
    let mut dependents = BTreeMap::<String, BTreeSet<String>>::new();
    for (plan_id, plan_dependencies) in dependencies {
        for dependency in plan_dependencies {
            if dependency == plan_id {
                bail!("Graph plan '{plan_id}' cannot depend on itself");
            }
            if !dependencies.contains_key(dependency) {
                bail!(
                    "Graph plan '{plan_id}' depends on unknown plan '{dependency}' in the selected plan set"
                );
            }
            dependents
                .entry(dependency.clone())
                .or_default()
                .insert(plan_id.clone());
        }
        indegree.insert(plan_id.clone(), plan_dependencies.len());
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(plan_id, degree)| (*degree == 0).then_some(plan_id.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(dependencies.len());
    while let Some(plan_id) = ready.pop_first() {
        if let Some(plan_dependents) = dependents.get(&plan_id) {
            for dependent in plan_dependents {
                let Some(degree) = indegree.get_mut(dependent) else {
                    bail!("Graph plan dependency index is inconsistent for '{dependent}'");
                };
                *degree = degree.saturating_sub(1);
                if *degree == 0 {
                    ready.insert(dependent.clone());
                }
            }
        }
        order.push(plan_id);
    }

    if order.len() != dependencies.len() {
        let cycle = indegree
            .into_iter()
            .filter_map(|(plan_id, degree)| (degree > 0).then_some(plan_id))
            .collect::<Vec<_>>();
        bail!(
            "Graph plan dependency cycle involving: {}",
            cycle.join(", ")
        );
    }

    Ok(order)
}

/// State on disk of `plan_id`, a plan outside the selected set.
///
/// A succeeded checkpoint under `.roko/state/graph/` makes it complete.
/// Otherwise its `tasks.toml` decides; the plan is looked up next to the
/// selected plans first, then under the workspace plans directory.
pub fn outside_plan_status(workdir: &Path, plans_dir: &Path, plan_id: &str) -> OutsidePlanStatus {
    let checkpoint = crate::graph_checkpoint::canonical_checkpoint_status(workdir, plan_id);
    if checkpoint == Some(GraphCheckpointStatus::Succeeded) {
        return OutsidePlanStatus::Complete("its Graph checkpoint succeeded".to_string());
    }
    let summary = locate_plan(workdir, plans_dir, plan_id)
        .map(|plan_info| crate::plan::summarize_plan_info(&plan_info));
    if let Some(summary) = &summary
        && summary.completed
    {
        return OutsidePlanStatus::Complete(format!("all {} tasks are done", summary.task_count));
    }
    match (checkpoint, summary) {
        (Some(status), _) => {
            OutsidePlanStatus::Incomplete(format!("checkpoint {}", checkpoint_label(status)))
        }
        (None, Some(summary)) => OutsidePlanStatus::Incomplete(summary.status_label()),
        (None, None) => OutsidePlanStatus::Missing,
    }
}

const fn checkpoint_label(status: GraphCheckpointStatus) -> &'static str {
    match status {
        GraphCheckpointStatus::Running => "running",
        GraphCheckpointStatus::Succeeded => "succeeded",
        GraphCheckpointStatus::Unverified => "unverified",
        GraphCheckpointStatus::Failed => "failed",
        GraphCheckpointStatus::Cancelled => "cancelled",
        GraphCheckpointStatus::Interrupted => "interrupted",
    }
}

/// Find `plan_id` in the selected plans' own set (the parent directory when
/// a single plan was selected), then under the workspace plans directory.
/// A selected directory outside the workspace is never scanned.
fn locate_plan(
    workdir: &Path,
    plans_dir: &Path,
    plan_id: &str,
) -> Option<crate::orchestrator::PlanInfo> {
    let own_set = if plans_dir.join("tasks.toml").is_file() {
        plans_dir.parent()
    } else {
        Some(plans_dir)
    };
    let workspace = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());
    let own_set = own_set.filter(|root| {
        root.canonicalize()
            .is_ok_and(|root| root.starts_with(&workspace))
    });
    own_set
        .map(Path::to_path_buf)
        .into_iter()
        .chain(std::iter::once(crate::plan::plans_dir(workdir)))
        .filter(|root| root.is_dir())
        .filter_map(|root| crate::orchestrator::discover_plans(&root).ok())
        .flatten()
        .find(|plan_info| {
            plan_info.base == plan_id || crate::plan::stable_plan_id(plan_info) == plan_id
        })
}

// ── Footprints ───────────────────────────────────────────────────────────

/// How long `cargo metadata` may take before footprints fall back to
/// treating every Rust edit as touching every package.
const CARGO_METADATA_TIMEOUT: Duration = Duration::from_secs(60);

/// Files whose presence makes a directory its own project: a task's file
/// below one belongs to that whole project.
const PROJECT_MANIFESTS: [&str; 4] = ["package.json", "go.mod", "pyproject.toml", "Cargo.toml"];

/// Cargo subcommands that neither build nor read the workspace sources.
const NON_BUILDING_CARGO_COMMANDS: [&str; 9] = [
    "metadata",
    "tree",
    "pkgid",
    "locate-project",
    "search",
    "version",
    "help",
    "--version",
    "--list",
];

/// The Cargo package graph footprints need: where each workspace member
/// lives, and which members build on it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CargoWorkspace {
    /// Member name → package root, relative to the workspace root.
    roots: BTreeMap<String, PathBuf>,
    /// Member name → members that depend on it, directly or transitively.
    dependents: BTreeMap<String, BTreeSet<String>>,
}

impl CargoWorkspace {
    /// Read the package graph of the Cargo workspace at `workdir`.
    ///
    /// `None` when `workdir` has no `Cargo.toml` or `cargo metadata` fails;
    /// footprints then treat every Rust edit as touching every package.
    pub async fn load(workdir: &Path) -> Option<Self> {
        if !workdir.join("Cargo.toml").is_file() {
            return None;
        }
        match crate::runner::impact_analysis::cargo_metadata(workdir, CARGO_METADATA_TIMEOUT).await
        {
            Ok(metadata) => Some(Self::from_metadata(workdir, &metadata)),
            Err(error) => {
                tracing::warn!(
                    %error,
                    "cargo metadata failed; plans that touch Rust code will not run at the same time"
                );
                None
            }
        }
    }

    fn from_metadata(workdir: &Path, metadata: &cargo_metadata::Metadata) -> Self {
        let workspace = workdir
            .canonicalize()
            .unwrap_or_else(|_| workdir.to_path_buf());
        let roots = metadata
            .workspace_packages()
            .into_iter()
            .filter_map(|package| {
                let root = package.manifest_path.as_std_path().parent()?;
                let relative = root
                    .strip_prefix(&workspace)
                    .or_else(|_| root.strip_prefix(workdir))
                    .ok()?;
                Some((package.name.to_string(), relative.to_path_buf()))
            })
            .collect::<BTreeMap<_, _>>();
        let dependents = roots
            .keys()
            .map(|name| {
                let (dependents, _) = crate::runner::impact_analysis::reverse_dependents(
                    metadata,
                    std::slice::from_ref(name),
                    usize::MAX,
                );
                (name.clone(), dependents.into_iter().collect())
            })
            .collect();
        Self { roots, dependents }
    }

    /// The workspace member owning `path`: the member with the deepest root
    /// containing it. A member at the workspace root owns nothing, so files
    /// outside every other member stay unowned.
    fn owner(&self, path: &Path) -> Option<(&str, &Path)> {
        self.roots
            .iter()
            .filter(|(_, root)| !root.as_os_str().is_empty() && path.starts_with(root))
            .max_by_key(|(_, root)| root.components().count())
            .map(|(name, root)| (name.as_str(), root.as_path()))
    }

    fn package(&self, name: &str) -> Option<Area> {
        self.roots.get(name).map(|root| Area::Package {
            name: name.to_string(),
            root: root.clone(),
        })
    }
}

/// Part of the workspace a plan writes or builds.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Area {
    /// Every Rust package: a whole-workspace build, or any Rust edit when
    /// the package graph is unknown.
    AllPackages,
    /// One Cargo workspace member, rooted at `root`.
    Package {
        /// Package name.
        name: String,
        /// Package directory, relative to the workspace root.
        root: PathBuf,
    },
    /// A directory and everything below it, such as a JavaScript app.
    Tree(PathBuf),
    /// One file.
    File(PathBuf),
}

impl Area {
    fn overlaps(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::AllPackages, Self::AllPackages | Self::Package { .. })
            | (Self::Package { .. }, Self::AllPackages) => true,
            (Self::AllPackages, _) | (_, Self::AllPackages) => false,
            (Self::Package { name: left, .. }, Self::Package { name: right, .. }) => left == right,
            _ => {
                let (left, right) = (self.path(), other.path());
                left.starts_with(right) || right.starts_with(left)
            }
        }
    }

    fn path(&self) -> &Path {
        match self {
            Self::AllPackages => Path::new(""),
            Self::Package { root, .. } => root,
            Self::Tree(path) | Self::File(path) => path,
        }
    }
}

impl fmt::Display for Area {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllPackages => formatter.write_str("every Rust package"),
            Self::Package { name, .. } => formatter.write_str(name),
            Self::Tree(path) | Self::File(path) => write!(formatter, "{}", path.display()),
        }
    }
}

/// What a plan writes and builds, used to decide which plans may share the
/// working tree at the same time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanFootprint {
    /// Set, with the reason, when the plan must run alone.
    pub exclusive: Option<String>,
    /// Areas the plan's tasks write: their `files` and `crates_touched`.
    pub writes: BTreeSet<Area>,
    /// Areas the plan builds or tests: what it writes, plus what its verify
    /// commands build.
    pub builds: BTreeSet<Area>,
    /// Areas whose build sees the plan's edits: what it writes, plus every
    /// workspace package that depends on a package it writes.
    pub affects: BTreeSet<Area>,
}

impl PlanFootprint {
    /// Footprint of `plan` in the workspace at `workdir`.
    #[must_use]
    pub fn of(plan: &Plan, workdir: &Path, cargo: Option<&CargoWorkspace>) -> Self {
        let areas = AreaResolver {
            workdir,
            cargo,
            rust_workspace: workdir.join("Cargo.toml").is_file(),
        };
        let mut footprint = Self::default();
        for task in &plan.tasks.tasks {
            let crates = task.crates_touched.as_deref().unwrap_or_default();
            if task.files.is_empty() && crates.is_empty() {
                footprint
                    .exclusive
                    .get_or_insert_with(|| format!("its task {} declares no files", task.id));
            }
            for file in &task.files {
                let path = workspace_relative(workdir, file);
                if is_workspace_build_input(&path) {
                    footprint.exclusive.get_or_insert_with(|| {
                        format!("its task {} writes {}", task.id, path.display())
                    });
                }
                footprint.writes.insert(areas.for_path(&path));
            }
            for name in crates {
                footprint.writes.insert(areas.for_package_name(name));
            }
            for step in &task.verify {
                footprint.builds.extend(areas.built_by(&step.command));
            }
        }
        footprint.builds.extend(footprint.writes.iter().cloned());
        footprint.affects = areas.affected_by(&footprint.writes);
        footprint
    }
}

/// Plan → plans it must not run beside → why.
pub type PlanConflicts = BTreeMap<String, BTreeMap<String, String>>;

/// Conflicts between the plans of a set in the workspace at `workdir`,
/// reading its Cargo package graph once.
pub async fn plan_set_conflicts(workdir: &Path, plans: &[Plan]) -> PlanConflicts {
    let cargo = CargoWorkspace::load(workdir).await;
    let footprints = plans
        .iter()
        .map(|plan| {
            (
                plan.id.clone(),
                PlanFootprint::of(plan, workdir, cargo.as_ref()),
            )
        })
        .collect();
    plan_conflicts(&footprints)
}

/// Every pair of plans whose footprints overlap, with the reason.
#[must_use]
pub fn plan_conflicts(footprints: &BTreeMap<String, PlanFootprint>) -> PlanConflicts {
    let mut conflicts = PlanConflicts::new();
    for (left_id, left) in footprints {
        for (right_id, right) in footprints.range::<String, _>((Excluded(left_id), Unbounded)) {
            if let Some(reason) = conflict_reason(left_id, left, right_id, right) {
                conflicts
                    .entry(left_id.clone())
                    .or_default()
                    .insert(right_id.clone(), reason.clone());
                conflicts
                    .entry(right_id.clone())
                    .or_default()
                    .insert(left_id.clone(), reason);
            }
        }
    }
    conflicts
}

fn conflict_reason(
    left_id: &str,
    left: &PlanFootprint,
    right_id: &str,
    right: &PlanFootprint,
) -> Option<String> {
    for (plan_id, footprint) in [(left_id, left), (right_id, right)] {
        if let Some(why) = &footprint.exclusive {
            return Some(format!("{plan_id} runs alone: {why}"));
        }
    }
    if let Some(area) = first_overlap(&left.writes, &right.writes) {
        return Some(format!("both write {area}"));
    }
    if let Some(area) = first_overlap(&right.builds, &left.affects) {
        return Some(format!(
            "{right_id} builds {area}, which {left_id}'s edits reach"
        ));
    }
    if let Some(area) = first_overlap(&left.builds, &right.affects) {
        return Some(format!(
            "{left_id} builds {area}, which {right_id}'s edits reach"
        ));
    }
    None
}

fn first_overlap<'a>(areas: &'a BTreeSet<Area>, others: &BTreeSet<Area>) -> Option<&'a Area> {
    areas
        .iter()
        .find(|area| others.iter().any(|other| area.overlaps(other)))
}

/// Maps paths and commands of one workspace to [`Area`]s.
struct AreaResolver<'a> {
    workdir: &'a Path,
    cargo: Option<&'a CargoWorkspace>,
    /// Whether the workspace root holds a `Cargo.toml`.
    rust_workspace: bool,
}

impl AreaResolver<'_> {
    /// Area a workspace-relative path belongs to.
    fn for_path(&self, path: &Path) -> Area {
        if let Some(cargo) = self.cargo {
            if let Some((name, root)) = cargo.owner(path) {
                return Area::Package {
                    name: name.to_string(),
                    root: root.to_path_buf(),
                };
            }
        } else if self.rust_workspace && self.is_rust_source(path) {
            return Area::AllPackages;
        }
        if let Some(project) = self.enclosing_project(path) {
            return Area::Tree(project);
        }
        if let Some(prefix) = glob_prefix(path) {
            return Area::Tree(prefix);
        }
        if self.workdir.join(path).is_dir() {
            return Area::Tree(path.to_path_buf());
        }
        Area::File(path.to_path_buf())
    }

    /// Area a directory belongs to: its package or project, else itself.
    fn for_dir(&self, dir: &Path) -> Area {
        match self.for_path(dir) {
            Area::File(path) => Area::Tree(path),
            area => area,
        }
    }

    /// Area of a Cargo package named by `-p` or `crates_touched`.
    fn for_package_name(&self, name: &str) -> Area {
        match self.cargo {
            Some(cargo) => cargo.package(name).unwrap_or(Area::AllPackages),
            None if self.rust_workspace => Area::AllPackages,
            None => Area::Tree(Path::new("crates").join(name)),
        }
    }

    /// Without a package graph: whether `path` is Rust code or sits inside
    /// a Cargo package.
    fn is_rust_source(&self, path: &Path) -> bool {
        path.extension().is_some_and(|extension| extension == "rs")
            || path
                .ancestors()
                .filter(|dir| !dir.as_os_str().is_empty())
                .any(|dir| self.workdir.join(dir).join("Cargo.toml").is_file())
    }

    /// Nearest directory at or above `path`, below the workspace root, that
    /// holds a project manifest.
    fn enclosing_project(&self, path: &Path) -> Option<PathBuf> {
        path.ancestors()
            .filter(|dir| !dir.as_os_str().is_empty())
            .find(|dir| {
                PROJECT_MANIFESTS
                    .iter()
                    .any(|manifest| self.workdir.join(dir).join(manifest).is_file())
            })
            .map(Path::to_path_buf)
    }

    /// Areas a verify command builds or tests.
    ///
    /// Tracks `cd` across the command's `&&`/`;`/`|` segments. A cargo
    /// invocation builds the packages it names, every package with
    /// `--workspace` or at the workspace root, else the package of its
    /// directory. Any other command run after a `cd` builds that directory's
    /// project. Commands at the root that are not cargo read files only.
    fn built_by(&self, command: &str) -> Vec<Area> {
        let mut areas = Vec::new();
        let mut cwd = PathBuf::new();
        for segment in command.split(['&', '|', ';', '\n']) {
            let mut words = segment
                .split_whitespace()
                .map(|word| word.trim_start_matches(['(', '{']).trim_end_matches(')'))
                .filter(|word| !word.is_empty())
                .peekable();
            while let Some(word) = words.peek() {
                if *word == "timeout" {
                    words.next();
                    words.next_if(|word| word.starts_with(|c: char| c.is_ascii_digit()));
                } else if matches!(
                    *word,
                    "env" | "time" | "nice" | "exec" | "command" | "nohup"
                ) || (word.contains('=') && !word.starts_with('-'))
                {
                    words.next();
                } else {
                    break;
                }
            }
            match words.next() {
                Some("cd") => {
                    cwd = words
                        .next()
                        .map_or_else(PathBuf::new, |dir| join_dir(&cwd, dir))
                }
                Some("cargo") => areas.extend(self.cargo_areas(&cwd, words)),
                Some(_) if !cwd.as_os_str().is_empty() => areas.push(self.for_dir(&cwd)),
                _ => {}
            }
        }
        areas
    }

    /// Areas one `cargo` invocation builds, given its arguments.
    fn cargo_areas<'w>(&self, cwd: &Path, args: impl Iterator<Item = &'w str>) -> Vec<Area> {
        let mut args = args.filter(|arg| !arg.starts_with('+'));
        let Some(subcommand) = args.next() else {
            return Vec::new();
        };
        if NON_BUILDING_CARGO_COMMANDS.contains(&subcommand) {
            return Vec::new();
        }
        let mut named = Vec::new();
        while let Some(arg) = args.next() {
            match arg {
                "--" => break,
                "--workspace" | "--all" => named.push(Area::AllPackages),
                "-p" | "--package" => {
                    if let Some(name) = args.next() {
                        named.push(self.for_package_name(name));
                    }
                }
                "--manifest-path" => {
                    if let Some(manifest) = args.next() {
                        named.push(self.manifest_area(cwd, manifest));
                    }
                }
                _ => {
                    if let Some(name) = arg
                        .strip_prefix("--package=")
                        .or_else(|| arg.strip_prefix("-p").filter(|name| !name.is_empty()))
                    {
                        named.push(self.for_package_name(name));
                    } else if let Some(manifest) = arg.strip_prefix("--manifest-path=") {
                        named.push(self.manifest_area(cwd, manifest));
                    }
                }
            }
        }
        if !named.is_empty() {
            named
        } else if !cwd.as_os_str().is_empty() {
            vec![self.for_dir(cwd)]
        } else if self.rust_workspace {
            vec![Area::AllPackages]
        } else {
            Vec::new()
        }
    }

    fn manifest_area(&self, cwd: &Path, manifest: &str) -> Area {
        let manifest = join_dir(cwd, manifest);
        match manifest.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => self.for_dir(dir),
            _ if self.rust_workspace => Area::AllPackages,
            _ => Area::File(manifest.clone()),
        }
    }

    /// `writes` plus every workspace package depending on a written package.
    fn affected_by(&self, writes: &BTreeSet<Area>) -> BTreeSet<Area> {
        let mut affects = writes.clone();
        if let Some(cargo) = self.cargo {
            for area in writes {
                if let Area::Package { name, .. } = area {
                    affects.extend(
                        cargo
                            .dependents
                            .get(name)
                            .into_iter()
                            .flatten()
                            .filter_map(|dependent| cargo.package(dependent)),
                    );
                }
            }
        }
        affects
    }
}

/// `raw` as a path relative to the workspace root, with `.` and `..`
/// resolved lexically. Absolute paths outside the workspace are kept.
fn workspace_relative(workdir: &Path, raw: &str) -> PathBuf {
    let raw = Path::new(raw.trim().trim_matches(['"', '\'']));
    normalize(raw.strip_prefix(workdir).unwrap_or(raw))
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

/// `dir` (from a `cd` or `--manifest-path`) resolved against `cwd`, both
/// relative to the workspace root.
fn join_dir(cwd: &Path, dir: &str) -> PathBuf {
    let dir = dir.trim_matches(['"', '\'']);
    if dir == "-" || dir.starts_with('~') || dir.starts_with('$') {
        return PathBuf::new();
    }
    normalize(&cwd.join(dir))
}

/// The directory part of a glob pattern, when `path` is one.
fn glob_prefix(path: &Path) -> Option<PathBuf> {
    let is_glob = |component: &Component<'_>| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|text| text.contains(['*', '?', '[']))
    };
    path.components()
        .any(|component| is_glob(&component))
        .then(|| {
            path.components()
                .take_while(|component| !is_glob(component))
                .collect()
        })
}

/// Workspace-wide build inputs: editing one changes every package's build.
fn is_workspace_build_input(path: &Path) -> bool {
    matches!(
        path.to_str(),
        Some("Cargo.toml" | "Cargo.lock" | "rust-toolchain" | "rust-toolchain.toml")
    ) || path.starts_with(".cargo")
}

// ── Scheduling ───────────────────────────────────────────────────────────

/// How a plan of the set ended, or why it never ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanOutcome {
    /// Every task passed its verify steps.
    Succeeded,
    /// Every task ran and none failed, but some ran no verify step, so
    /// nothing shows their work is right. Not a success: the plans that
    /// depend on it are blocked. Not a failure: `fail_fast` still starts
    /// the plans that do not.
    Unverified,
    /// The plan ran and failed.
    Failed,
    /// Never started: a prerequisite did not succeed, or `fail_fast`.
    Blocked,
    /// An operator cancelled it.
    Cancelled,
    /// A stop request (SIGINT, SIGTERM, server cancel) ended it.
    Interrupted,
}

impl PlanOutcome {
    /// Whether the plan succeeded.
    #[must_use]
    pub const fn succeeded(self) -> bool {
        matches!(self, Self::Succeeded)
    }

    /// Lower-case label for summaries.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Unverified => "unverified",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }
}

/// Why a plan will never start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    /// These prerequisites did not succeed.
    Prerequisites(Vec<String>),
    /// An earlier plan failed and the run was asked to fail fast.
    FailFast,
}

/// One scheduling decision of [`PlanSetScheduler::admit`].
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Admission {
    /// Plans to start now, in execution order.
    pub start: Vec<String>,
    /// Plans that will never start, in execution order.
    pub blocked: Vec<(String, BlockReason)>,
    /// Plans newly held back by a conflict: the plan, the plan it waits
    /// for, and the reason. Each pair is reported once.
    pub waiting: Vec<(String, String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotState {
    Pending,
    Running,
    Done(PlanOutcome),
}

/// Decides when each plan of a set starts.
///
/// Plans are considered in execution order. One starts once every
/// prerequisite succeeded, fewer than `limit` plans are running, and it
/// conflicts neither with a running plan nor with an earlier plan that is
/// still pending, so conflicting plans keep their order. A plan whose
/// prerequisite did not succeed is blocked. Scanning stops as soon as every
/// slot is taken, so with `limit` 1 plans start, and blocked plans are
/// reported, exactly in execution order.
#[derive(Debug, Clone)]
pub struct PlanSetScheduler {
    order: Vec<String>,
    dependencies: BTreeMap<String, BTreeSet<String>>,
    conflicts: PlanConflicts,
    limit: usize,
    fail_fast: bool,
    states: BTreeMap<String, SlotState>,
    stopped: bool,
    failed: bool,
    announced: BTreeSet<(String, String)>,
}

impl PlanSetScheduler {
    /// Scheduler for `order`, running up to `limit` (at least 1) plans at a time.
    #[must_use]
    pub fn new(
        order: &PlanSetOrder,
        conflicts: PlanConflicts,
        limit: usize,
        fail_fast: bool,
    ) -> Self {
        Self {
            states: order
                .order
                .iter()
                .map(|plan_id| (plan_id.clone(), SlotState::Pending))
                .collect(),
            order: order.order.clone(),
            dependencies: order.dependencies.clone(),
            conflicts,
            limit: limit.max(1),
            fail_fast,
            stopped: false,
            failed: false,
            announced: BTreeSet::new(),
        }
    }

    /// Start, block or hold every plan that can be decided now.
    pub fn admit(&mut self) -> Admission {
        let mut admission = Admission::default();
        if self.stopped {
            return admission;
        }
        let mut running = self.running_count();
        for index in 0..self.order.len() {
            if running >= self.limit {
                break;
            }
            let plan_id = self.order[index].clone();
            if self.states.get(&plan_id) != Some(&SlotState::Pending) {
                continue;
            }
            let prerequisites = self.dependencies.get(&plan_id).cloned().unwrap_or_default();
            let unsuccessful = prerequisites
                .iter()
                .filter(|prerequisite| {
                    matches!(
                        self.states.get(*prerequisite),
                        Some(SlotState::Done(outcome)) if !outcome.succeeded()
                    )
                })
                .cloned()
                .collect::<Vec<_>>();
            if !unsuccessful.is_empty() {
                self.states
                    .insert(plan_id.clone(), SlotState::Done(PlanOutcome::Blocked));
                admission
                    .blocked
                    .push((plan_id, BlockReason::Prerequisites(unsuccessful)));
                continue;
            }
            if self.failed && self.fail_fast {
                self.states
                    .insert(plan_id.clone(), SlotState::Done(PlanOutcome::Blocked));
                admission.blocked.push((plan_id, BlockReason::FailFast));
                continue;
            }
            let prerequisites_done = prerequisites.iter().all(|prerequisite| {
                self.states.get(prerequisite) == Some(&SlotState::Done(PlanOutcome::Succeeded))
            });
            if !prerequisites_done {
                continue;
            }
            if let Some((other, reason)) = self.conflict_holding(&plan_id, index) {
                if self.announced.insert((plan_id.clone(), other.clone())) {
                    admission.waiting.push((plan_id, other, reason));
                }
                continue;
            }
            self.states.insert(plan_id.clone(), SlotState::Running);
            running += 1;
            admission.start.push(plan_id);
        }
        admission
    }

    /// A running plan, or an earlier pending one, that `plan_id` conflicts with.
    fn conflict_holding(&self, plan_id: &str, index: usize) -> Option<(String, String)> {
        let conflicts = self.conflicts.get(plan_id)?;
        self.order
            .iter()
            .enumerate()
            .find_map(|(other_index, other)| {
                let reason = conflicts.get(other)?;
                match self.states.get(other) {
                    Some(SlotState::Running) => Some((other.clone(), reason.clone())),
                    Some(SlotState::Pending) if other_index < index => {
                        Some((other.clone(), reason.clone()))
                    }
                    _ => None,
                }
            })
    }

    /// Record how a started plan ended.
    pub fn finish(&mut self, plan_id: &str, outcome: PlanOutcome) {
        self.states
            .insert(plan_id.to_string(), SlotState::Done(outcome));
        if outcome == PlanOutcome::Failed {
            self.failed = true;
        }
    }

    /// Cancel a plan that has not started. Returns whether it was pending.
    pub fn cancel_pending(&mut self, plan_id: &str) -> bool {
        if self.states.get(plan_id) != Some(&SlotState::Pending) {
            return false;
        }
        self.states
            .insert(plan_id.to_string(), SlotState::Done(PlanOutcome::Cancelled));
        true
    }

    /// Run `plan_id` again (gap-c002bb): a plan that failed or was cancelled
    /// goes back to pending, and so does each plan blocked behind it that
    /// nothing else blocks now. Returns whether `plan_id` runs again: not
    /// when it is pending, running or did not fail, nor once the run has
    /// stopped.
    pub fn retry(&mut self, plan_id: &str) -> bool {
        if self.stopped
            || !matches!(
                self.states.get(plan_id),
                Some(SlotState::Done(PlanOutcome::Failed | PlanOutcome::Cancelled))
            )
        {
            return false;
        }
        self.states.insert(plan_id.to_string(), SlotState::Pending);
        self.failed = self
            .states
            .values()
            .any(|state| *state == SlotState::Done(PlanOutcome::Failed));
        // Execution order puts prerequisites first, so one pass frees a chain
        // of blocked plans.
        for plan in &self.order {
            if self.states.get(plan) == Some(&SlotState::Done(PlanOutcome::Blocked))
                && !self.blocked_now(plan)
            {
                self.states.insert(plan.clone(), SlotState::Pending);
            }
        }
        true
    }

    /// Whether `plan_id` would be blocked if it were pending now: a
    /// prerequisite ended without succeeding, or a plan failed in a
    /// fail-fast run.
    fn blocked_now(&self, plan_id: &str) -> bool {
        let prerequisite_failed = self
            .dependencies
            .get(plan_id)
            .into_iter()
            .flatten()
            .any(|prerequisite| {
                matches!(
                    self.states.get(prerequisite),
                    Some(SlotState::Done(outcome)) if !outcome.succeeded()
                )
            });
        prerequisite_failed || (self.failed && self.fail_fast)
    }

    /// Start nothing more; pending plans stay unstarted.
    pub fn stop(&mut self) {
        self.stopped = true;
    }

    /// Whether `plan_id` is running.
    #[must_use]
    pub fn is_running(&self, plan_id: &str) -> bool {
        self.states.get(plan_id) == Some(&SlotState::Running)
    }

    /// Whether nothing is running and nothing more will start.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        self.running_count() == 0
            && (self.stopped
                || !self
                    .states
                    .values()
                    .any(|state| *state == SlotState::Pending))
    }

    /// How `plan_id` ended, once it has.
    #[must_use]
    pub fn outcome(&self, plan_id: &str) -> Option<PlanOutcome> {
        match self.states.get(plan_id) {
            Some(SlotState::Done(outcome)) => Some(*outcome),
            _ => None,
        }
    }

    fn running_count(&self) -> usize {
        self.states
            .values()
            .filter(|state| **state == SlotState::Running)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(id: &str, depends_on_plan: &[&str], files: &[&str], verify: &[&str]) -> Plan {
        let quoted = |items: &[&str]| {
            items
                .iter()
                .map(|item| format!("{item:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let verify_steps = verify
            .iter()
            .map(|command| format!("\n[[task.verify]]\nphase = \"test\"\ncommand = {command:?}\n"))
            .collect::<String>();
        let tasks = crate::task_parser::TasksFile::parse_str(&format!(
            "[meta]\nplan = \"{id}\"\n\n[[task]]\nid = \"T1\"\ntitle = \"Task\"\n\
             depends_on_plan = [{}]\nfiles = [{}]\n{verify_steps}",
            quoted(depends_on_plan),
            quoted(files),
        ))
        .expect("parse test plan");
        Plan {
            id: id.to_string(),
            dir: PathBuf::from(id),
            tasks,
            prd_excerpt: String::new(),
        }
    }

    fn order(plans: &[Plan]) -> PlanSetOrder {
        order_plan_set(plans, |_| OutsidePlanStatus::Missing).expect("valid plan set")
    }

    // ── Order and prerequisites ──────────────────────────────────────

    #[test]
    fn order_puts_prerequisites_first_then_ids() {
        let plans = [
            plan("a-consumer", &["z-foundation"], &["a"], &[]),
            plan("m-independent", &[], &["m"], &[]),
            plan("z-foundation", &[], &["z"], &[]),
        ];
        assert_eq!(
            order(&plans).order,
            ["m-independent", "z-foundation", "a-consumer"]
        );
    }

    #[test]
    fn order_rejects_duplicates_self_dependencies_and_cycles() {
        let duplicate = [plan("dup", &[], &["a"], &[]), plan("dup", &[], &["b"], &[])];
        let error = order_plan_set(&duplicate, |_| OutsidePlanStatus::Missing)
            .expect_err("duplicate plan ID");
        assert!(error.to_string().contains("duplicate plan ID 'dup'"));

        let selfish = [plan("self-dependent", &["self-dependent"], &["a"], &[])];
        let error =
            order_plan_set(&selfish, |_| OutsidePlanStatus::Missing).expect_err("self dependency");
        assert!(
            error
                .to_string()
                .contains("plan 'self-dependent' cannot depend on itself")
        );

        let cycle = [
            plan("plan-a", &["plan-b"], &["a"], &[]),
            plan("plan-b", &["plan-a"], &["b"], &[]),
        ];
        let error =
            order_plan_set(&cycle, |_| OutsidePlanStatus::Missing).expect_err("dependency cycle");
        assert!(error.to_string().contains("dependency cycle"));
        assert!(error.to_string().contains("plan-a, plan-b"));
    }

    #[test]
    fn prerequisite_outside_the_set_is_satisfied_when_complete_on_disk() {
        let plans = [plan("consumer", &["foundation"], &["c"], &[])];
        let order = order_plan_set(&plans, |plan_id| {
            assert_eq!(plan_id, "foundation");
            OutsidePlanStatus::Complete("its Graph checkpoint succeeded".to_string())
        })
        .expect("a completed outside prerequisite is satisfied");
        assert_eq!(order.order, ["consumer"]);
        assert!(order.dependencies["consumer"].is_empty());
        assert_eq!(
            order.satisfied_outside["foundation"],
            "its Graph checkpoint succeeded"
        );
    }

    #[test]
    fn incomplete_or_unknown_outside_prerequisite_is_rejected_by_name() {
        let plans = [plan("consumer", &["foundation"], &["c"], &[])];
        let error = order_plan_set(&plans, |_| {
            OutsidePlanStatus::Incomplete("checkpoint failed".to_string())
        })
        .expect_err("an incomplete outside prerequisite fails the set");
        let message = error.to_string();
        assert!(message.contains("'consumer' depends on plan 'foundation'"));
        assert!(message.contains("status: checkpoint failed"));

        let error = order_plan_set(&plans, |_| OutsidePlanStatus::Missing)
            .expect_err("an unknown prerequisite fails the set");
        assert!(error.to_string().contains("unknown plan 'foundation'"));
    }

    fn write_plan(dir: &Path, id: &str, task_status: &str, depends_on_plan: &str) {
        let plan_dir = dir.join(id);
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            format!(
                "[meta]\nplan = \"{id}\"\n\n[[task]]\nid = \"T1\"\ntitle = \"Task\"\n\
                 status = \"{task_status}\"\nfiles = [\"{id}.txt\"]\n\
                 depends_on_plan = [{depends_on_plan}]\n\n\
                 [[task.verify]]\nphase = \"structural\"\ncommand = \"true\"\n"
            ),
        )
        .expect("tasks.toml");
    }

    fn write_checkpoint(workdir: &Path, id: &str, status: &str) {
        let dir = workdir.join(".roko/state/graph").join(id);
        std::fs::create_dir_all(&dir).expect("checkpoint dir");
        std::fs::write(
            dir.join("checkpoint.json"),
            format!("{{\"status\": \"{status}\"}}"),
        )
        .expect("checkpoint");
    }

    #[test]
    fn outside_status_reads_checkpoints_then_task_status() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let workdir = workspace.path();
        let plans_dir = workdir.join("plans");
        write_plan(&plans_dir, "consumer", "ready", "");
        write_plan(&plans_dir, "checkpointed", "ready", "");
        write_plan(&plans_dir, "hand-marked", "done", "");
        write_plan(&plans_dir, "unfinished", "ready", "");
        write_plan(&plans_dir, "crashed", "ready", "");
        write_checkpoint(workdir, "checkpointed", "succeeded");
        write_checkpoint(workdir, "crashed", "interrupted");
        let selected = plans_dir.join("consumer");

        let status = |plan_id: &str| outside_plan_status(workdir, &selected, plan_id);
        assert!(matches!(
            status("checkpointed"),
            OutsidePlanStatus::Complete(_)
        ));
        assert_eq!(
            status("hand-marked"),
            OutsidePlanStatus::Complete("all 1 tasks are done".to_string())
        );
        assert_eq!(
            status("unfinished"),
            OutsidePlanStatus::Incomplete("pending".to_string())
        );
        assert_eq!(
            status("crashed"),
            OutsidePlanStatus::Incomplete("checkpoint interrupted".to_string())
        );
        assert_eq!(status("nowhere"), OutsidePlanStatus::Missing);
    }

    #[test]
    fn single_plan_run_resolves_a_completed_sibling_prerequisite() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let workdir = workspace.path();
        let set = workdir.join("plans").join("programme");
        write_plan(&set, "01-foundation", "ready", "");
        write_plan(&set, "02-consumer", "ready", "\"01-foundation\"");
        let selected = set.join("02-consumer");
        let plans = crate::runner::plan_loader::load_plans(&selected).expect("load consumer");

        let error = plan_set_order(workdir, &selected, &plans)
            .expect_err("an unfinished sibling prerequisite fails the run");
        assert!(
            error
                .to_string()
                .contains("depends on plan '01-foundation'")
        );
        assert!(error.to_string().contains("status: pending"));

        write_checkpoint(workdir, "01-foundation", "succeeded");
        let order = plan_set_order(workdir, &selected, &plans)
            .expect("a succeeded sibling prerequisite satisfies the run");
        assert_eq!(order.order, ["02-consumer"]);
        assert!(order.satisfied_outside.contains_key("01-foundation"));
    }

    // ── Footprints ───────────────────────────────────────────────────

    /// `core` ← `serve` ← `cli` (arrows point at dependents), plus an
    /// unrelated `mcp`.
    fn cargo() -> CargoWorkspace {
        let roots = [
            ("core", "crates/core"),
            ("serve", "crates/serve"),
            ("cli", "crates/cli"),
            ("mcp", "crates/mcp"),
        ];
        CargoWorkspace {
            roots: roots
                .iter()
                .map(|(name, root)| ((*name).to_string(), PathBuf::from(root)))
                .collect(),
            dependents: [
                ("core", &["serve", "cli"][..]),
                ("serve", &["cli"][..]),
                ("cli", &[][..]),
                ("mcp", &[][..]),
            ]
            .iter()
            .map(|(name, dependents)| {
                (
                    (*name).to_string(),
                    dependents.iter().map(|d| (*d).to_string()).collect(),
                )
            })
            .collect(),
        }
    }

    fn conflicts_of(
        workdir: &Path,
        plans: &[Plan],
        cargo: Option<&CargoWorkspace>,
    ) -> PlanConflicts {
        let footprints = plans
            .iter()
            .map(|plan| (plan.id.clone(), PlanFootprint::of(plan, workdir, cargo)))
            .collect();
        plan_conflicts(&footprints)
    }

    /// gap-60233f: without `[meta] verify`, a plan in a Cargo workspace checks
    /// formatting, lints and tests over the crates it affects (those it
    /// writes and their dependents), never the whole workspace's tests. An
    /// authored `[meta] verify` replaces the default.
    #[test]
    fn default_meta_verify_covers_the_touched_crates() {
        use super::super::plan_verify::plan_verify_steps;

        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::write(workspace.path().join("Cargo.toml"), "[workspace]\n").expect("manifest");
        let cargo = cargo();
        let commands = |plan: &Plan, cargo: Option<&CargoWorkspace>| {
            let footprint = PlanFootprint::of(plan, workspace.path(), cargo);
            plan_verify_steps(plan, Some(&footprint))
                .into_iter()
                .map(|step| step.command)
                .collect::<Vec<_>>()
        };
        let core_change = plan("core-change", &[], &["crates/core/src/lib.rs"], &[]);

        assert_eq!(
            commands(&core_change, Some(&cargo)),
            [
                "cargo fmt -p cli -p core -p serve -- --check",
                "cargo clippy -p cli -p core -p serve --no-deps -- -D warnings",
                "cargo test -p cli -p core -p serve",
            ]
        );
        // Without the package graph: formatting and lints over the
        // workspace, and no tests.
        assert_eq!(
            commands(&core_change, None),
            [
                "cargo fmt --all -- --check",
                "cargo clippy --workspace --no-deps -- -D warnings",
            ]
        );
        // A plan that writes no crate gets no default.
        let docs_change = plan("docs-change", &[], &["docs/guide.md"], &[]);
        assert!(commands(&docs_change, Some(&cargo)).is_empty());
        // An authored `[meta] verify` wins.
        let authored = Plan {
            tasks: crate::task_parser::TasksFile::parse_str(
                "[meta]\nplan = \"authored\"\n\n[[meta.verify]]\ncommand = \"make check\"\n\n\
                 [[task]]\nid = \"T1\"\ntitle = \"Task\"\nfiles = [\"crates/core/src/lib.rs\"]\n",
            )
            .expect("parse"),
            ..plan("authored", &[], &[], &[])
        };
        assert_eq!(commands(&authored, Some(&cargo)), ["make check"]);
    }

    #[test]
    fn upstream_writer_conflicts_with_downstream_builder_but_not_siblings() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let cargo = cargo();
        let plans = [
            plan(
                "core-change",
                &[],
                &["crates/core/src/lib.rs"],
                &["cargo test -p core"],
            ),
            plan(
                "cli-change",
                &[],
                &["crates/cli/src/main.rs"],
                &["cargo check -p cli"],
            ),
            plan(
                "mcp-change",
                &[],
                &["crates/mcp/src/lib.rs"],
                &["cargo test -p mcp"],
            ),
        ];

        let conflicts = conflicts_of(workspace.path(), &plans, Some(&cargo));

        assert_eq!(
            conflicts["core-change"]["cli-change"],
            "cli-change builds cli, which core-change's edits reach"
        );
        assert!(!conflicts.contains_key("mcp-change"));
    }

    #[test]
    fn same_package_and_workspace_builds_conflict() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let cargo = cargo();
        let plans = [
            plan("a", &[], &["crates/mcp/src/a.rs"], &[]),
            plan("b", &[], &["crates/mcp/src/b.rs"], &[]),
            plan(
                "c",
                &[],
                &["docs/c.md"],
                &["cargo clippy --workspace -- -D warnings"],
            ),
            plan("d", &[], &["docs/d.md"], &["grep -q x docs/d.md"]),
        ];

        let conflicts = conflicts_of(workspace.path(), &plans, Some(&cargo));

        assert_eq!(conflicts["a"]["b"], "both write mcp");
        assert_eq!(
            conflicts["a"]["c"],
            "c builds every Rust package, which a's edits reach"
        );
        assert!(
            !conflicts.contains_key("d"),
            "docs-only plans run beside anything"
        );
    }

    #[test]
    fn project_directories_are_one_area() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let app = workspace.path().join("apps/portal");
        std::fs::create_dir_all(&app).expect("app dir");
        std::fs::write(app.join("package.json"), "{}").expect("package.json");
        let cargo = cargo();
        let plans = [
            plan("shell", &[], &["apps/portal/src/shell.tsx"], &[]),
            plan("theme", &[], &["apps/portal/src/theme.css"], &[]),
            plan(
                "tests",
                &[],
                &["docs/notes.md"],
                &["cd apps/portal && npm test"],
            ),
            plan(
                "backend",
                &[],
                &["crates/serve/src/lib.rs"],
                &["cargo test -p serve"],
            ),
        ];

        let conflicts = conflicts_of(workspace.path(), &plans, Some(&cargo));

        assert_eq!(conflicts["shell"]["theme"], "both write apps/portal");
        assert!(conflicts["tests"].contains_key("shell"));
        assert!(
            !conflicts.contains_key("backend"),
            "the portal and the Rust backend run side by side"
        );
    }

    #[test]
    fn undeclared_files_and_workspace_manifests_run_alone() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let cargo = cargo();
        let plans = [
            plan("vague", &[], &[], &[]),
            plan("lockfile", &[], &["Cargo.lock"], &[]),
            plan("docs", &[], &["docs/x.md"], &[]),
        ];

        let conflicts = conflicts_of(workspace.path(), &plans, Some(&cargo));

        assert_eq!(
            conflicts["docs"]["vague"],
            "vague runs alone: its task T1 declares no files"
        );
        assert_eq!(
            conflicts["docs"]["lockfile"],
            "lockfile runs alone: its task T1 writes Cargo.lock"
        );
    }

    #[test]
    fn without_a_package_graph_rust_plans_conflict_and_others_do_not() {
        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::write(workspace.path().join("Cargo.toml"), "[workspace]\n").expect("Cargo.toml");
        let plans = [
            plan("a", &[], &["crates/a/src/lib.rs"], &[]),
            plan("b", &[], &["crates/b/src/lib.rs"], &[]),
            plan("docs", &[], &["docs/x.md"], &[]),
        ];

        let conflicts = conflicts_of(workspace.path(), &plans, None);

        assert_eq!(conflicts["a"]["b"], "both write every Rust package");
        assert!(!conflicts.contains_key("docs"));
    }

    #[test]
    fn verify_command_parsing_follows_cd_and_cargo_flags() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let cargo = cargo();
        let resolver = AreaResolver {
            workdir: workspace.path(),
            cargo: Some(&cargo),
            rust_workspace: true,
        };
        let serve = cargo.package("serve").expect("serve");
        let cli = cargo.package("cli").expect("cli");

        assert_eq!(
            resolver.built_by("cargo test -p serve --lib plans 2>&1 | tail -5"),
            [serve.clone()]
        );
        assert_eq!(
            resolver.built_by("cargo check --package=serve && cargo +nightly build -pcli"),
            [serve.clone(), cli]
        );
        assert_eq!(
            resolver.built_by("cd crates/serve && cargo test"),
            [serve.clone()]
        );
        assert_eq!(
            resolver.built_by("tmp=$(mktemp); cargo test > \"$tmp\""),
            [Area::AllPackages]
        );
        assert_eq!(resolver.built_by("cargo metadata --no-deps"), []);
        assert_eq!(
            resolver.built_by("grep -q 'cargo test --workspace' justfile"),
            []
        );
        assert_eq!(
            resolver.built_by("cd demo/app && npm test"),
            [Area::Tree(PathBuf::from("demo/app"))]
        );
    }

    // ── Scheduling ───────────────────────────────────────────────────

    fn scheduler(plans: &[Plan], conflicts: PlanConflicts, limit: usize) -> PlanSetScheduler {
        PlanSetScheduler::new(&order(plans), conflicts, limit, false)
    }

    fn conflict(pairs: &[(&str, &str)]) -> PlanConflicts {
        let mut conflicts = PlanConflicts::new();
        for (left, right) in pairs {
            conflicts
                .entry((*left).to_string())
                .or_default()
                .insert((*right).to_string(), "shared".to_string());
            conflicts
                .entry((*right).to_string())
                .or_default()
                .insert((*left).to_string(), "shared".to_string());
        }
        conflicts
    }

    /// Drive `scheduler` to completion, finishing each started plan with
    /// `outcome_of`, and return the start order.
    fn run_sequence(
        scheduler: &mut PlanSetScheduler,
        outcome_of: impl Fn(&str) -> PlanOutcome,
    ) -> Vec<String> {
        let mut started = Vec::new();
        loop {
            let admission = scheduler.admit();
            started.extend(admission.start.iter().cloned());
            let Some(first) = admission.start.first().cloned().or_else(|| {
                started
                    .iter()
                    .find(|plan_id| scheduler.is_running(plan_id))
                    .cloned()
            }) else {
                break;
            };
            scheduler.finish(&first, outcome_of(&first));
        }
        started
    }

    #[test]
    fn one_slot_starts_plans_in_execution_order() {
        let plans = [
            plan("c", &["a"], &["c"], &[]),
            plan("b", &[], &["b"], &[]),
            plan("a", &[], &["a"], &[]),
        ];
        let mut scheduler = scheduler(&plans, PlanConflicts::new(), 1);

        let started = run_sequence(&mut scheduler, |_| PlanOutcome::Succeeded);

        assert_eq!(started, order(&plans).order);
        assert_eq!(started, ["a", "b", "c"]);
        assert!(scheduler.is_settled());
    }

    #[test]
    fn one_slot_reports_blocked_plans_where_the_sequential_loop_did() {
        let plans = [
            plan("a", &[], &["a"], &[]),
            plan("b", &[], &["b"], &[]),
            plan("c", &["a"], &["c"], &[]),
        ];
        let mut scheduler = scheduler(&plans, PlanConflicts::new(), 1);

        assert_eq!(scheduler.admit().start, ["a"]);
        scheduler.finish("a", PlanOutcome::Failed);
        let admission = scheduler.admit();
        assert_eq!(admission.start, ["b"]);
        assert!(
            admission.blocked.is_empty(),
            "c is reported after b, as before"
        );
        scheduler.finish("b", PlanOutcome::Succeeded);
        let admission = scheduler.admit();
        assert_eq!(
            admission.blocked,
            [(
                "c".to_string(),
                BlockReason::Prerequisites(vec!["a".to_string()])
            )]
        );
        assert!(scheduler.is_settled());
    }

    #[test]
    fn independent_plans_share_the_slots() {
        let plans = [
            plan("a", &[], &["a"], &[]),
            plan("b", &[], &["b"], &[]),
            plan("c", &[], &["c"], &[]),
        ];
        let mut scheduler = scheduler(&plans, PlanConflicts::new(), 2);

        assert_eq!(scheduler.admit().start, ["a", "b"]);
        assert!(scheduler.admit().start.is_empty(), "both slots are taken");
        scheduler.finish("b", PlanOutcome::Succeeded);
        assert_eq!(scheduler.admit().start, ["c"]);
    }

    /// gap-7c9e48: in the diamond A → {B, C} → D the middle plans start
    /// together as soon as A succeeds, and D starts once both have.
    #[test]
    fn diamond_plan_set_runs_middle_plans_together() {
        let plans = [
            plan("a", &[], &["a"], &[]),
            plan("b", &["a"], &["b"], &[]),
            plan("c", &["a"], &["c"], &[]),
            plan("d", &["b", "c"], &["d"], &[]),
        ];
        let mut scheduler = scheduler(&plans, PlanConflicts::new(), 4);

        assert_eq!(scheduler.admit().start, ["a"]);
        assert!(scheduler.admit().start.is_empty(), "b and c wait for a");
        scheduler.finish("a", PlanOutcome::Succeeded);
        assert_eq!(scheduler.admit().start, ["b", "c"]);
        scheduler.finish("c", PlanOutcome::Succeeded);
        assert!(scheduler.admit().start.is_empty(), "d waits for b too");
        scheduler.finish("b", PlanOutcome::Succeeded);
        assert_eq!(scheduler.admit().start, ["d"]);
        scheduler.finish("d", PlanOutcome::Succeeded);
        assert!(scheduler.is_settled());
    }

    #[test]
    fn dependents_wait_for_success_and_later_plans_do_not() {
        let plans = [
            plan("a", &[], &["a"], &[]),
            plan("b", &["a"], &["b"], &[]),
            plan("c", &[], &["c"], &[]),
            plan("d", &["b"], &["d"], &[]),
        ];
        let mut scheduler = scheduler(&plans, PlanConflicts::new(), 3);

        assert_eq!(scheduler.admit().start, ["a", "c"]);
        scheduler.finish("a", PlanOutcome::Failed);
        let admission = scheduler.admit();
        assert!(admission.start.is_empty());
        assert_eq!(
            admission.blocked,
            [
                (
                    "b".to_string(),
                    BlockReason::Prerequisites(vec!["a".to_string()])
                ),
                (
                    "d".to_string(),
                    BlockReason::Prerequisites(vec!["b".to_string()])
                ),
            ]
        );
        assert_eq!(scheduler.outcome("d"), Some(PlanOutcome::Blocked));
    }

    #[test]
    fn conflicting_plans_never_overlap_and_keep_their_order() {
        let plans = [
            plan("a", &[], &["a"], &[]),
            plan("b", &[], &["b"], &[]),
            plan("c", &[], &["c"], &[]),
        ];
        let mut scheduler = scheduler(&plans, conflict(&[("a", "c")]), 3);

        let admission = scheduler.admit();
        assert_eq!(admission.start, ["a", "b"]);
        assert_eq!(
            admission.waiting,
            [("c".to_string(), "a".to_string(), "shared".to_string())]
        );
        assert!(
            scheduler.admit().waiting.is_empty(),
            "each wait is announced once"
        );
        scheduler.finish("a", PlanOutcome::Failed);
        assert_eq!(
            scheduler.admit().start,
            ["c"],
            "a conflict is not a dependency"
        );
    }

    #[test]
    fn a_later_plan_does_not_overtake_an_earlier_conflicting_one() {
        let plans = [
            plan("a", &[], &["a"], &[]),
            plan("b", &["a"], &["b"], &[]),
            plan("c", &[], &["c"], &[]),
        ];
        let mut scheduler = scheduler(&plans, conflict(&[("b", "c")]), 3);

        let admission = scheduler.admit();
        assert_eq!(
            admission.start,
            ["a"],
            "c waits behind b, which waits for a"
        );
        assert_eq!(admission.waiting[0].1, "b");
        scheduler.finish("a", PlanOutcome::Succeeded);
        assert_eq!(scheduler.admit().start, ["b"]);
        scheduler.finish("b", PlanOutcome::Succeeded);
        assert_eq!(scheduler.admit().start, ["c"]);
    }

    #[test]
    fn fail_fast_blocks_everything_after_a_failure() {
        let plans = [
            plan("a", &[], &["a"], &[]),
            plan("b", &[], &["b"], &[]),
            plan("c", &[], &["c"], &[]),
        ];
        let mut scheduler = PlanSetScheduler::new(&order(&plans), PlanConflicts::new(), 2, true);

        assert_eq!(scheduler.admit().start, ["a", "b"]);
        scheduler.finish("a", PlanOutcome::Failed);
        let admission = scheduler.admit();
        assert!(admission.start.is_empty());
        assert_eq!(
            admission.blocked,
            [("c".to_string(), BlockReason::FailFast)]
        );
        scheduler.finish("b", PlanOutcome::Succeeded);
        assert!(scheduler.is_settled());
    }

    /// gap-29a84b: an unverified plan is no prerequisite, but it does not
    /// stop a fail-fast run either.
    #[test]
    fn an_unverified_plan_blocks_its_dependants_but_is_no_failure() {
        let plans = [
            plan("a", &[], &["a"], &[]),
            plan("b", &["a"], &["b"], &[]),
            plan("c", &[], &["c"], &[]),
        ];
        let mut scheduler = PlanSetScheduler::new(&order(&plans), PlanConflicts::new(), 1, true);

        assert_eq!(scheduler.admit().start, ["a"]);
        scheduler.finish("a", PlanOutcome::Unverified);
        let admission = scheduler.admit();
        assert_eq!(
            admission.blocked,
            [(
                "b".to_string(),
                BlockReason::Prerequisites(vec!["a".to_string()])
            )]
        );
        assert_eq!(admission.start, ["c"]);
    }

    /// gap-c002bb: a plan that failed runs again on the operator's retry,
    /// and the plan it blocked waits for it again. A plan that is running,
    /// waiting or succeeded is not retried, nor anything once the run stops.
    #[test]
    fn retry_runs_a_failed_plan_again_and_frees_what_it_blocked() {
        let plans = [
            plan("a", &[], &["a"], &[]),
            plan("b", &["a"], &["b"], &[]),
            plan("c", &[], &["c"], &[]),
        ];
        let mut scheduler = scheduler(&plans, PlanConflicts::new(), 2);

        assert_eq!(scheduler.admit().start, ["a", "c"]);
        assert!(!scheduler.retry("a"), "a is running");
        scheduler.finish("a", PlanOutcome::Failed);
        assert_eq!(
            scheduler.admit().blocked,
            [(
                "b".to_string(),
                BlockReason::Prerequisites(vec!["a".to_string()])
            )]
        );

        assert!(scheduler.retry("a"));
        assert!(!scheduler.retry("b"), "b waits for a again");
        assert_eq!(scheduler.admit().start, ["a"]);
        scheduler.finish("a", PlanOutcome::Succeeded);
        assert_eq!(scheduler.admit().start, ["b"]);
        assert!(!scheduler.retry("a"), "a succeeded");

        scheduler.stop();
        scheduler.finish("c", PlanOutcome::Failed);
        assert!(!scheduler.retry("c"), "the run has stopped");
    }

    #[test]
    fn stop_and_cancel_leave_plans_unstarted() {
        let plans = [
            plan("a", &[], &["a"], &[]),
            plan("b", &["a"], &["b"], &[]),
            plan("c", &[], &["c"], &[]),
        ];
        let mut scheduler = scheduler(&plans, PlanConflicts::new(), 1);

        assert_eq!(scheduler.admit().start, ["a"]);
        assert!(scheduler.cancel_pending("b"));
        assert!(!scheduler.cancel_pending("a"), "a is running, not pending");
        scheduler.stop();
        scheduler.finish("a", PlanOutcome::Interrupted);
        assert_eq!(scheduler.admit(), Admission::default());
        assert!(scheduler.is_settled());
        assert_eq!(scheduler.outcome("b"), Some(PlanOutcome::Cancelled));
        assert_eq!(scheduler.outcome("c"), None, "c never started");
    }
}
