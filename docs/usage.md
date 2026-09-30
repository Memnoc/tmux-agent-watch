# Daily use

[Documentation](README.md) · [Project home](../README.md)

In this guide, `prefix` means your tmux prefix key (normally `Ctrl+b`). Press it, release it, then press the next key. Uppercase letters use Shift; `C-s` means `Ctrl+s`.

## Find your workspace

See what is running, what needs attention, and where to act without leaving tmux.

| Surface             | What it answers                               | Open it        |
| ------------------- | --------------------------------------------- | -------------- |
| Status bar          | Where am I, what changed, and who needs me?   | Always visible |
| Workspace Navigator | What is running across all tmux sessions?     | `prefix + w`   |
| Workspace Cockpit   | What should I review, open, start, or finish? | `prefix + P`   |

### Status at a glance

<img src="images/design/status-bar-anatomy.png" alt="Annotated tmux status bar split into three zones: ordinary workspaces on the left, content-blind Git context in the centre, and lifecycle-colored agents on the right, with a variant showing the active-workspace highlight">

| Color | State   | Meaning                  |
| ----- | ------- | ------------------------ |
| Blue  | Working | The agent is active      |
| Gold  | Waiting | The agent needs input    |
| Green | Review  | Work is ready to inspect |
| Red   | Failed  | The agent or task failed |

Only lifecycle state, branch, and Git counts are shown. The default Rust
implementation never reads prompts, responses, or terminal scrollback.

### Find work, then act

Use the session navigator to find a session, the workspace navigator to find a window, and the cockpit to act on a workspace.

<img src="images/navigator-cockpit-surfaces.png" alt="Comparison of three complementary tmux-drudwyn surfaces: the Session Navigator for switching tmux sessions, the Workspace Navigator for finding windows and agents across sessions, and the Workspace Cockpit for starting, reviewing, opening, and finishing agent work">

| Surface             | What you can do                                      | Use it when                                    |
| ------------------- | ---------------------------------------------------- | ---------------------------------------------- |
| Session Navigator   | Find and switch tmux sessions                        | You know which session you want                |
| Workspace Navigator | Find any window or active agent across every session | You need to locate a workspace or agent        |
| Workspace Cockpit   | Start, review, open, and finish agent work           | You need context or want to act on a workspace |

| Cockpit action | Result                                                         | Direct key   |
| -------------- | -------------------------------------------------------------- | ------------ |
| Start          | Create a linked worktree, choose an agent, and begin the task  | `prefix + W` |
| Review         | Show agents waiting, failed, or ready for review               | —            |
| Jump           | Open a live agent workspace                                    | `prefix + w` |
| Finish         | Remove a clean, integrated worktree while retaining its branch | `prefix + X` |

<img src="images/design/lifecycle-flow.png" alt="Lifecycle flow diagram: start task creates a worktree, the agent moves to working, then needs attention when waiting or failed, then review when idle and ready, then finish safely once clean and merged">

Finish refuses primary checkouts, dirty worktrees, and branches not integrated
into the configured base branch.

### Responsive layout

The status bar adapts to narrower terminals and tiled windows.

<img src="images/design/responsive-layouts.png" alt="The status bar at three widths: wide showing every workspace and agent, narrow collapsing inactive workspace labels to an arrow, and compact under 80 columns sharing project, Git, lifecycle, and navigator cues in one flow">

Workspace, Git, lifecycle, and navigation context remain visible as the
terminal narrows; labels collapse before information collides.

## Keyboard reference

| Key              | Action                                         |
| ---------------- | ---------------------------------------------- |
| `prefix + H`     | Open help (`q` or Escape closes it)            |
| `prefix + P`     | Open the Workspace Cockpit                     |
| `prefix + w`     | Open the grouped workspace navigator           |
| `prefix + C-w`   | Open tmux's native window tree                 |
| `prefix + s`     | Open the compact session navigator             |
| `prefix + S`     | Open tmux's native session tree                |
| `prefix + C-s`   | Save all sessions (tmux-resurrect)              |
| `prefix + a`     | Jump to the oldest agent needing attention     |
| `prefix + W`     | Create a worktree and start an agent           |
| `prefix + X`     | Finish the selected clean, integrated worktree |
| `prefix + Space` | Toggle the optional legacy sidebar             |
| `prefix + A`     | Recreate a stuck legacy sidebar                |

### Independent terminal views

Drudwyn's navigators, Cockpit Open, status-tab clicks, sidebar clicks, and
attention shortcut route to the terminal that invoked them. When another client
is attached to the destination session, Drudwyn creates a grouped tmux session
view with independent window selection. The windows, panes, worker processes,
files, and branches remain shared; worker totals count each window once.

Views are created only when needed and reused while that client stays in the
project. Drudwyn assigns an internal view name; `@drudwyn_view_of` identifies
its original session by ID. Drudwyn shows the shared project name and hides these extra views
from its session list. User-created grouped sessions remain visible and are
never treated as disposable views. Switching away or detaching removes an unused
application view through tmux's `destroy-unattached` option. Other sessions and
clients retain their linked windows. The original project session is not removed.
If you deliberately kill that original session, remaining attached views keep
its windows until their last session closes.

Native tmux navigation still follows tmux's normal session semantics: clients
attached to the same session share selection. The first Drudwyn navigation
separates their selections when necessary. This does not give separate pane
layouts or independent input into the same worker process.

For scripts, use stable IDs and an explicit client (listed by `tmux list-clients
-F '#{client_name}'`):

```sh
tmux-drudwyn navigate --client /dev/pts/3 --window '@12'
tmux-drudwyn navigate --client /dev/pts/3 --session '$2'
```

`DRUDWYN_CLIENT` is also accepted. Without an explicit client, Drudwyn proceeds
only if the invoking pane's memberships identify exactly one attached client
(or only one client exists when called outside a pane). Ambiguous or detached
clients and vanished targets produce errors without choosing a replacement by
window index. You can supply both `--session` and `--window` to require a specific
membership; otherwise navigation prefers the requesting client's membership,
then an original session over application views.

Inside either navigator, these shortcuts act on the selected item:

| Key | Action |
| --- | --- |
| `j` / `k` or arrow keys | Move selection |
| `Enter` | Switch to the selected session or window |
| `/` | Filter the list |
| `n` | New shell session (session navigator only) |
| `r` | Rename the selected session (`prefix + s`) or window (`prefix + w`) |
| `x` | Kill the selected item after confirmation |
| `s` | Save all sessions with tmux-resurrect |
| `Esc` / `q` | Close the navigator |

Renaming starts with the current name. Use `Backspace` to delete, `Enter` to
apply, and `Esc` to cancel. The navigator stays open and reports any error so
you can correct the name. Press `s` afterward to save the updated layout with
Resurrect.

In the session navigator, `n` opens **New Session**. Enter a name and directory;
the directory starts at the requesting terminal's selected workspace. `Tab`
changes fields, `Enter` advances from the name or creates from the directory,
`Backspace` deletes, and `Esc` cancels. Errors retain the form for correction.
The shell opens only in the requesting terminal and appears in both navigators.
Names cannot be empty or contain dots, colons, control characters, or `␟`;
spaces and shell metacharacters are accepted as literal data.

The command equivalent prints the new stable session ID:

```sh
tmux-drudwyn session new --name 'Editing notes' --directory '/path/with spaces'
```

Omit `--directory` to use the invoking workspace. An explicit relative directory
is resolved from the command's working directory. When more than one terminal
could be the requester, supply `--client CLIENT` or `DRUDWYN_CLIENT`. Missing or
ambiguous clients fail before creation. New Session uses tmux's configured shell,
bypassing `default-command`, and does not create a worktree or change a branch.
Creation and switching failures preserve existing sessions. If a newly created
shell cannot be opened, Drudwyn removes that new session and allows retry; a
failed cleanup or uncertain creation response reports that the new session may
remain for inspection in the navigator.

Inside either navigator, select an item and press `x`, then `y` to confirm killing
it (`Esc` or `n` cancels). In `prefix + w`, this kills the selected tmux window
and all its panes, including any links to that window in other sessions. In
`prefix + s`, it kills the selected session; windows linked to another session
survive. Running processes in closed panes stop. Git worktrees and files remain
on disk. The list refreshes after a kill; killing the session hosting the popup
may close it and detach its clients.

Existing tmux window navigation, naming, and pane zoom continue to work.

## Worktrees from the command line

Use the cockpit for everyday work. For automation, run these scripts from the
plugin checkout directory:

```sh
scripts/worktree-new.sh feature/auth opencode
scripts/worktree-new.sh --repo /path/to/repository feature/auth opencode
scripts/worktree-remove.sh feature/auth
```

Extra arguments after the branch are used as the exact agent command. Set
`DRUDWYN_WORKTREE_ROOT` to change the worktree parent directory. Removal
refuses dirty worktrees and retains the branch.

## Session persistence

Live state is stored in tmux window options. Use `tmux-resurrect` and
`tmux-continuum` to restore sessions, windows, layouts, and working directories.
Load Continuum after themes that replace `status-right`.

Press `s` inside either navigator to save **all sessions** through the installed
Resurrect plugin without closing the navigator. The footer reports the result.
After cleaning up windows or sessions, save before exiting tmux so the next
restore uses the updated layout. Kills do not automatically save; if killing the
session hosting the popup closes it, reopen a navigator in a remaining session
and save there.

Resurrect's default global save shortcut is `prefix + Ctrl+s`. Drudwyn leaves
that key available and puts the native session tree on `prefix + Shift+s`.
If upgrading from a version that used `Ctrl+s` for the native tree, reload your
tmux configuration with Resurrect enabled to restore its save binding. Remove
any explicit `@drudwyn-native-session-key C-s` override that would reclaim it.

Saving requires `tmux-resurrect` to be loaded. Drudwyn delegates to its
`@resurrect-save-script-path` and creates no separate snapshot. Resurrect controls
what is saved, including pane contents if you enabled its capture option.

### Choosing a task base (Rust v2)

New workspaces start from `@drudwyn-base-branch` (default `main`), even when
launched inside another task's worktree. Drudwyn prefers that branch's locally
available upstream ref, then `origin/<base>`, then the local base branch. If none
exists, creation stops with an error instead of inheriting the current branch.

Press `prefix + W` (or `n` in the cockpit) to open the start form, which shows
the resolved ref and commit. Press **F2** to choose
**Continue from current branch** when the new task intentionally depends on the
current checkout. The displayed commit is the starting point used at creation.

Refs are read locally; remote freshness is unknown. Run `git fetch <remote>`
before opening the form when you need the latest remote commits.

```sh
scripts/worktree-new.sh --base main feature/independent codex
scripts/worktree-new.sh --from-current feature/dependent codex
```

Without either flag, the CLI uses the configured base. These choices apply to
the default Rust implementation; the legacy fallback retains its existing behavior.

### Failed worker starts (Rust v2)

A failed start reports the retained worktree path and branch. Once tmux may have
started a worker, Drudwyn preserves its checkout, commits, untracked files, and
any surviving window, even if attaching workspace metadata fails. Inspect the
reported window when it still exists, or open the retained directory in a shell.
The launch error in Cockpit wraps to show recovery details; label redaction hides
them until you disable it.

Retrying the same branch or path reports a collision without overwriting work
or starting another worker. To launch a separate worker, choose a new branch and
path. Drudwyn only removes a clean, unchanged allocation automatically when the
tmux executable could not start at all. An immediate process exit, including
exit code zero, is a failed launch rather than evidence of task completion.

### Project coordinator and worker names (Rust v2)

Explicitly choose an existing shell or agent window as the project's coordinator.
Use the stable window ID shown in the workspace navigator:

```sh
tmux-drudwyn coordinator set --window @12
tmux-drudwyn workspace start --repo . --name api work/api codex
tmux-drudwyn coordinator open
```

The commands infer the requesting terminal when unambiguous; pass `--client`
when needed. `coordinator set` and `coordinator open` also accept `--session $ID`
for an explicit project session. Quote a literal session ID, for example
`--session '$3'`. Coordinator association preserves its checkout branch and
current window name. The window can host a shell or an agent; coordinator shells
remain reachable without inflating live-agent totals.

New workers launched from that project inherit its explicit association only
when their Git common directory matches. Linked checkouts share that identity;
separate repositories with the same display name do not. This identifies a
project and coordinator, not a worker batch or integration destination. Existing
external agents keep an unknown association until explicitly selected as a
coordinator; sharing a repository alone does not assign them to a project.

Press `c` in the workspace navigator or Cockpit to return to the selected
workspace's coordinator, or in the session navigator to open the selected
project's coordinator. Navigation moves only the requesting client. Workspace
rows show text roles and stable window IDs; Cockpit details show the project ID,
coordinator availability, and unknown associations. Redaction hides labels while
retaining IDs and roles for navigation.

`workspace start --name` supplies a deliberate short window name; omitting it
uses the branch. Managed windows disable process-driven automatic/escape-sequence
renaming. Explicit later renames through tmux or the navigator remain authoritative.
The coordinator's existing name is preserved when it is associated.

If the coordinator disappears, return reports that it is unavailable instead of
selecting a window that reused its name or index. Open a shell in the project
checkout, then explicitly select it with `coordinator set --window ID` to recover
the route. This does not restore an agent conversation. Associations live only
in tmux and must be selected again after that metadata is lost.
