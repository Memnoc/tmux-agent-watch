# 07 — Show evidence-backed activity and exit state

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done — queued hook lifetime P1 correction pending fresh independent review

**What to build:** Users can distinguish a running process, reported task
activity, attention requests, and an agent that exited, across current surfaces.

**Blocked by:** 01.
**Priority:** P1. **Stories:** 6, 7, 13, 16, 22.

- [x] A process without a supporting lifecycle event displays Running, not Working.
  Show evidence source and unknown state where attribution is unavailable.
- [x] Supported events produce Starting/Working/Needs input/Review/Failed
  consistently; Review does not imply passing checks or integration readiness.
- [x] Preserve inspectable exit code/time when available in live metadata;
  zero exit is distinct from task completion, and lost history remains unknown.
- [x] Attention survives inspection and unrelated scans; newer authoritative
  evidence supersedes it, and a replacement process cannot inherit stale evidence.
- [x] Split-pane/active-pane changes do not reclassify the wrong process or route
  events to a different worker; unknown multi-agent ownership remains explicit.
- [x] Existing pending lifecycle work is reviewed and covered by behavior tests;
  fixed metadata replaces content inspection and no inferred percent/ETA is added.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.


## Implementation receipt — 2026-09-30

The builder worked from `5b5183e` on `work/worktree-worker-workflow`. The
coordinating session will independently crosscheck this atomic ticket commit
against the originating spec and repository standards; no builder-owned child
review was substituted for that review.

Behavior now uses Running for process presence and Working only for supported
hook activity. Starting has launch evidence; Needs input/Review/Failed persist
across scans and UI inspection. Pane-local evidence binds to the pane root PID,
agent PID, and process birth time. A replacement, including a new child under
the same shell, cannot inherit the old event. All panes are reconciled once,
deduplicated across linked sessions, before projecting window state. Distinct
agents leave explicit ambiguous ownership. Discovery selects the bound activity
pane rather than the active split. CLI, Cockpit, navigator, HUD, sidebar, and
current bar use the corresponding activity and source semantics.

Process exit is separate from activity. Available tmux exit code, signal, and
time remain live metadata; zero does not mean completion. An unresolved hook
handoff survives exit, with nonzero exit also visibly Failed. Dead agents first
seen after exit have Unknown activity rather than an invented handoff. A missing
receipt stays unknown, and closing the pane/tmux session loses its history.

Managed starts use a fixed `sh -c` setup that enables pane-local remain-on-exit
before `exec "$@"`. Worker arguments remain separate argv entries, not generated
shell code; exec preserves the launch PID and exact executable arguments. The
wrapper has no task text, output parsing, persistent file, or background monitor.
Failure to set retention does not kill a worker that may write: initialization
still reports failure and preserves uncertain resources. Existing writer,
commit, metadata failure, uncertain response, collision, and provably-unused
allocation checks remain in the suite. Delivery's checkout-wide `File::try_lock`
and lossless launch checkout identity remain unchanged. On this tmux 3.4 host,
some signal-killed and immediately exited panes supplied neither code nor time; those fields are
explicitly unknown instead of fabricating shell-style `128 + signal` values.
A repeated early-exit probe reproduced missing native code/signal/time on attempt
14, still absent immediately and after 50 ms, 200 ms, and 1 s; repeated CLI scans
correctly remained Unknown. Immediate-exit tests poll briefly and assert the
native availability boundary, retain the pane/worktree, and never claim task
completion. Separate controlled delayed exits require known codes 0 and 23.

Red regressions observed before fixes: process-only Working; attention surviving
same-pane replacement; missing immediate-exit receipts; invisible first-scan
exited agents; linked pane duplicates misreported as multiple agents; and exit
codes clipped at 80 columns. New tests cover eleven public CLI/tmux scenarios,
including two actually attached grouped clients and subsequent real two-agent
ambiguity, real Cockpit/navigator keys, Starting, supported failure events,
shell-child replacement, early zero/nonzero exits, and handoff through exit.
Rendered-buffer checks cover activity/evidence/exit receipts at 80/120/160
columns and compact sidebar exit labels.

Existing navigation/settings fixtures had window-only lifecycle values on
ordinary shells. They now use absolute, resolution-checked fake executables and
real hook commands; navigation/settings assertions remain intact. No installed
agent is used. The legacy classifier's printed conversational lines are literal
preexisting strings in `tests/classify_test.sh`, not agent invocations.

A recurring ordinary-bar/new-session fixture failure was investigated using
only disposable pane path/process metadata. During zsh startup, observed paths
were repository → `/home` → repository, and `/home/memnoc` → repository, within
approximately the first 100 ms. Those fixture shells now use
`bash --noprofile --norc`; no production path logic or user shell configuration
was changed. The failed runs are not counted as passing validation.

Validation on final code: `cargo fmt --check` passed; `cargo test --locked`
passed all 41 Rust tests; `bash tests/run.sh` exited 0, including all 15 delivery
checks, 11 activity scenarios, 29 independent-navigation cases, 10 settings
cases, privacy, packaging, and release checks. The navigator suite retained its
one preexisting skip. Focused status-bar, lifecycle, start-failure, v2, activity,
navigation, settings, and render checks also passed. `git diff --check` passed.
Disposable tmux tests used authorized sandbox escalation; live user fixtures and
`main` were untouched. No Python cache or probe artifact is committed.

Crosscheck handoff: builder inspection found and corrected stale fixture
assumptions, linked-pane double counting, and clipped receipt presentation.
Independent Standards/Spec review by the coordinating session remains the next
step; this receipt does not claim that review has already occurred.

Limits: Linux/tmux 3.4 runtime was exercised; macOS was not. Executable/ancestry
observation is not agent readiness, acceptance, or task completion. Missing or
ambiguous attribution is not guessed. Global Cockpit, redesigned status bar,
recovery, and durable history are outside this ticket. `main` remains `eaf2446`.

## Independent review P1 correction — 2026-09-30

The independent review reproduced an initial scan paused after its pane
snapshot, followed by a completed `permissionRequest` hook. Resuming the scan
changed Needs input/hook to Running/process and erased `attention_since`.
`python3 /tmp/drudwyn-review07-race.py` reproduced that report, and the committed
public-CLI regression failed with `running != needs_input` before the fix.

Lifecycle snapshots, reconciliation, and window projection now share one
kernel advisory guard. Hooks and startup bookkeeping call reconciliation with
the guard already held, avoiding recursive acquisition; style refreshes use
the same guard. This closes both the stale-snapshot race and the remaining
read/write gap that a narrow reread would leave.

The guard resolves `#{socket_path}` through the selected tmux connection and
locks the existing parent-directory inode. It creates no file and reads no
directory contents. Servers whose sockets share a directory conservatively
share the lock. It waits up to five seconds, then fails explicitly with a retry
message without writing lifecycle evidence. It checks device/inode identity
after acquisition and fails closed if the directory changed. The descriptor
is close-on-exec; command error, unwind, or owner death releases it. A stopped
owner therefore cannot leave a tmux `wait-for` lock stranded indefinitely.
The caller/UI may wait during contention; the five-second bound covers lock
acquisition, not an independently stalled tmux subprocess.

Ticket 06's delivery lock remains separate. Static call-path inspection found
no lifecycle call under `deliver`'s checkout guard and no delivery call under
the lifecycle guard. A custom `tmux -S` socket directly in that checkout can
make the locks share an inode, causing bounded contention between independent
commands, without a synchronous nested acquisition in these paths.

A second red regression injected a tmux write failure during replacement
initialization and observed the replacement incorrectly inherit Review/hook.
Pane state and binding are now submitted in one synchronous tmux command queue,
with identity last, instead of publishing identity before separate state
commands. This is a single submitted command batch, not a claim of a database
transaction or atomicity across every external tmux/process mutation.

Seven new public-CLI scenarios cover initial-snapshot contention, replacement
contention paused at the write boundary, simultaneous scans and a hook,
owner death while a read subprocess remains alive, command-error release,
failed replacement publication, and bounded contention/retry. The replacement
case also confirms the next worker loses the previous worker's attention.
All original eleven activity/exit cases remain covered. Fake workers resolve
explicitly to `sleep`; fixture wrappers delay or fail real tmux boundary calls
without replacing pane, process, or Git behavior.

The first full-suite run stopped at a v2 fixture that changed its mode option
without reloading the previously installed v1 new-window observer. A focused
trace reproduced the legacy observer's asynchronous Working overwrite. The
fixture now exercises the supported v1-to-v2 plugin reload, asserts replacement
of that hook, then stops background observers for its explicit CLI assertions.
The focused v2 suite passed. Production compatibility behavior is unchanged;
the failed full run is not counted as validation.

Validation on final code: `cargo fmt --check` passed; `cargo test --locked`
passed all 41 Rust tests; the complete `bash tests/run.sh` rerun exited 0,
including 15 delivery cases, 18 activity cases, 29 independent-navigation
cases, 10 settings cases, privacy, packaging, and release checks. The navigator
suite retained its preexisting optional skip. Focused activity and v2 suites
also passed. `git diff --check` passed. No Python cache or probe artifact is
included. `main` remains `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.

Fresh independent Standards/Spec re-review remains required; this correction
does not mark crosscheck clear.
Runtime evidence remains Linux/tmux 3.4 only; macOS was not exercised.

## Independent re-review P1 correction — 2026-09-30

The next independent review reproduced a different crash boundary: pause an
already-spawned initial-scan mutation, SIGKILL its parent, complete a newer
`permissionRequest` hook, then release the orphaned mutation. The prior
close-on-exec guard released with the parent, and the orphan changed Needs
input/hook/nonempty attention to Running/process/empty attention.
`PYTHONDONTWRITEBYTECODE=1 python3 /tmp/drudwyn-review07-orphan.py` reproduced
those exact values. Two committed CLI regressions, for initial and replacement
pane writes, also failed before the correction with Needs input missing from
the resulting Running status. This supersedes the earlier receipt's claim
that releasing the lock on owner death was sufficient.

Every lifecycle mutation now explicitly receives the acquired guard. The
controlled `set-option` child receives a cloned `File` through Rust's safe
`Stdio` API as its unused stdin. That clone shares the existing kernel lock
and survives parent death until the child completes. The parent descriptor
remains close-on-exec; metadata reads and process scans do not inherit it.
There is no unsafe code, global descriptor-flag change, explicit unlock, new
file, persistent registry, or content inspection. Pane state, window projection,
style updates, and launch-stage bookkeeping all use this guarded mutation path.
All other tmux calls in this module only display/list/show metadata.

The behavior was checked against the actual supported runtime, tmux 3.4. Its
[client implementation](https://github.com/tmux/tmux/blob/3.4/client.c#L339-L416)
sends the command, retains stdin throughout its command loop, and returns after
the exit response. `client_send_identify` duplicates stdin for the server
(lines 444–446) without closing the client's descriptor. The
[server identify path](https://github.com/tmux/tmux/blob/3.4/server-client.c#L2880-L2934)
closes its nonterminal duplicate when terminal initialization fails; this is
not relied on to retain the lock. The controlled option commands do not consume
stdin. These source observations support the child-lifetime design; the tests
exercise real tmux completion and verify the lock is subsequently released.
The test timing wrapper uses a small shell bootstrap to preserve stdin while
starting Python, which otherwise refuses a directory stdin; it restores the
descriptor before pausing or executing the real tmux command.

The acquisition deadline remains five seconds. A permanently stalled mutation
child prevents newer updates from passing it, returning an explicit retry
error; the implementation does not trade correctness for a timed lock bypass.
The deadline bounds contention, not the execution time of an already-started
tmux command. Read-only orphan children still permit immediate recovery because
they cannot mutate state. Command failure before submission still releases the
guard. When the outstanding mutation finishes, a retry or waiting hook takes a
fresh snapshot and publishes newer evidence.

Five new scenarios cover orphaned initial-pane writes, replacement-pane writes,
window projection before a repair scan, bounded contention/retry while an
orphan is stalled, and an actual mutation exec failure before child startup.
The orphan race cases assert hook provenance and attention timestamps, then
replace the worker again and require Running/process with no inherited
attention. The existing surviving read-child and failed-submission cases remain
separate checks.

The complete `bash tests/run.sh` passed on the final production code, including
22 activity cases, 15 delivery cases, 29 independent-navigation cases, 10
settings cases, privacy, packaging, and release checks, with the existing
optional navigator skip. During that run, only the additive exec-failure test
and its fixture mode were added; the loaded activity suite was still the
22-case version. The final focused rerun passed all 23 activity cases.
`cargo fmt --check`, all 41 `cargo test --locked` tests, and
`git diff --check` passed. No Python cache or probe artifact is included.

Fresh independent Standards/Spec review is still required; this receipt does
not mark crosscheck clear. Runtime validation remains Linux/tmux 3.4 only;
macOS was not exercised. `main` remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.

## Independent re-review queued hook P1 correction — 2026-09-30

Review of `b39329d` confirmed the orphan-mutation correction, then found that
`hook()` first observed its worker after acquiring the lifecycle guard. A hook
waiting behind a paused scan could therefore bind itself to a replacement in
the same pane. The public CLI reproduction
`PYTHONDONTWRITEBYTECODE=1 python3 /tmp/drudwyn-review07-queued-replacement.py`
confirmed different original/replacement PIDs followed by Needs input/hook and
a nonempty attention timestamp on the replacement. The new deterministic
`Activity.test_queued_hook_rejects_same_pane_replacement` regression failed
before the fix because the queued hook returned success after replacement.
Rerunning the original script after the fix instead stopped at its old success
assertion with the explicit observed-agent-changed error; the regression now
asserts that rejection and the replacement remains Running/process.

Hooks now observe the pane root PID and unique matching agent PID/birth identity
before guard acquisition. After acquiring the guard, they observe the target
again and reject changed, unavailable, or ambiguous ownership before submitting
any lifecycle mutation. The previous lifecycle state used for timestamp updates
comes from the second, guarded observation. A matching worker still accepts the
queued event, preserving an existing attention timestamp when its state has not
changed. Revalidation and publication use the existing lifecycle guard and
child-held mutation lock; the process metadata boundary is unchanged.

This binds the event to the worker the CLI observed, not to proven caller
ancestry or an unobservable emission history. An externally invoked hook with a
pane environment remains an observation of that pane. Same-worker hooks still
apply in successful guard-acquisition order; the lock does not promise FIFO
invocation/observation order. No event sequence or chronology is invented by
this correction. The existing pane identity also ensures later discovery can
invalidate evidence after an external process change.

Five added command-boundary scenarios cover unchanged queued attention,
same-pane replacement, child replacement under an unchanged shell, exit while
queued, and ambiguous ownership appearing while queued. The child replacement
case also queues a valid event for the new child; the old event is rejected and
the new Working/hook evidence survives either acquisition order. The fixture
observes the real socket-resolution call to establish that the hook reached
guard acquisition before changing the worker, rather than assuming that a
sleep was long enough. All calls continue to real tmux; worker readiness uses
only process metadata and asserted fake sleep executables. Two initial fixture
names exceeded Unix socket limits and were shortened; readiness polling was
corrected to exclude exited zombie processes. Those failed fixture runs are
not counted as validation.

Validation on the frozen correction: focused activity coverage passed all 28
cases; `cargo fmt --check` and all 41 `cargo test --locked` tests passed; the
complete `bash tests/run.sh` exited 0, including all 28 activity cases, 15
delivery cases, 29 independent-navigation cases, 10 settings cases, privacy,
packaging, and release checks. The navigator retained its existing optional
skip. `git diff --check` passed, and no Python cache or probe artifact is
included. Runtime evidence remains Linux/tmux 3.4; macOS was not exercised.

Fresh independent Standards/Spec review remains required; no crosscheck
clearance is claimed. `main` remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.
