//! One content-blind refresh shared by command and interactive global views.
use crate::{
    discovery,
    domain::{Checkout, GitState, Lifecycle, Workspace},
    navigation::tmux,
};
use clap::{Args, ValueEnum};
use std::{
    collections::{BTreeMap, HashMap},
    io,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum State {
    #[default]
    All,
    Attention,
    Failed,
    Input,
    Review,
    Working,
    Running,
    Starting,
    Unknown,
    Exited,
}
impl State {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Attention => "attention",
            Self::Failed => "failed",
            Self::Input => "input",
            Self::Review => "review",
            Self::Working => "working",
            Self::Running => "running",
            Self::Starting => "starting",
            Self::Unknown => "unknown",
            Self::Exited => "exited",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Attention,
            Self::Attention => Self::Failed,
            Self::Failed => Self::Input,
            Self::Input => Self::Review,
            Self::Review => Self::Working,
            Self::Working => Self::Running,
            Self::Running => Self::Starting,
            Self::Starting => Self::Unknown,
            Self::Unknown => Self::Exited,
            Self::Exited => Self::All,
        }
    }
    fn matches(self, w: &Workspace) -> bool {
        match self {
            Self::All => true,
            Self::Attention => needs_attention(w),
            Self::Exited => w.process == "exited",
            Self::Failed => has_failure(w),
            Self::Input => w.lifecycle == Lifecycle::Waiting,
            Self::Review => w.lifecycle == Lifecycle::Review,
            Self::Working => w.lifecycle == Lifecycle::Working,
            Self::Running => w.lifecycle == Lifecycle::Running,
            Self::Starting => w.lifecycle == Lifecycle::Starting,
            Self::Unknown => w.lifecycle == Lifecycle::Unknown,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Group {
    #[default]
    Project,
    Attention,
}
#[derive(Clone, Debug, Default, Args)]
pub struct Query {
    /// Project session ID/name or repository path; "unassociated" selects unknown projects.
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long, value_enum, default_value_t=State::All)]
    pub state: State,
    /// Search names, refs, sessions, projects and agent kinds.
    #[arg(long, default_value = "")]
    pub search: String,
    #[arg(long, value_enum, default_value_t=Group::Project)]
    pub group: Group,
    /// Exact tmux session ID: include only its window membership (also linked views).
    #[arg(long, conflicts_with = "project")]
    pub local_session: Option<String>,
    /// Include all project windows, including shells and coordinators.
    #[arg(long)]
    pub windows: bool,
}
pub fn is_worker(workspace: &Workspace) -> bool {
    workspace.is_agent()
        && workspace.coordinator.as_deref() != Some(workspace.identity.window_id.as_str())
}

/// Failure is independent of a retained hook handoff. These pure predicates are
/// shared by filters, grouping and counts without rewriting lifecycle history.
pub fn has_failure(workspace: &Workspace) -> bool {
    workspace.lifecycle == Lifecycle::Failed
        || (workspace.process == "exited"
            && (workspace.exit_code.is_some_and(|code| code != 0)
                || workspace.exit_signal.is_some()))
}
pub fn needs_attention(workspace: &Workspace) -> bool {
    has_failure(workspace) || workspace.lifecycle.needs_attention()
}
/// Each row belongs to one attention group; a known failure takes precedence.
pub fn attention_state(workspace: &Workspace) -> Lifecycle {
    if has_failure(workspace) {
        Lifecycle::Failed
    } else {
        workspace.lifecycle
    }
}

/// Display grouping is independent of coordinator/batch association. An ordinary
/// agent can have a known repository without granting it any managed-session role.
pub fn project_key(workspaces: &[Workspace], w: &Workspace) -> String {
    if let Some(project) = &w.project {
        return project.clone();
    }
    if let Some(repo) = &w.checkout.repository {
        if let Some(project) = workspaces.iter().find_map(|other| {
            (other.checkout.repository.as_ref() == Some(repo))
                .then_some(other.project.as_ref())
                .flatten()
        }) {
            return project.clone();
        }
        return format!("repo:{}", repo.display());
    }
    "unassociated".into()
}

pub fn project_group_label(key: &str) -> &str {
    key.strip_prefix("repo:").unwrap_or(key)
}

#[derive(Default, Clone)]
pub struct Totals {
    pub workers: usize,
    pub live: usize,
    pub exited: usize,
    pub attention: usize,
    pub failed: usize,
    pub input: usize,
    pub review: usize,
    pub projects: usize,
}
impl Totals {
    pub fn from_workspaces(workspaces: &[Workspace]) -> Self {
        let mut totals = Self::default();
        let mut projects = std::collections::HashSet::new();
        for w in workspaces {
            let key = project_key(workspaces, w);
            if key != "unassociated" {
                projects.insert(key);
            }
            if !is_worker(w) {
                continue;
            }
            totals.workers += 1;
            totals.live += usize::from(w.process == "running");
            totals.exited += usize::from(w.process == "exited");
            totals.attention += usize::from(needs_attention(w));
            totals.failed += usize::from(has_failure(w));
            totals.input += usize::from(w.lifecycle == Lifecycle::Waiting);
            totals.review += usize::from(w.lifecycle == Lifecycle::Review);
        }
        totals.projects = projects.len();
        totals
    }
    pub fn categories_overlap(&self) -> bool {
        self.failed + self.input + self.review > self.attention
    }
    pub fn summary(&self) -> String {
        format!(
            "GLOBAL {} workers · {} live · {} project{} · {} exited · {} attention ({} failed / {} input / {} review{})",
            self.workers,
            self.live,
            self.projects,
            if self.projects == 1 { "" } else { "s" },
            self.exited,
            self.attention,
            self.failed,
            self.input,
            self.review,
            if self.categories_overlap() {
                "; categories overlap"
            } else {
                ""
            }
        )
    }
}
#[derive(Clone, Default)]
pub struct Detail {
    pub integration: Option<String>,
    pub verification: Option<String>,
    pub batch: Option<crate::batch::Batch>,
    pub task_reference: Option<String>,
    pub source_commit: Option<String>,
    pub delivery: String,
    pub commit: Option<String>,
    pub changed_files: Vec<String>,
    pub warning: Option<String>,
}

pub struct Snapshot {
    pub workspaces: Vec<Workspace>,
    pub projects: BTreeMap<String, String>,
    pub elapsed_ms: u128,
    pub details: HashMap<String, Detail>,
    pub checkouts: usize,
    pub memberships: HashMap<String, std::collections::HashSet<String>>,
}
impl Snapshot {
    pub fn capture() -> io::Result<Self> {
        let start = Instant::now();
        let mut workspaces = discovery::discover_all_tmux().map_err(io::Error::other)?;
        let sessions = tmux(&[
            "list-sessions",
            "-F",
            "#{session_id}␟#{session_name}␟#{@drudwyn_view_of}␟#{@drudwyn_coordinator}␟#{@drudwyn_project_repo}",
        ])?;
        let mut projects = BTreeMap::new();
        let mut names = HashMap::new();
        let mut coords = HashMap::new();
        for row in sessions.lines() {
            let f: Vec<_> = row.split('␟').collect();
            if f.len() != 5 {
                return Err(io::Error::other("Invalid project inventory"));
            }
            names.insert(
                f[1].to_owned(),
                if f[2].is_empty() { f[0] } else { f[2] }.to_owned(),
            );
            if f[2].is_empty() {
                projects.insert(f[0].to_owned(), f[1].to_owned());
            }
            if !f[3].is_empty() || !f[4].is_empty() {
                coords.insert(f[0].to_owned(), f[3].to_owned());
            }
        }
        for w in &mut workspaces {
            if w.project.is_none() {
                w.project = w
                    .identity
                    .sessions
                    .iter()
                    .filter_map(|n| names.get(n))
                    .find(|p| coords.contains_key(*p))
                    .cloned();
            }
            if let Some(p) = &w.project {
                if w.coordinator.is_none() {
                    w.coordinator = coords.get(p).filter(|s| !s.is_empty()).cloned();
                }
            }
        }
        let memberships: HashMap<_, _> = workspaces
            .iter()
            .map(|w| (w.identity.window_id.clone(), w.identity.sessions.clone()))
            .collect();
        for w in &mut workspaces {
            w.coordinator_available = w
                .coordinator
                .as_ref()
                .and_then(|id| memberships.get(id))
                .is_some_and(|sessions| {
                    w.project
                        .as_ref()
                        .and_then(|p| projects.get(p))
                        .is_some_and(|name| sessions.contains(name))
                });
        }
        let (details, checkouts) = enrich(&mut workspaces)?;
        let mut memberships: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
        for row in tmux(&["list-windows", "-a", "-F", "#{session_id}␟#{window_id}"])?.lines() {
            if let Some((session, window)) = row.split_once('␟') {
                memberships
                    .entry(session.into())
                    .or_default()
                    .insert(window.into());
            }
        }
        // Repository groups are display/filter identities only. Never assign
        // them to Workspace.project, which routes real tmux session actions.
        for w in &workspaces {
            let key = project_key(&workspaces, w);
            if key.starts_with("repo:") {
                projects.insert(key.clone(), project_group_label(&key).into());
            }
        }
        Ok(Self {
            memberships,
            details,
            checkouts,
            workspaces,
            projects,
            elapsed_ms: start.elapsed().as_millis(),
        })
    }
    pub fn project_id(&self, requested: &str) -> io::Result<String> {
        if requested == "unassociated" {
            return Ok(requested.into());
        }
        if self.projects.contains_key(requested) {
            return Ok(requested.into());
        }
        self.projects
            .iter()
            .find(|(_, name)| name.as_str() == requested)
            .map(|(id, _)| id.clone())
            .ok_or_else(|| {
                io::Error::other("Project is unavailable; refresh or select an existing project")
            })
    }
    pub fn visible(&self, query: &Query) -> io::Result<Vec<usize>> {
        let project = query
            .project
            .as_deref()
            .map(|p| self.project_id(p))
            .transpose()?;
        let members = query
            .local_session
            .as_ref()
            .map(|session| {
                self.memberships.get(session).ok_or_else(|| {
                    io::Error::other("Local session disappeared; reopen its inventory")
                })
            })
            .transpose()?;
        let search = query.search.to_lowercase();
        let mut result: Vec<_> = self
            .workspaces
            .iter()
            .enumerate()
            .filter(|(_, w)| {
                (query.windows
                    || is_worker(w)
                    || (!search.is_empty()
                        && w.coordinator.as_deref() == Some(w.identity.window_id.as_str())))
                    && project.as_ref().is_none_or(|p| {
                        if p == "unassociated" {
                            project_key(&self.workspaces, w) == "unassociated"
                        } else {
                            project_key(&self.workspaces, w) == *p
                                || w.identity
                                    .sessions
                                    .iter()
                                    .any(|s| self.projects.get(p) == Some(s))
                        }
                    })
                    && members.is_none_or(|ids| ids.contains(&w.identity.window_id))
                    && query.state.matches(w)
                    && (search.is_empty()
                        || [
                            w.identity.window_name.as_str(),
                            w.identity.session.as_str(),
                            w.checkout.branch.as_deref().unwrap_or(""),
                            w.agent.label(),
                            self.projects
                                .get(&project_key(&self.workspaces, w))
                                .map(String::as_str)
                                .unwrap_or("unassociated"),
                            w.checkout
                                .repository
                                .as_deref()
                                .and_then(Path::to_str)
                                .unwrap_or(""),
                            w.lifecycle.label(),
                            attention_state(w).label(),
                            self.details
                                .get(&w.identity.window_id)
                                .and_then(|d| d.batch.as_ref())
                                .map(|b| b.source.reference.as_str())
                                .unwrap_or(""),
                            self.details
                                .get(&w.identity.window_id)
                                .and_then(|d| d.batch.as_ref())
                                .map(|b| b.destination.as_str())
                                .unwrap_or(""),
                        ]
                        .iter()
                        .any(|s| s.to_lowercase().contains(&search))
                        || w.identity
                            .sessions
                            .iter()
                            .any(|s| s.to_lowercase().contains(&search)))
            })
            .map(|(i, _)| i)
            .collect();
        result.sort_by_key(|i| {
            let w = &self.workspaces[*i];
            (
                if query.group == Group::Project {
                    project_key(&self.workspaces, w)
                } else {
                    String::new()
                },
                !needs_attention(w),
                attention_state(w).label(),
                w.identity.window_id.clone(),
            )
        });
        Ok(result)
    }
    pub fn display(&self, query: &Query, redact: bool) -> io::Result<String> {
        let visible = self.visible(query)?;
        let mut output = format!(
            "{}\n{} · refresh {}ms · {} unique checkouts\n",
            Totals::from_workspaces(&self.workspaces).summary(),
            matching_label(&self.workspaces, &visible, query.windows),
            self.elapsed_ms,
            self.checkouts
        );
        for i in visible {
            let w = &self.workspaces[i];
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                w.identity.window_id,
                if redact {
                    "[redacted]"
                } else {
                    &w.identity.window_name
                },
                w.role(),
                w.lifecycle.label(),
                w.process_label(),
                if redact {
                    "[redacted]"
                } else {
                    self.projects
                        .get(&project_key(&self.workspaces, w))
                        .map(String::as_str)
                        .unwrap_or("unassociated")
                },
                if redact {
                    "[redacted]"
                } else {
                    &w.identity.session
                },
                if redact {
                    "[redacted]"
                } else {
                    w.checkout.branch.as_deref().unwrap_or("unknown")
                }
            ));
        }
        for (id, detail) in &self.details {
            if let Some(warning) = &detail.warning {
                output.push_str(&format!(
                    "UNAVAILABLE {id}: {}\n",
                    if redact {
                        "metadata unavailable"
                    } else {
                        warning
                    }
                ));
            }
        }
        if self.visible(query)?.is_empty() {
            output.push_str("No workspaces match. Clear filters to see all projects.\n");
        }
        Ok(output)
    }
}

// Status names only; no diff bodies, file contents or task contents are read.
fn git(path: &Path, args: &[&str]) -> io::Result<Vec<u8>> {
    let output = crate::workspace::checkout_git(path, args).output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            "Git metadata unavailable; refresh or inspect checkout",
        ));
    }
    Ok(output.stdout)
}
/// A checkout is its canonical root and per-worktree Git directory. The shared
/// common directory alone would collapse independent linked branches.
#[derive(Clone, Eq, PartialEq, Hash)]
struct CheckoutIdentity {
    root: PathBuf,
    common_dir: PathBuf,
    git_dir: PathBuf,
}
fn checkout_identity(path: &Path) -> io::Result<CheckoutIdentity> {
    let raw = git(
        path,
        &[
            "rev-parse",
            "--show-toplevel",
            "--path-format=absolute",
            "--git-common-dir",
            "--git-dir",
        ],
    )?;
    let text = String::from_utf8_lossy(&raw);
    let f: Vec<_> = text.lines().collect();
    if f.len() != 3 {
        return Err(io::Error::other("Git checkout identity unavailable"));
    }
    Ok(CheckoutIdentity {
        root: Path::new(f[0]).canonicalize()?,
        common_dir: Path::new(f[1]).canonicalize()?,
        git_dir: Path::new(f[2]).canonicalize()?,
    })
}
fn checkout(identity: &CheckoutIdentity) -> io::Result<(Checkout, Option<String>, Vec<String>)> {
    let path = &identity.root;
    let linked = identity.common_dir != identity.git_dir;
    let branch = String::from_utf8_lossy(&git(path, &["branch", "--show-current"])?)
        .trim_end()
        .to_owned();
    let commit = git(path, &["rev-parse", "HEAD"])
        .ok()
        .map(|s| String::from_utf8_lossy(&s).trim_end().to_owned());
    let status = git(
        path,
        &["status", "--porcelain=v1", "-z", "--untracked-files=normal"],
    )?;
    let mut files = Vec::new();
    let mut records = status.split(|b| *b == 0).filter(|b| !b.is_empty());
    while let Some(record) = records.next() {
        if record.len() < 3 {
            return Err(io::Error::other("Git status metadata malformed"));
        }
        files.push(String::from_utf8_lossy(&record[3..]).into_owned());
        if record[..2].iter().any(|b| *b == b'R' || *b == b'C') {
            if let Some(old) = records.next() {
                files.push(String::from_utf8_lossy(old).into_owned());
            }
        }
    }
    Ok((
        Checkout {
            working_directory: path.into(),
            repository: if linked {
                identity.common_dir.parent().map(PathBuf::from)
            } else {
                Some(identity.root.clone())
            },
            worktree: linked.then(|| identity.root.clone()),
            branch: (!branch.is_empty()).then_some(branch),
            git_state: if files.is_empty() {
                GitState::Clean
            } else {
                GitState::Dirty
            },
            is_linked_worktree: linked,
        },
        commit,
        files,
    ))
}
fn enrich(workspaces: &mut [Workspace]) -> io::Result<(HashMap<String, Detail>, usize)> {
    let rows = tmux(&[
        "list-panes",
        "-a",
        "-F",
        "#{window_id}␟#{pane_id}␟#{pane_pid}␟#{pane_dead}␟#{@drudwyn_launch_pane}␟#{@drudwyn_launch_pid}␟#{@drudwyn_launch_checkout}␟#{@drudwyn_recovery_checkout}␟#{@drudwyn_batch}␟#{@drudwyn_task_reference}␟#{@drudwyn_delivery}␟#{@drudwyn_delivery_wait}",
    ])?;
    let records: HashMap<_, _> = rows
        .lines()
        .filter_map(|row| {
            let f: Vec<_> = row.split('␟').collect();
            (f.len() == 12).then(|| (f[1], f))
        })
        .collect();
    let mut batches = HashMap::new();
    let mut identities = HashMap::new();
    let mut checkouts = HashMap::new();
    let mut details = HashMap::new();
    let mut destinations = HashMap::new();
    let mut containment = HashMap::new();
    let mut verification = HashMap::new();
    for w in workspaces {
        let mut detail = Detail::default();
        let mut source_common = None;
        if let Some(f) = records
            .get(w.identity.pane_id.as_str())
            .filter(|f| f[0] == w.identity.window_id)
        {
            let encoded = if f[4] == f[1] && f[5] == f[2] {
                f[6]
            } else {
                f[7]
            };
            let path = if !encoded.is_empty() {
                crate::recovery::decode(encoded).filter(|p| p.is_absolute())
            } else if f[3] == "0" {
                std::fs::read_link(format!("/proc/{}/cwd", f[2]))
                    .ok()
                    .or_else(|| Some(w.checkout.working_directory.clone()))
            } else {
                None
            };
            if let Some(path) = path.and_then(|p| p.canonicalize().ok()) {
                let identity = identities
                    .entry(path.clone())
                    .or_insert_with(|| checkout_identity(&path).map_err(|e| e.to_string()));
                let data = match identity {
                    Ok(identity) => {
                        source_common = Some(identity.common_dir.clone());
                        checkouts
                            .entry(identity.clone())
                            .or_insert_with(|| checkout(identity).map_err(|e| e.to_string()))
                            .as_ref()
                            .map_err(String::as_str)
                    }
                    Err(error) => Err(error.as_str()),
                };
                match data {
                    Ok((checkout, commit, files)) => {
                        w.checkout = checkout.clone();
                        // The cached Git root is not the selected pane's cwd.
                        w.checkout.working_directory = path;
                        detail.commit = commit.clone();
                        detail.changed_files = files.clone();
                    }
                    Err(error) => {
                        w.checkout = unknown_checkout(path);
                        detail.warning = Some(error.to_owned());
                    }
                }
            } else {
                w.checkout = unknown_checkout(w.checkout.working_directory.clone());
                detail.warning = Some(
                    "Checkout unavailable; stale display metadata is not authoritative".into(),
                );
            }
            if !f[8].is_empty() {
                let batch = batches
                    .entry(f[8])
                    .or_insert_with(|| crate::batch::load(f[8]).map_err(|e| e.to_string()));
                match batch {
                    Ok(b) => detail.batch = Some(b.clone()),
                    Err(e) => detail.warning = Some(e.clone()),
                }
            }
            detail.task_reference = crate::recovery::decode(f[9]).map(|p| p.display().to_string());
            if let (Some(batch), Some(commit), Some(common)) =
                (&detail.batch, &detail.commit, source_common)
            {
                let target = destinations
                    .entry((
                        batch.repository.clone(),
                        batch.destination.clone(),
                        batch.checkout.clone(),
                    ))
                    .or_insert_with(|| {
                        crate::integration::batch_destination(batch).map_err(|e| e.to_string())
                    });
                if let Ok(target) = target {
                    detail.verification = Some(verification.entry(target.path.clone()).or_insert_with(|| {
                        crate::verification::inspect(&target.path, false).unwrap_or_else(|_| "Assembled verification: Not verified · metadata unavailable".into())
                    }).clone());
                }
                detail.integration = Some(match target {
                    Ok(target) => containment
                        .entry((target.clone(), common.clone(), commit.clone()))
                        .or_insert_with(|| {
                            target
                                .observe(&common, commit)
                                .map(str::to_owned)
                                .unwrap_or_else(|_| "unknown · ancestry unavailable".into())
                        })
                        .clone(),
                    Err(_) => "unknown · destination unavailable or association changed".into(),
                });
            }
            detail.source_commit = detail.batch.as_ref().map(|b| b.source.commit.clone());
            detail.delivery = crate::workspace::observed_delivery(
                f[10],
                f[11],
                f[4] == f[1] && f[5] == f[2] && f[3] == "0",
            );
            if matches!(
                detail.delivery.as_str(),
                "waiting" | "setup_expired" | "setup_interrupted" | "setup_changed"
            ) {
                detail.warning = Some(crate::workspace::delivery_label(&detail.delivery).into());
            }
        } else {
            detail.warning =
                Some("Selected pane disappeared during refresh; refresh before action".into());
        }
        details.insert(w.identity.window_id.clone(), detail);
    }
    Ok((details, checkouts.len()))
}

fn unknown_checkout(path: PathBuf) -> Checkout {
    Checkout {
        working_directory: path,
        repository: None,
        worktree: None,
        branch: None,
        git_state: GitState::Unknown,
        is_linked_worktree: false,
    }
}

/// Coordinators remain searchable without becoming workers in matching totals.
pub fn matching_label(workspaces: &[Workspace], visible: &[usize], windows: bool) -> String {
    if windows {
        return format!("MATCHING {} windows", visible.len());
    }
    let workers = visible
        .iter()
        .filter(|i| is_worker(&workspaces[**i]))
        .count();
    let coordinators = visible.len() - workers;
    format!(
        "MATCHING {workers} workers{}",
        if coordinators == 0 {
            String::new()
        } else {
            format!(" · {coordinators} coordinators")
        }
    )
}
