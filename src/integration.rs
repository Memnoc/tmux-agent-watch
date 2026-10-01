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
    pub(crate) git_dir: PathBuf,
    pub(crate) common: PathBuf,
    pub(crate) device: u64,
    pub(crate) inode: u64,
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
pub(crate) fn raw(path: &Path, args: &[&str]) -> Result<Vec<u8>, Error> {
    let result = command(path, args).output()?;
    if !result.status.success() {
        return Err(invalid(
            "Git metadata unavailable; inspect the selected checkout and retry",
        ));
    }
    Ok(result.stdout)
}
pub(crate) fn git(path: &Path, args: &[&str]) -> Result<String, Error> {
    String::from_utf8(raw(path, args)?)
        .map(|s| s.strip_suffix('\n').unwrap_or(&s).into())
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
pub(crate) fn canonical(path: &Path) -> Result<PathBuf, Error> {
    safe(
        path.to_str()
            .ok_or_else(|| invalid("Checkout path must be UTF-8"))?,
    )?;
    let result = path
        .canonicalize()
        .map_err(|_| invalid("Selected checkout is unavailable; review its current location"))?;
    safe(
        result
            .to_str()
            .ok_or_else(|| invalid("Checkout path must be UTF-8"))?,
    )?;
    Ok(result)
}
pub(crate) fn checkout(path: &Path) -> Result<Checkout, Error> {
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
pub(crate) fn operation(checkout: &Checkout) -> Result<bool, Error> {
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
pub(crate) fn ready(checkout: &Checkout) -> Result<(), Error> {
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
pub(crate) fn ancestor(path: &Path, source: &str, target: &str) -> Result<bool, Error> {
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
pub(crate) fn destination(
    repo: &Path,
    branch: &str,
    recorded: Option<&Path>,
) -> Result<Checkout, Error> {
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
/// Filename metadata only; disabling rename detection avoids similarity reads
/// and makes the safety check independent of the merge's rename settings.
fn changed_paths(path: &Path, base: &str, head: &str, filter: &str) -> Result<Vec<Vec<u8>>, Error> {
    Ok(raw(
        path,
        &[
            "diff",
            "--name-only",
            &format!("--diff-filter={filter}"),
            "--no-renames",
            "-z",
            base,
            head,
            "--",
        ],
    )?
    .split(|b| *b == 0)
    .filter(|p| !p.is_empty())
    .map(Vec::from)
    .collect())
}
fn path_collision(changed: &[u8], local: &[u8]) -> bool {
    let parent = |a: &[u8], b: &[u8]| b.starts_with(a) && b.get(a.len()) == Some(&b'/');
    changed == local || parent(changed, local) || parent(local, changed)
}
fn directory_prefixes(
    paths: &[Vec<u8>],
    include_root: bool,
) -> std::collections::BTreeSet<Vec<u8>> {
    let mut directories = std::collections::BTreeSet::new();
    if include_root && !paths.is_empty() {
        directories.insert(Vec::new());
    }
    for path in paths {
        for (index, byte) in path.iter().enumerate() {
            if *byte == b'/' {
                directories.insert(path[..index].to_vec());
            }
        }
    }
    directories
}
/// Deletions and additions/changes can denote directory moves, even when Git's
/// similarity thresholds or rename limits differ. Consider every relevant
/// ancestor mapping rather than claiming to predict Git's rename resolution.
/// Root is never a move source, but a real directory may be flattened into it.
fn relocated_collision(
    incoming: &[Vec<u8>],
    removed: &[Vec<u8>],
    destinations: &[Vec<u8>],
    ignored: &[Vec<u8>],
) -> bool {
    let old_dirs = directory_prefixes(removed, false);
    let new_dirs = directory_prefixes(destinations, true);
    for path in incoming {
        for old in &old_dirs {
            if !path.starts_with(old) || path.get(old.len()) != Some(&b'/') {
                continue;
            }
            let suffix = &path[old.len() + 1..];
            for new in &new_dirs {
                let mut candidate = new.clone();
                if !candidate.is_empty() {
                    candidate.push(b'/');
                }
                candidate.extend_from_slice(suffix);
                if ignored
                    .iter()
                    .any(|local| path_collision(&candidate, local))
                {
                    return true;
                }
            }
        }
    }
    false
}
/// Git's --no-overwrite-ignore does not protect every divergent merge path.
/// Check direct writes and conservative directory-relocated outputs in both
/// directions without reading bodies, invoking a merge preview, or mutating Git.
fn protect_ignored(target: &Checkout, source: &Checkout) -> Result<(), Error> {
    let ignored: Vec<Vec<u8>> = raw(
        &target.path,
        &[
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "-z",
        ],
    )?
    .split(|b| *b == 0)
    .filter(|p| !p.is_empty())
    .map(Vec::from)
    .collect();
    if ignored.is_empty() {
        return Ok(());
    }
    let bases = git(
        &target.path,
        &["merge-base", "--all", &target.commit, &source.commit],
    )?;
    for base in bases.lines() {
        let incoming = changed_paths(&target.path, base, &source.commit, "ACMRT")?;
        let target_changes = changed_paths(&target.path, base, &target.commit, "ACMRT")?;
        let target_removed = changed_paths(&target.path, base, &target.commit, "D")?;
        let source_removed = changed_paths(&target.path, base, &source.commit, "D")?;
        let direct = incoming
            .iter()
            .any(|changed| ignored.iter().any(|local| path_collision(changed, local)));
        if direct
            || relocated_collision(&incoming, &target_removed, &target_changes, &ignored)
            || relocated_collision(&target_changes, &source_removed, &incoming, &ignored)
        {
            return Err(invalid(
                "Git merge blocked by possible ignored destination path collision, including directory relocation; destination and worker retained; no mutation attempted. Inspect or move only the colliding local work before retry",
            ));
        }
    }
    Ok(())
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
    protect_ignored(&fresh.target, &fresh.source)?;
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
        if fresh.target.git_dir.join("MERGE_HEAD").try_exists()? {
            retain_conflict(&fresh, &directory)?;
            return Err(invalid(
                "Integration conflict or interrupted merge retained in destination; inspect with workspace conflict --path TARGET. Open/recover coordinator; Continue/Abort available; checks unknown",
            ));
        }
        return Err(invalid(
            "Git merge failed; destination and worker retained. Inspect Git state before retrying; no reset or cleanup attempted",
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

/// Metadata owned by the live tmux server, never a file or a task history.
#[derive(Clone)]
struct Conflict {
    target: Checkout,
    source_ref: String,
    source_commit: String,
    merge_identity: String,
    project: String,
    delivery: String,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConflictAction {
    Inspect,
    Continue,
    Abort,
    Retry,
    Open,
}
#[derive(PartialEq, Eq)]
enum ConflictState {
    Unmerged,
    Resolved,
    Completed,
    Aborted,
}
fn merge_identity(target: &Checkout) -> Result<String, Error> {
    let m = fs::symlink_metadata(target.git_dir.join("MERGE_HEAD"))?;
    if !m.is_file() {
        return Err(invalid(
            "Merge operation identity unavailable; inspect Git manually",
        ));
    }
    Ok(format!(
        "{}:{}:{}:{}:{}:{}:{}",
        m.dev(),
        m.ino(),
        m.ctime(),
        m.ctime_nsec(),
        m.mtime(),
        m.mtime_nsec(),
        m.len()
    ))
}
fn conflict_option(path: &Path) -> String {
    let mut hash = DefaultHasher::new();
    path.hash(&mut hash);
    format!("@drudwyn_conflict_{:016x}", hash.finish())
}
fn encode(value: &str) -> String {
    crate::recovery::encode(Path::new(value))
}
fn decode(value: &str) -> Result<String, Error> {
    if value.is_empty() {
        return Ok(String::new());
    }
    crate::recovery::decode(value)
        .and_then(|p| p.into_os_string().into_string().ok())
        .ok_or_else(|| invalid("Conflict receipt malformed; inspect destination; nothing changed"))
}
impl Conflict {
    fn save(&self, guard: &File) -> Result<(), Error> {
        let fields = [
            "1".to_owned(),
            encode(self.target.path.to_str().unwrap()),
            encode(&self.target.reference),
            self.target.commit.clone(),
            encode(self.target.git_dir.to_str().unwrap()),
            encode(self.target.common.to_str().unwrap()),
            self.target.device.to_string(),
            self.target.inode.to_string(),
            encode(&self.source_ref),
            self.source_commit.clone(),
            self.merge_identity.clone(),
            encode(&self.project),
            self.delivery.clone(),
        ];
        let status = Command::new("tmux")
            .args([
                "set-option",
                "-s",
                &conflict_option(&self.target.path),
                &fields.join("|"),
            ])
            .stdin(Stdio::from(guard.try_clone()?))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !status.success() {
            return Err(invalid(
                "Could not retain live conflict receipt; merge preserved; inspect destination",
            ));
        }
        Ok(())
    }
    fn load(path: &Path) -> Result<Self, Error> {
        let value = crate::navigation::tmux(&["show-option", "-sqv", &conflict_option(path)])?;
        let f: Vec<_> = value.split('|').collect();
        if f.len() != 13 || f[0] != "1" {
            return Err(invalid(
                "No live conflict receipt; inspect destination. Lost operation history cannot authorize Continue or Abort",
            ));
        }
        let number = |value: &str| {
            value
                .parse::<u64>()
                .map_err(|_| invalid("Conflict checkout identity malformed"))
        };
        let receipt = Self {
            target: Checkout {
                path: PathBuf::from(decode(f[1])?),
                reference: decode(f[2])?,
                commit: f[3].into(),
                git_dir: PathBuf::from(decode(f[4])?),
                common: PathBuf::from(decode(f[5])?),
                device: number(f[6])?,
                inode: number(f[7])?,
            },
            source_ref: decode(f[8])?,
            source_commit: f[9].into(),
            merge_identity: f[10].into(),
            project: decode(f[11])?,
            delivery: f[12].into(),
        };
        if receipt.target.path != path {
            return Err(invalid(
                "Conflict destination identity changed; nothing changed",
            ));
        }
        Ok(receipt)
    }
    fn state(&self) -> Result<ConflictState, Error> {
        let current = checkout(&self.target.path)?;
        if current.path != self.target.path
            || current.reference != self.target.reference
            || current.git_dir != self.target.git_dir
            || current.common != self.target.common
            || current.device != self.target.device
            || current.inode != self.target.inode
        {
            return Err(invalid(
                "Conflict destination changed; nothing changed; inspect the selected checkout",
            ));
        }
        for name in [
            "CHERRY_PICK_HEAD",
            "REVERT_HEAD",
            "rebase-merge",
            "rebase-apply",
            "sequencer",
            "BISECT_LOG",
            "index.lock",
            "HEAD.lock",
        ] {
            if current.git_dir.join(name).try_exists()? {
                return Err(invalid(
                    "Another Git operation or lock is present; nothing changed",
                ));
            }
        }
        if current.git_dir.join("MERGE_HEAD").try_exists()? {
            if merge_identity(&current)? != self.merge_identity
                || current.commit != self.target.commit
                || git(
                    &current.path,
                    &["rev-parse", "--verify", "MERGE_HEAD^{commit}"],
                )? != self.source_commit
                || merge_identity(&current)? != self.merge_identity
            {
                return Err(invalid(
                    "Expected merge operation changed or was replaced; nothing changed; inspect destination",
                ));
            }
            return Ok(
                if raw(&current.path, &["ls-files", "--unmerged", "-z"])?.is_empty() {
                    ConflictState::Resolved
                } else {
                    ConflictState::Unmerged
                },
            );
        }
        if ancestor(&current.path, &self.source_commit, &current.commit)? {
            return Ok(ConflictState::Completed);
        }
        if current.commit == self.target.commit {
            return Ok(ConflictState::Aborted);
        }
        Err(invalid(
            "Merge ended with a changed destination; outcome unknown; nothing repeated",
        ))
    }
    fn coordinator(&self) -> Result<String, Error> {
        if self.project.is_empty() {
            return Err(invalid(
                "Coordinator association unknown; recover/select a coordinator in the destination before retry",
            ));
        }
        let window = crate::navigation::tmux(&[
            "show-option",
            "-qv",
            "-t",
            &self.project,
            "@drudwyn_coordinator",
        ])?;
        let members =
            crate::navigation::tmux(&["list-windows", "-t", &self.project, "-F", "#{window_id}"])?;
        if window.is_empty() || !members.lines().any(|w| w == window) {
            return Err(invalid(
                "Coordinator unavailable; Open/recover it in the destination before retry",
            ));
        }
        Ok(window)
    }
    fn handoff(&self, guard: &File) -> Result<(), Error> {
        let window = self.coordinator()?;
        let instruction = format!(
            "Resolve the retained Git merge in checkout {:?}. Integrate source ref {:?} at commit {} into target ref {:?}, starting from commit {}. Inspect the conflicts yourself, preserve the intended changes, and stage resolutions. Do not start another merge, reset, remove the worktree, or claim checks passed. Use Drudwyn Continue when resolved, or report why Abort is needed.",
            self.target.path,
            self.source_ref,
            self.source_commit,
            self.target.reference,
            self.target.commit
        );
        crate::workspace::deliver_coordinator(
            &window,
            &self.target.path,
            &instruction,
            guard,
            || {
                if !matches!(
                    self.state()?,
                    ConflictState::Unmerged | ConflictState::Resolved
                ) || self.coordinator()? != window
                {
                    return Err(invalid("Conflict or coordinator changed; nothing resent"));
                }
                Ok(())
            },
            |state| {
                let mut updated = self.clone();
                updated.delivery = state.into();
                updated.save(guard)
            },
        )
    }
    fn display(&self, state: ConflictState, redact: bool) -> String {
        let label = |v: &str| if redact { "[redacted]" } else { v }.to_owned();
        let state = match state {
            ConflictState::Unmerged => "Conflict retained · unmerged entries remain",
            ConflictState::Resolved => "Conflict retained · resolved entries; Continue available",
            ConflictState::Completed => "Completed externally or by Continue · source contained",
            ConflictState::Aborted => "Aborted or ended without integration",
        };
        format!(
            "{state}\nSource: {}\nSource commit: {}\nDestination: {}\nInitial target commit: {}\nTarget checkout: {}\nHandoff: {}\nOpen coordinator / recover coordinator in target checkout / deliberate Retry\nContinue requires resolved expected merge; Abort uses Git normally; checks unknown; worker retained",
            label(&self.source_ref),
            label(&self.source_commit),
            label(&self.target.reference),
            label(&self.target.commit),
            label(self.target.path.to_str().unwrap()),
            self.delivery
        )
    }
}
fn retain_conflict(reviewed: &Preview, guard: &File) -> Result<(), Error> {
    let project = match &reviewed.request.destination {
        Destination::Batch(id) => batch::load(id).map(|b| b.project).unwrap_or_default(),
        Destination::Branch(_) => crate::coordinator::launch_project(&reviewed.source.path)
            .ok()
            .flatten()
            .unwrap_or_default(),
    };
    let conflict = Conflict {
        target: reviewed.target.clone(),
        source_ref: reviewed.source.reference.clone(),
        source_commit: reviewed.source.commit.clone(),
        merge_identity: merge_identity(&reviewed.target)?,
        project,
        delivery: "not_sent".into(),
    };
    conflict.state()?;
    conflict.save(guard)?;
    // One automatic initial attempt only. The receipt precedes transmission;
    // any further attempt is an explicit user action.
    if let Err(error) = conflict.handoff(guard) {
        let current = Conflict::load(&conflict.target.path)?;
        if current.delivery == "not_sent" {
            let mut unavailable = current;
            unavailable.delivery =
                "not_sent (Open/recover coordinator in target checkout; deliberate Retry)".into();
            unavailable.save(guard)?;
        }
        let _ = error; // Fixed receipt only; never retain agent/task/error content.
    }
    Ok(())
}
/// A failed Apply may open only the active operation from that reviewed attempt.
/// Historical completed receipts remain inspectable through the explicit action.
pub(crate) fn active_conflict_for(reviewed: &Preview) -> bool {
    Conflict::load(&reviewed.target.path).is_ok_and(|receipt| {
        receipt.target == reviewed.target
            && receipt.source_ref == reviewed.source.reference
            && receipt.source_commit == reviewed.source.commit
            && matches!(
                receipt.state(),
                Ok(ConflictState::Unmerged | ConflictState::Resolved)
            )
    })
}

/// Explicit agent creation for one expected conflict. No prompt is sent here:
/// the user inspects this new conversation before deliberately retrying handoff.
pub fn recover_conflict_agent(
    path: &Path,
    agent: crate::domain::AgentKind,
) -> Result<String, Error> {
    let client = crate::navigation::client()?;
    let path = canonical(path)?;
    let guard = File::open(&path)?;
    guard.try_lock().map_err(|_| {
        invalid("Destination integration/recovery/delivery already in progress; retry explicitly")
    })?;
    let receipt = Conflict::load(&path)?;
    if !matches!(
        receipt.state()?,
        ConflictState::Unmerged | ConflictState::Resolved
    ) {
        return Err(invalid("Merge already ended; no agent started"));
    }
    if receipt.project.is_empty() {
        return Err(invalid(
            "Choose a project explicitly before coordinator recovery",
        ));
    }
    let project = crate::coordinator::project(&receipt.project)?;
    let previous = {
        let _lifecycle =
            crate::lifecycle::LifecycleGuard::acquire().map_err(|e| invalid(&e.to_string()))?;
        crate::navigation::tmux(&["show-option", "-qv", "-t", &project, "@drudwyn_coordinator"])?
    };
    // An existing agent at the destination must be inspected, even when it is
    // unassociated or ambiguous. Recovery cannot multiply uncertain work.
    let workspaces = crate::discovery::discover_all_tmux().map_err(|e| invalid(&e.to_string()))?;
    for workspace in workspaces {
        if (workspace.is_agent() || workspace.process == "ambiguous")
            && workspace.process != "exited"
        {
            let checkout = crate::recovery::selected_checkout(
                &workspace.identity.window_id,
                &workspace.identity.pane_id,
            );
            if checkout.is_ok_and(|p| p == path) {
                return Err(invalid(
                    "Destination already has a live agent; Open/select its coordinator and inspect before deliberate Retry",
                ));
            }
        }
    }
    let command = crate::recovery::agent_command(agent)?;
    let started = crate::workspace::launch_existing(
        crate::workspace::Launch {
            path: path.clone(),
            repo: path.clone(),
            branch: receipt
                .target
                .reference
                .trim_start_matches("refs/heads/")
                .to_owned(),
            name: "Coordinator".into(),
            command,
            project: Some(project.clone()),
            batch: None,
            task_file: None,
            allocated_commit: None,
            track_agent: true,
        },
        Some(&guard),
    )?;
    if !matches!(
        receipt.state()?,
        ConflictState::Unmerged | ConflictState::Resolved
    ) {
        return Err(invalid(
            "Merge changed during recovery; new window retained; task not sent",
        ));
    }
    crate::coordinator::replace_after_conflict_recovery(&started.window_id, &project, &previous)?;
    crate::navigation::open_for(&client, Some(&started.window_id), Some(&project))?;
    Ok("Coordinator agent opened in destination; original window retained. New conversation; task not sent. Inspect startup, then deliberate Retry handoff".into())
}

/// Deliberate recovery of a live receipt's routing, never a guessed project.
pub fn select_conflict_project(path: &Path, project: &str) -> Result<(), Error> {
    let path = canonical(path)?;
    let guard = File::open(&path)?;
    guard.try_lock().map_err(|_| {
        invalid("Destination integration/recovery/delivery already in progress; retry explicitly")
    })?;
    let mut receipt = Conflict::load(&path)?;
    if !matches!(
        receipt.state()?,
        ConflictState::Unmerged | ConflictState::Resolved
    ) {
        return Err(invalid(
            "Merge already ended; no coordinator association changed",
        ));
    }
    let project = crate::coordinator::project(project)?;
    let repository = crate::navigation::tmux(&[
        "show-option",
        "-qv",
        "-t",
        &project,
        "@drudwyn_project_repo",
    ])?;
    if repository.is_empty() || canonical(Path::new(&repository))? != receipt.target.common {
        return Err(invalid(
            "Selected project is not associated with the destination repository; select its coordinator explicitly first",
        ));
    }
    receipt.project = project;
    receipt.save(&guard)
}

pub fn conflict(path: &Path, action: ConflictAction, redact: bool) -> Result<String, Error> {
    let path = canonical(path)?;
    let guard = File::open(&path)?;
    guard.try_lock().map_err(|_| invalid("Destination integration/recovery/delivery already in progress; inspect before retrying"))?;
    let receipt = Conflict::load(&path)?;
    let held = guard.metadata()?;
    if held.dev() != receipt.target.device || held.ino() != receipt.target.inode {
        return Err(invalid(
            "Conflict checkout changed while acquiring guard; nothing changed",
        ));
    }
    let state = receipt.state()?;
    if matches!(state, ConflictState::Completed | ConflictState::Aborted) {
        return Ok(receipt.display(state, redact));
    }
    match action {
        ConflictAction::Inspect => (),
        ConflictAction::Continue | ConflictAction::Abort => {
            if action == ConflictAction::Continue && state != ConflictState::Resolved {
                return Err(invalid(
                    "Unmerged entries remain; coordinator must resolve and stage them before Continue",
                ));
            }
            let args: &[&str] = if action == ConflictAction::Continue {
                &["-c", "core.editor=true", "merge", "--continue"]
            } else {
                &["merge", "--abort"]
            };
            // Last metadata check precedes mutation; Git owns its own locks.
            receipt.state()?;
            let status = command(Path::new("."), args)
                .current_dir(&path)
                .env("GIT_EDITOR", "true")
                .stdin(Stdio::from(guard.try_clone()?))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            if !status.success() {
                return Err(invalid(
                    "Git conflict action failed; resolution work retained; inspect destination. No reset or fallback attempted",
                ));
            }
        }
        ConflictAction::Retry => {
            receipt.handoff(&guard)?;
            let updated = Conflict::load(&path)?;
            return Ok(updated.display(updated.state()?, redact));
        }
        ConflictAction::Open => {
            let window = receipt.coordinator()?;
            crate::navigation::open(Some(&window), Some(&receipt.project))?;
        }
    }
    Ok(receipt.display(receipt.state()?, redact))
}
