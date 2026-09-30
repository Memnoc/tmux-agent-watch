//! Explicit, content-blind review and integration into an existing destination.
//! A preview owns no reservation; apply reacquires facts under a checkout lock.
use crate::{batch, workspace::Error};
use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File},
    hash::{Hash, Hasher},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Clone, Debug, Hash)]
pub enum Destination {
    Batch(String),
    Branch(String),
}
#[derive(Clone, Debug)]
pub struct Request {
    pub source: PathBuf,
    pub destination: Destination,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Checkout {
    pub path: PathBuf,
    pub reference: String,
    pub commit: String,
    git_dir: PathBuf,
    common: PathBuf,
    device: u64,
    inode: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Plan {
    Contained,
    FastForward,
    Merge,
}
impl Plan {
    fn label(&self) -> &'static str {
        match self {
            Self::Contained => "Already contained · no mutation needed",
            Self::FastForward => "Fast-forward",
            Self::Merge => "Normal merge · divergent history",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Preview {
    pub request: Request,
    pub source: Checkout,
    pub target: Checkout,
    pub plan: Plan,
    pub files: Vec<String>,
    pub commits: Vec<String>,
    pub token: String,
}
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}
fn command(path: &Path, args: &[&str]) -> Command {
    crate::workspace::checkout_git(path, args)
}
fn raw(path: &Path, args: &[&str]) -> Result<Vec<u8>, Error> {
    let result = command(path, args).output()?;
    if !result.status.success() {
        return Err(invalid(
            "Git metadata unavailable; inspect the selected checkout and retry",
        ));
    }
    Ok(result.stdout)
}
fn git(path: &Path, args: &[&str]) -> Result<String, Error> {
    String::from_utf8(raw(path, args)?)
        .map(|s| s.trim_end_matches('\n').into())
        .map_err(|_| invalid("Git metadata is not valid UTF-8"))
}
fn safe(value: &str) -> Result<(), Error> {
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(invalid(
            "Integration paths and refs must be nonempty and contain no control characters",
        ));
    }
    Ok(())
}
fn canonical(path: &Path) -> Result<PathBuf, Error> {
    safe(
        path.to_str()
            .ok_or_else(|| invalid("Checkout path must be UTF-8"))?,
    )?;
    path.canonicalize()
        .map_err(|_| invalid("Selected checkout is unavailable; review its current location"))
}
fn checkout(path: &Path) -> Result<Checkout, Error> {
    let path = canonical(path)?;
    let root = canonical(Path::new(&git(&path, &["rev-parse", "--show-toplevel"])?))?;
    let git_dir = canonical(Path::new(&git(
        &root,
        &["rev-parse", "--path-format=absolute", "--git-dir"],
    )?))?;
    let common = canonical(Path::new(&git(
        &root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?))?;
    let reference = git(&root, &["symbolic-ref", "--quiet", "HEAD"])
        .map_err(|_| invalid("Detached checkout; select an explicit checked-out local branch"))?;
    if !reference.starts_with("refs/heads/") {
        return Err(invalid("Integration requires a checked-out local branch"));
    }
    safe(&reference)?;
    let commit = git(&root, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    if git(&root, &["symbolic-ref", "--quiet", "HEAD"])? != reference
        || git(
            &root,
            &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
        )? != commit
    {
        return Err(invalid(
            "Checkout branch or HEAD changed while inspecting it; review again",
        ));
    }
    let metadata = fs::metadata(&root)?;
    Ok(Checkout {
        path: root,
        reference,
        commit,
        git_dir,
        common,
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}
fn operation(checkout: &Checkout) -> Result<bool, Error> {
    for name in [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "rebase-merge",
        "rebase-apply",
        "sequencer",
        "BISECT_LOG",
        "index.lock",
        "HEAD.lock",
    ] {
        if checkout.git_dir.join(name).try_exists()? {
            return Ok(true);
        }
    }
    Ok(false)
}
fn ready(checkout: &Checkout) -> Result<(), Error> {
    if operation(checkout)? {
        return Err(invalid(
            "An existing Git operation or lock is present; resolve it before integration",
        ));
    }
    if !raw(
        &checkout.path,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?
    .is_empty()
    {
        return Err(invalid(
            "Source or destination is dirty (including untracked files); preserve and commit changes before integration",
        ));
    }
    Ok(())
}
fn ancestor(path: &Path, source: &str, target: &str) -> Result<bool, Error> {
    let status = command(path, &["merge-base", "--is-ancestor", source, target])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(invalid(
            "Git ancestry unavailable; no integration conclusion",
        )),
    }
}
fn destination(repo: &Path, branch: &str, recorded: Option<&Path>) -> Result<Checkout, Error> {
    safe(branch)?;
    git(repo, &["check-ref-format", &format!("refs/heads/{branch}")])?;
    let reference = format!("refs/heads/{branch}");
    let records = git(repo, &["worktree", "list", "--porcelain", "-z"])?;
    let mut matches = Vec::new();
    for record in records.split("\0\0") {
        let fields: Vec<_> = record.split('\0').collect();
        if !fields.iter().any(|f| *f == format!("branch {reference}")) {
            continue;
        }
        if fields
            .iter()
            .any(|f| *f == "prunable" || f.starts_with("prunable "))
        {
            return Err(invalid(
                "Destination checkout is prunable or unavailable; recover it explicitly",
            ));
        }
        let path = fields
            .iter()
            .find_map(|f| f.strip_prefix("worktree "))
            .ok_or_else(|| invalid("Destination checkout identity is unavailable"))?;
        matches.push(canonical(Path::new(path))?);
    }
    if matches.len() != 1 {
        return Err(invalid(
            "Destination must have exactly one existing checkout; select or create it explicitly with batch setup",
        ));
    }
    let target = checkout(&matches[0])?;
    if target.reference != reference {
        return Err(invalid(
            "Destination branch or repository changed; review again",
        ));
    }
    if let Some(recorded) = recorded {
        if canonical(recorded)? != target.path {
            return Err(invalid(
                "Batch destination checkout moved; explicitly reselect its branch or set up a new batch",
            ));
        }
    }
    Ok(target)
}
/// Snapshot enrichment caches this once per live destination, without requiring
/// a clean tree merely to observe committed ancestry. Apply always revalidates.
pub(crate) fn batch_destination(batch: &batch::Batch) -> Result<Checkout, Error> {
    let common = canonical(Path::new(&batch.repository))?;
    let target = destination(&common, &batch.destination, Some(&batch.checkout))?;
    if target.common != common {
        return Err(invalid("Batch destination repository changed"));
    }
    Ok(target)
}
impl Checkout {
    pub(crate) fn observe(
        &self,
        source_common: &Path,
        source_commit: &str,
    ) -> Result<&'static str, Error> {
        if source_common != self.common {
            return Err(invalid(
                "Source and destination repository association differ",
            ));
        }
        if operation(self)? {
            return Ok("Git operation or lock in destination · inspect before integrating");
        }
        if ancestor(&self.path, source_commit, &self.commit)? {
            Ok("Contained in selected destination · committed ancestry only")
        } else {
            Ok("Not contained in selected destination")
        }
    }
}
pub fn preview(request: Request) -> Result<Preview, Error> {
    let source = checkout(&request.source)?;
    ready(&source)?;
    let (branch, recorded) = match &request.destination {
        Destination::Batch(id) => {
            let selected = batch::load(id)?;
            if canonical(Path::new(&selected.repository))? != source.common {
                return Err(invalid(
                    "Batch belongs to another repository; explicitly select the destination",
                ));
            }
            (selected.destination, Some(selected.checkout))
        }
        Destination::Branch(branch) => (branch.clone(), None),
    };
    let target = destination(&source.path, &branch, recorded.as_deref())?;
    if target.common != source.common {
        return Err(invalid(
            "Destination belongs to another repository; select explicitly",
        ));
    }
    ready(&target)?;
    let plan = if ancestor(&source.path, &source.commit, &target.commit)? {
        Plan::Contained
    } else if ancestor(&source.path, &target.commit, &source.commit)? {
        Plan::FastForward
    } else {
        Plan::Merge
    };
    let commits = git(
        &source.path,
        &[
            "rev-list",
            "--reverse",
            &format!("{}..{}", target.commit, source.commit),
        ],
    )?
    .lines()
    .map(str::to_owned)
    .collect();
    let files = raw(
        &source.path,
        &[
            "diff",
            "--name-only",
            "-z",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            &target.commit,
            &source.commit,
            "--",
        ],
    )?
    .split(|b| *b == 0)
    .filter(|s| !s.is_empty())
    .map(|s| String::from_utf8_lossy(s).into_owned())
    .collect();
    // An ephemeral comparison token, not authentication or durable metadata.
    let mut hash = DefaultHasher::new();
    (
        "integration-v1",
        &request.destination,
        &source,
        &target,
        &plan,
    )
        .hash(&mut hash);
    let token = format!("{:016x}", hash.finish());
    Ok(Preview {
        request,
        source,
        target,
        plan,
        files,
        commits,
        token,
    })
}
impl Preview {
    pub fn display(&self, redact: bool) -> String {
        let label = |s: &str| {
            if redact {
                "[redacted]".into()
            } else {
                s.to_owned()
            }
        };
        let mut text = format!(
            "INTEGRATE PREVIEW\nSource: {}\nSource commit: {}\nSource checkout: {}\nDestination: {}\nDestination commit: {}\nTarget checkout: {}\nPlan: {}\nChecks: unknown · not verified\nCommits to integrate: {}\n",
            label(&self.source.reference),
            label(&self.source.commit),
            label(&self.source.path.to_string_lossy()),
            label(&self.target.reference),
            label(&self.target.commit),
            label(&self.target.path.to_string_lossy()),
            self.plan.label(),
            self.commits.len()
        );
        for commit in &self.commits {
            text.push_str(&format!("  {}\n", label(commit)));
        }
        text.push_str(&format!(
            "Changed-file metadata: {} names (target vs source; not predicted merge result)\n",
            self.files.len()
        ));
        for file in &self.files {
            text.push_str(&format!(
                "  {}\n",
                if redact {
                    "[redacted]".into()
                } else {
                    format!("{file:?}")
                }
            ));
        }
        text.push_str(&format!("Review token: {}\n", self.token));
        text
    }
}
/// The existing directory inode serializes aliases and all tmux clients/servers.
/// Only the mutating Git child inherits the descriptor; parent death cannot allow
/// another Drudwyn merge while that child still owns the mutation.
pub fn apply(reviewed: &Preview) -> Result<&'static str, Error> {
    let directory = File::open(&reviewed.target.path)?;
    directory.try_lock().map_err(|_| invalid("Destination integration/recovery/delivery already in progress; inspect before retrying"))?;
    let held = directory.metadata()?;
    if !held.is_dir() || held.dev() != reviewed.target.device || held.ino() != reviewed.target.inode
    {
        return Err(invalid(
            "Destination checkout changed after preview; review again",
        ));
    }
    let fresh = preview(reviewed.request.clone())?;
    if fresh.token != reviewed.token
        || fresh.source != reviewed.source
        || fresh.target != reviewed.target
    {
        return Err(invalid(
            "Source or destination changed after preview; review again",
        ));
    }
    if fresh.plan == Plan::Contained {
        return Ok("No-op: source already contained in destination; checks unknown");
    }
    let mode = if fresh.plan == Plan::FastForward {
        "--ff-only"
    } else {
        "--no-ff"
    };
    let status = command(
        Path::new("."),
        &[
            "merge",
            // Override branch.<name>.mergeOptions: the preview promises a
            // committed merge, never squash or an uncommitted alternate mode.
            "--no-squash",
            "--commit",
            "--no-edit",
            "--no-autostash",
            "--no-overwrite-ignore",
            mode,
            &fresh.source.commit,
        ],
    )
    .current_dir(&fresh.target.path)
    .stdin(Stdio::from(directory.try_clone()?))
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status()?;
    if !status.success() {
        return Err(invalid(
            if fresh.target.git_dir.join("MERGE_HEAD").try_exists()? {
                "Integration conflict or interrupted merge retained in destination; inspect there. No success, cleanup or automatic retry; coordinator resolution is separate"
            } else {
                "Git merge failed; destination and worker retained. Inspect Git state before retrying; no reset or cleanup attempted"
            },
        ));
    }
    let target = checkout(&fresh.target.path)?;
    if target.reference != fresh.target.reference
        || target.device != fresh.target.device
        || target.inode != fresh.target.inode
        || !ancestor(&target.path, &fresh.source.commit, &target.commit)?
    {
        return Err(invalid(
            "Destination changed during integration; result uncertain, inspect Git state; all work retained",
        ));
    }
    Ok(
        "Integrated: reviewed source is contained in selected destination; checks unknown; worker retained",
    )
}
