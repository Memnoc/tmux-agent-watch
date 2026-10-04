# Privacy and local data flow

`tmux-drudwyn` is local-only supervisor software. The project operates no
backend and receives no workspace data. It has no accounts, telemetry,
analytics, crash uploader, update ping, remote feature flags, database, task
history, or content cache.

## Version boundary

The default Rust implementation (`@drudwyn-v2 on`) is the privacy boundary
described below. The public `v1.0.0` release and the explicit
`@drudwyn-v2 off` compatibility mode classify agents by reading up to 200
lines of pane scrollback with `tmux capture-pane`; they may keep a derived
summary in tmux window options for the current session. They do not send that
content to this project, but they are not content-blind and should not be used
for sensitive or organisational workflows without a separate assessment.

## Data used by the default Rust implementation

| Local datum | Immediate purpose | Lifetime | Surface |
|-------------|-------------------|----------|---------|
| tmux session, window, and pane IDs, explicit project/coordinator association | route navigation and lifecycle updates | current command or tmux session | internal routing and click map |
| tmux socket path and socket-directory inode | serialize lifecycle snapshots and updates | current command and outstanding mutation child | internal synchronization |
| process executable name, UID, PID, parent PID, process birth time/state, and derived agent kind | identify Codex, Claude Code, or OpenCode and bind evidence to its lifetime | current scan or tmux session | agent symbol and label |
| working directory, Git repository/common directory, worktree, and branch | identify workspaces and enforce safe start/finish | current command or tmux session | cockpit/sidebar unless redacted |
| batch ID, pinned source ref/commit, destination branch/starting commit/checkout | keep sibling launches and integration choices explicit | current tmux session only | batch commands and Cockpit unless redacted |
| clean/dirty state, refs/commit IDs, changed-file names and merge ancestry | preview/integrate reviewed commits and prevent unsafe worktree removal | current command | fixed readiness state |
| lifecycle state, evidence source, timestamps, and available pane exit code/signal/time | distinguish running, working, attention, and process exit | current tmux session | cockpit, HUD, and sidebar |
| launch pane/process identity, losslessly encoded checkout path, expected executable and delivery state | bind and serialize a send to its worker and distinguish not sent/sent/uncertain | current tmux session | workspace commands and Cockpit |
| verification check identity, canonical checkout/inode fingerprint, tested revision, process birth identities, timestamps, exit and stale state | reconcile explicit assembled checks | current tmux server only | verification command and Cockpit, labels redacted |
| expected merge refs/commits, destination directory identity, MERGE_HEAD inode/timestamps, project and handoff state | guard Continue/Abort and reconcile external resolution | current tmux server only | conflict command and Cockpit, labels redacted |
| deliberately selected repository task-file reference (including lossless path encoding) | let the agent read the chosen task in its checkout | current tmux session | launch form and live window option |
| terminal unread-input byte count (Linux) | wait for the editor to consume a paste before one submit | delivery call only; not retained | internal transport metadata |
| task entered in the start form or stdin | deliver the initial instruction to the selected agent | form/command memory and delete-on-paste tmux buffer | selected third-party agent pane |

The default Rust implementation does not read terminal scrollback, prompts,
responses, permission text,
clipboard content, file content, diffs, commit bodies, or environment-variable
values. Task text is sent through stdin to a uniquely named tmux buffer, pasted
with delete-on-paste, and is never placed in arguments, options, logs, or files. Buffers are explicitly
removed on failure as well as successful paste. File-reference validation reads
Git tree/path metadata only; Drudwyn never reads the referenced task file. A sent
receipt does not establish agent acceptance or task completion.
Recovery enumerates Git worktree metadata only for the selected repository. It
reads pane working-directory metadata (including `/proc/PID/cwd` on Linux) and
retains only a lossless checkout/reference identity in live tmux options. Shell
recovery carries no agent lifecycle or task-delivery binding. It creates no
worktree, prompt history, conversation archive or repair state.

Concurrent recovery and delivery use a kernel advisory lock on the existing checkout
directory for the command's lifetime. It creates no lock file or durable registry
and reads no directory or task-file content.
Lifecycle updates likewise use a kernel advisory lock on the existing tmux
socket directory. They read no directory contents and create no lock file.
Servers sharing that directory serialize conservatively. Contention waits at
most five seconds before returning an explicit retry error. Only lifecycle and coordinator-association
mutation children inherit a duplicate of the lock descriptor, through their
unused stdin; read commands and agent processes do not inherit it. The lock
releases after the parent and any outstanding mutation child finish, including
after errors. Killing the parent cannot allow an older surviving write to
overwrite a newer hook. A stalled child can delay updates, which still return
the bounded retry error rather than bypassing the lock.

Coordinator assignment uses that same guard. Recovery snapshots missing ownership,
releases the guard during process creation, then compares ownership again under
the guard before assignment. A deliberate selection or another recovery that
wins meanwhile remains selected; the losing recovery retains its window and
checkout with an explicit conflict. Recovery takes the checkout guard before
the socket guard; coordinator setters never acquire checkout guards. No tmux
wait-for lock or persistent reservation is created.

Integration previews keep only source/target refs, commits, canonical checkout
identities and file-name metadata in command/UI memory. The displayed review
token is an ephemeral comparison of those facts, not a stored history or a
reservation. Integration ignores inherited Git repository/index-addressing
variables without reading their values. Destination and source Git metadata use
the same explicit-checkout command seam as inventory enrichment. Mutating Git
and hooks may operate on repository files as normal, but their output is not
captured, interpreted or retained by the supervisor.

Before mutation, ignored/untracked filename metadata is compared against incoming
source changes from merge-base metadata, including file/directory path collisions.
Deletion and addition/change names conservatively identify possible directory
relocations in both directions; no rename similarity calculation or merge preview
is invoked. Root is never a relocation source, but a directory can be flattened
into it. A possible relocated collision is refused before mutation. No ignored
file contents are read. Unrelated ignored files outside these candidate paths and
unchanged source paths deleted at the destination do not block integration.

Apply serializes on the existing canonical destination-directory inode, sharing
the checkout lock namespace with recovery and task delivery. Competing actions
return an explicit retry error. The mutating Git process inherits the descriptor
through unused stdin and starts in the selected directory; process death does
not release the guard while the child remains able to mutate. Git provides its
normal hook stdin. No global lifecycle guard is held during Git or hooks, and
no lock file, durable merge registry or task history is created. Failed/conflicting
merges remain in Git for deliberate recovery; nothing is automatically reset.

Conflict receipts retain metadata only in server options; they contain no task or
resolution text. A generated coordinator instruction exists only in command
memory and the same delete-on-paste transient buffer as worker tasks. The send
verifies a unique agent's pane, process birth identity, and actual destination
checkout before paste and before Enter. This scoped binding does not create or
relax managed worker launch receipts. Uncertainty is recorded before transmission;
only deliberate Retry can send again. An ordinary coordinator can receive this
handoff without being relaunched as a managed worker.

Continue/Abort share the destination guard and retain it in the mutating Git
child; normal hooks run with output discarded. Continue uses a noninteractive Git
editor. Replaced merge metadata cannot be adopted, and loss of live receipt
history leaves the operation unavailable to these controls. Explicit conflict
agent recovery starts a new conversation without a task, retains the old window,
and compares coordinator ownership under the existing lifecycle guard before
selection. The user then deliberately retries the generated handoff. No agent
acceptance, resolution, or assembled verification is inferred from transmission.

Assembled verification accepts user-selected commands only in form memory or
stdin. A noninteractive Bash process receives them through an anonymous pipe;
Drudwyn neither captures nor interprets its inherited terminal output. It creates
no script file, history entry or log. The selected script is never placed in
Drudwyn's or the shell runner's argv or tmux options; tools invoked by that script
control their own arguments and output.
The latest receipt lives in one server option per canonical checkout, with only
check identity, checkout/revision, timestamp/exit state, runner process lifetime,
and an opaque hash of Git/path/stat metadata. File contents are never hashed.
Linux runner identity uses `/proc/PID/stat` process birth ticks and exit state;
other Unix platforms use process birth metadata from `ps`.

The check inherits an advisory lock on the existing destination directory through
a separate descriptor while stdin carries the script. Child setup uses the held
directory descriptor as its working directory. Its foreground process tree can
therefore retain the guard after runner death, without a lock file or registry.
Inspection reads tracked/untracked filenames and inode, mode, size, modification
and change timestamps; ignored contents and external environment are not certified.
Receipt comparison and mutation use the existing bounded lifecycle guard; Git/stat
observation occurs outside it and stale writes are rejected on comparison. Inventory
shares one verification observation per unique destination per refresh and skips
file enumeration when there is no receipt. Losing metadata never restores success
from Git ancestry or agent activity. Selected commands and third-party tools remain
responsible for their own effects, files, output and data practices.

Global Cockpit refreshes keep a single in-memory snapshot of known tmux windows,
projects, process evidence, Git checkout identity/status, changed-file names and
live batch/task-reference metadata. Git status uses NUL-separated names; no diff
bodies or file contents are read. Each unique checkout and live batch is probed
once per refresh, then shared by filtering, grouping, counts and details.
Current batch destinations and source-commit containment are also shared within
each refresh; the UI never probes ancestry while rendering or from status cells. The
interactive refresh thread creates no persistent cache; failed snapshots remain
visibly stale and cannot authorize an action. Exiting the popup discards the
snapshot. Lifecycle window projections use a single guarded tmux command queue,
retaining the existing bounded lock and mutation-child lifetime guarantees.
If a sampled live pane's root is missing from the subsequent process observation,
reconciliation retries both observations before any writes. Persistent uncertainty
returns an error without clearing evidence or publishing a fresh scan timestamp.

Both status layouts read the observer's existing content-blind projection. Successful
reconciliation publishes only a live scan timestamp under the existing guard;
it is not a history or file cache. Ambient jobs do not start process scans or
probe Git per tab. The selected bound checkout supplies branch metadata and,
in Focus, tracked-line totals against HEAD. Git produces short statistics only;
external diff and text conversion are disabled, inherited Git namespace
variables are cleared, and no diff content is displayed or retained.
Each row uses explicit client width and stable session/window IDs, with no
shared mutable rendered-row cache. Click ranges contain fixed action names or
stable IDs; no label becomes a command. Stale observation timestamps are shown
explicitly, and action resolution validates the requesting client and target.

Set `@drudwyn-redact-labels on` before screen sharing to replace repository,
branch, session, and window labels while preserving lifecycle state and click
navigation.

Codex, Claude Code, OpenCode, Git hosts, package registries, and download tools
are separate products with their own data practices. Launching an agent can
send the task to services configured by that agent; `tmux-drudwyn` neither
controls nor receives that traffic.

The explicit `s` action in either navigator invokes the locally installed
`tmux-resurrect` save script. Resurrect owns the saved snapshot and follows its
existing configuration, including optional terminal-content capture. Drudwyn
does not read the snapshot or save-script output, and the save action adds no
separate content store. This external persistence is distinct from the live,
content-blind supervision described above.

Organisational operators should treat workspace labels and activity state as
potential personal data and assess access, employee notice or consultation,
lawful basis, and any DPIA requirements for their own deployment. Employee
scoring, performance monitoring, and decisions about people are outside this
project's intended purpose.


Explicit promotion shares integration's content-blind preview, mutation guard,
conflict receipt and verification boundaries. It creates no promotion registry
and copies no verification evidence between checkouts.

Finish keeps only an ephemeral comparison token for the selected source/destination
refs, commits and directory identities. It observes process UID/executable, ancestry,
birth and cwd metadata to veto known writers; Linux uses `/proc`, with metadata-only
`lsof -a -u UID -d cwd -F0pn` on hosts without it. It never requests process argv,
environment values, open-file contents or terminal content. Observation covers
associated pane/process descendants and readable outside cwd metadata; unrelated
protected process cwd and unobserved external changes are not certified. Only the current synchronous invocation's shell ancestry is exempt; other source
shells, including builtin-only waiting loops, remain potential writers. Calling
agents/editors and background children are not exempt. No process is stopped as a
precondition of removal. Tracked/untracked/ignored filename metadata and Git
operations are checked without file contents.

Cleanup locks the existing source and destination (or Git common directory when
the destination branch has no checkout). The Git removal child inherits both
descriptors, so parent death cannot release an ongoing removal. After successful
Git removal, the existing lifecycle guard serializes coordinator-role checks and
window closure; it is never held during Git. Closure rechecks stable pane IDs,
process births and cwd inodes, and a tmux-side pane-set comparison prevents a newly
split/replaced pane being swept into the removal. No branch deletion, force
removal, durable receipt, push or deployment is added.

Startup additionally observes terminal driver flags (canonical input and echo), never screen text. The Codex adapter verifies its terminal ancestor and that process's cwd, rechecks identity under the lifecycle guard, and never parses hook payloads. Shared-daemon origins cannot prove terminal ownership and are rejected; managed default Codex workers use --no-daemon. Runtime snapshots contain application code/assets only. Coordinator return bookmarks are non-content window IDs scoped to the live tmux session and requesting client.
