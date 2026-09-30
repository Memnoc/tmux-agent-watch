//! Immutable batch records live in their project session, never on disk.
use crate::{
    coordinator,
    navigation::{self, tmux},
    workspace::{self, Error, StartPoint, git},
};
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub struct Request {
    pub repo: PathBuf,
    pub session: String,
    pub base: String,
    /// `base`, `current`, or an explicit locally available revision.
    pub source: String,
    pub integration: Option<String>,
    pub destination_start: Option<String>,
    pub checkout: Option<PathBuf>,
    pub reuse_existing: bool,
}

#[derive(Clone, Debug)]
pub struct Preview {
    pub request: Request,
    pub project: String,
    pub coordinator: String,
    pub repository: String,
    pub source: StartPoint,
    pub destination: String,
    pub destination_commit: String,
    pub checkout: PathBuf,
    pub create_checkout: bool,
    pub create_branch: bool,
    pub dirty_source: bool,
}

#[derive(Clone, Debug)]
pub struct Batch {
    pub id: String,
    pub project: String,
    pub coordinator: String,
    pub repository: String,
    pub source: StartPoint,
    pub destination: String,
    pub destination_commit: String,
    pub checkout: PathBuf,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
fn safe(value: &str) -> Result<(), Error> {
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(invalid(
            "Batch labels and paths must be nonempty and contain no control characters",
        ));
    }
    Ok(())
}
pub fn resolve(repo: &Path, reference: &str) -> Result<StartPoint, Error> {
    safe(reference)?;
    let commit = git(
        repo,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{reference}^{{commit}}"),
        ],
    )?;
    Ok(StartPoint {
        reference: reference.into(),
        commit,
    })
}

pub fn preview(request: Request) -> Result<Preview, Error> {
    if std::env::var_os("DRUDWYN_CLIENT").is_some() {
        navigation::client()?;
    }
    let project = coordinator::project(&request.session)?;
    let repository = coordinator::repository(&request.repo)?;
    let recorded = tmux(&[
        "show-option",
        "-qv",
        "-t",
        &project,
        "@drudwyn_project_repo",
    ])?;
    let coordinator = tmux(&["show-option", "-qv", "-t", &project, "@drudwyn_coordinator"])?;
    let windows = tmux(&["list-windows", "-t", &project, "-F", "#{window_id}"])?;
    if recorded != repository || !windows.lines().any(|w| w == coordinator) {
        return Err(invalid(
            "Batch requires an explicit, available coordinator for this repository; use coordinator set",
        ));
    }
    let source = match request.source.as_str() {
        "base" => workspace::resolve_start_point(&request.repo, &request.base, false)?,
        "current" => workspace::resolve_start_point(&request.repo, &request.base, true)?,
        reference => resolve(&request.repo, reference)?,
    };
    let destination = request
        .integration
        .clone()
        .unwrap_or_else(|| request.base.clone());
    safe(&destination)?;
    git(
        &request.repo,
        &["check-ref-format", &format!("refs/heads/{destination}")],
    )?;
    let existing = resolve(&request.repo, &format!("refs/heads/{destination}")).ok();
    if existing.is_some() && request.integration.is_some() && !request.reuse_existing {
        return Err(invalid(
            "Integration branch already exists; explicitly select it with --reuse-existing",
        ));
    }
    if existing.is_some() && request.destination_start.is_some() {
        return Err(invalid(
            "An existing destination keeps its current commit; omit --destination-start",
        ));
    }
    let destination_commit = match &existing {
        Some(point) => point.commit.clone(),
        None if request.integration.is_some() => match &request.destination_start {
            Some(reference) => resolve(&request.repo, reference)?.commit,
            None => workspace::resolve_start_point(&request.repo, &request.base, false)?.commit,
        },
        None => {
            return Err(invalid(
                "Configured destination branch is unavailable locally",
            ));
        }
    };
    // NUL records preserve spaces in worktree paths. Control characters are not
    // accepted as operational labels, including newlines in filesystem paths.
    let rows = git(&request.repo, &["worktree", "list", "--porcelain", "-z"])?;
    let mut path = None;
    let mut found = None;
    for field in rows.split('\0') {
        if let Some(value) = field.strip_prefix("worktree ") {
            path = Some(PathBuf::from(value));
        }
        if field == format!("branch refs/heads/{destination}") {
            found = path.clone();
        }
    }
    let (checkout, create_checkout) = match (found, &request.checkout) {
        (Some(found), requested) => {
            if let Some(requested) = requested {
                if requested.canonicalize()? != found.canonicalize()? {
                    return Err(invalid(
                        "Destination is already checked out elsewhere; select its existing checkout",
                    ));
                }
            }
            (found.canonicalize()?, false)
        }
        (None, Some(path)) => {
            let path = std::path::absolute(path)?;
            if path.try_exists()? {
                return Err(invalid(
                    "Destination path already exists; choose an unused path",
                ));
            }
            (path, true)
        }
        (None, None) => {
            return Err(invalid(
                "Destination needs a dedicated checkout; explicitly supply --checkout PATH (coordinator branch is preserved)",
            ));
        }
    };
    safe(&checkout.to_string_lossy())?;
    safe(&repository)?;
    if !create_checkout
        && !git(
            &checkout,
            &["status", "--porcelain", "--untracked-files=all"],
        )?
        .is_empty()
    {
        return Err(invalid(
            "Destination checkout is dirty; commit or preserve its changes before selecting it",
        ));
    }
    let dirty_source = !git(
        &request.repo,
        &["status", "--porcelain", "--untracked-files=all"],
    )?
    .is_empty();
    Ok(Preview {
        request,
        project,
        coordinator,
        repository,
        source,
        destination,
        destination_commit,
        checkout,
        create_checkout,
        create_branch: existing.is_none(),
        dirty_source,
    })
}

pub fn create(reviewed: &Preview) -> Result<Batch, Error> {
    let fresh = preview(reviewed.request.clone())?;
    if fresh.dirty_source != reviewed.dirty_source
        || fresh.source.reference != reviewed.source.reference
        || fresh.source.commit != reviewed.source.commit
        || fresh.destination_commit != reviewed.destination_commit
        || fresh.checkout != reviewed.checkout
        || fresh.coordinator != reviewed.coordinator
        || fresh.create_branch != reviewed.create_branch
        || fresh.create_checkout != reviewed.create_checkout
    {
        return Err(invalid(
            "Source or destination changed after preview; review a fresh preview",
        ));
    }
    if fresh.create_checkout {
        let path = fresh
            .checkout
            .to_str()
            .ok_or_else(|| invalid("Checkout path must be UTF-8"))?;
        let mut args = vec!["worktree", "add"];
        if fresh.create_branch {
            args.extend(["-b", fresh.destination.as_str()]);
        }
        args.extend([
            "--",
            path,
            if fresh.create_branch {
                &fresh.destination_commit
            } else {
                &fresh.destination
            },
        ]);
        git(&fresh.request.repo, &args)?;
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let key = format!("{}-{nonce}", std::process::id());
    let batch = Batch {
        id: format!("{}/{key}", fresh.project),
        project: fresh.project,
        coordinator: fresh.coordinator,
        repository: fresh.repository,
        source: fresh.source,
        destination: fresh.destination,
        destination_commit: fresh.destination_commit,
        checkout: fresh.checkout,
    };
    // Encode every field so tmux command parsing and output whitespace trimming
    // cannot change a literal ref or path (notably trailing ';' and spaces).
    let record = [
        &batch.coordinator,
        &batch.repository,
        &batch.source.reference,
        &batch.source.commit,
        &batch.destination,
        &batch.destination_commit,
        batch.checkout.to_str().unwrap(),
    ]
    .map(|field| {
        field
            .bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    })
    .join(":");
    let record = format!("v1:{record}");
    tmux(&["set-option", "-t", &batch.project, &format!("@drudwyn_batch_{key}"), &record]).map_err(|error| invalid(format!("Batch metadata failed: {error}; destination checkout retained at {}. Select the existing destination when retrying", batch.checkout.display())))?;
    Ok(batch)
}

pub fn load(id: &str) -> Result<Batch, Error> {
    let (project, key) = id.split_once('/').ok_or_else(|| {
        invalid("Batch association unknown; select a live batch or set up a new one")
    })?;
    if project.len() < 2
        || !project.starts_with('$')
        || !project[1..].bytes().all(|b| b.is_ascii_digit())
        || key.is_empty()
        || !key.bytes().all(|b| b.is_ascii_digit() || b == b'-')
    {
        return Err(invalid("Invalid batch identity"));
    }
    let value = tmux(&[
        "show-option",
        "-qv",
        "-t",
        project,
        &format!("@drudwyn_batch_{key}"),
    ])?;
    if value.is_empty() {
        return Err(invalid(
            "Batch metadata unknown or lost; reselect source and destination explicitly",
        ));
    }
    let malformed = || {
        invalid("Batch metadata malformed or outdated; reselect source and destination explicitly")
    };
    let encoded = value.strip_prefix("v1:").ok_or_else(malformed)?;
    let fields = encoded
        .split(':')
        .map(|field| {
            if field.len() % 2 != 0 || !field.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(malformed());
            }
            let bytes = (0..field.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&field[i..i + 2], 16).map_err(|_| malformed()))
                .collect::<Result<Vec<_>, _>>()?;
            let field = String::from_utf8(bytes).map_err(|_| malformed())?;
            safe(&field).map_err(|_| malformed())?;
            Ok(field)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    if fields.len() != 7 {
        return Err(malformed());
    }
    Ok(Batch {
        id: id.into(),
        project: project.into(),
        coordinator: fields[0].clone(),
        repository: fields[1].clone(),
        source: StartPoint {
            reference: fields[2].clone(),
            commit: fields[3].clone(),
        },
        destination: fields[4].clone(),
        destination_commit: fields[5].clone(),
        checkout: PathBuf::from(&fields[6]),
    })
}

pub fn for_window(window: &str) -> Result<Option<Batch>, Error> {
    let id = tmux(&["show-option", "-wqv", "-t", window, "@drudwyn_batch"])?;
    if id.is_empty() {
        Ok(None)
    } else {
        load(&id).map(Some)
    }
}
pub fn current() -> Result<Option<Batch>, Error> {
    if std::env::var_os("DRUDWYN_CLIENT").is_some() {
        navigation::client()?;
    }
    if std::env::var_os("DRUDWYN_CLIENT").is_none() && std::env::var_os("TMUX_PANE").is_none() {
        return Ok(None);
    }
    for_window(&navigation::context("#{window_id}")?)
}
pub fn select(id: &str, window: &str) -> Result<(), Error> {
    let batch = load(id)?;
    let rows = tmux(&[
        "list-windows",
        "-t",
        &batch.project,
        "-F",
        "#{window_id}␟#{pane_current_path}",
    ])?;
    let path = rows
        .lines()
        .filter_map(|r| r.split_once('␟'))
        .find(|(w, _)| *w == window)
        .map(|(_, p)| p)
        .ok_or_else(|| invalid("Batch window is not in this project"))?;
    if coordinator::repository(Path::new(path))? != batch.repository {
        return Err(invalid("Batch belongs to another repository"));
    }
    tmux(&["set-option", "-w", "-t", window, "@drudwyn_batch", id])?;
    Ok(())
}
/// Initial launch already owns the exact checkout path. Verify project membership
/// and repository from that path rather than reinterpreting tmux's display string.
pub(crate) fn select_created(id: &str, window: &str, checkout: &Path) -> Result<(), Error> {
    let batch = load(id)?;
    let windows = tmux(&["list-windows", "-t", &batch.project, "-F", "#{window_id}"])?;
    if !windows.lines().any(|w| w == window)
        || coordinator::repository(checkout)? != batch.repository
    {
        return Err(invalid(
            "Created worker does not belong to this batch project",
        ));
    }
    tmux(&["set-option", "-w", "-t", window, "@drudwyn_batch", id])?;
    Ok(())
}
impl Batch {
    pub fn display(&self, redacted: bool) -> String {
        if redacted {
            return format!(
                "Batch: {}\nSource: [redacted]\nDestination: [redacted]\nCheckout: [redacted]",
                self.id
            );
        }
        format!(
            "Batch: {}\nSource: {} {}\nDestination: {} {} (at setup)\nCheckout: {}",
            self.id,
            self.source.reference,
            self.source.commit,
            self.destination,
            self.destination_commit,
            self.checkout.display()
        )
    }
}
impl Preview {
    pub fn display(&self, redacted: bool) -> String {
        let details = if redacted {
            "Source: [redacted]\nDestination: [redacted]\nCheckout: [redacted]".into()
        } else {
            format!(
                "Source: {} {}\nDestination: {} {} (at setup)\nCheckout: {}",
                self.source.reference,
                self.source.commit,
                self.destination,
                self.destination_commit,
                self.checkout.display()
            )
        };
        format!(
            "{details}\n{}\nLocal refs only; remote freshness unknown.{}",
            if self.create_checkout {
                "Create dedicated checkout; coordinator branch preserved."
            } else {
                "Reuse existing destination checkout."
            },
            if self.dirty_source {
                "\nSource checkout is dirty: uncommitted files are NOT inherited by workers."
            } else {
                ""
            }
        )
    }
}

pub fn select_current(id: &str) -> Result<(), Error> {
    if std::env::var_os("DRUDWYN_CLIENT").is_some() {
        navigation::client()?;
    }
    select(id, &navigation::context("#{window_id}")?)
}
