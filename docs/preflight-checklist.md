# Internal release preflight

Run this checklist against a manual **Release** workflow before pushing a
version tag. A manual run builds and verifies all four supported artifacts but
cannot publish a GitHub Release.

## Candidate

- Commit:
- Cargo version:
- Workflow run:
- Tester and date:
- Platforms exercised:

## Artifact gate

1. Run the Release workflow with **Run workflow** on the candidate commit.
2. Confirm all four native build jobs and **Verify release bundle** pass.
3. Download `release-bundle` from the workflow run.
4. Run `sha256sum --check SHA256SUMS` on Linux or
   `shasum -a 256 --check SHA256SUMS` on macOS.
5. Install the archive matching each available test machine through
   `install.sh` using `TMUX_AGENT_WATCH_BASE_URL` pointed at a local HTTP or
   file URL. Confirm `tmux-agent-watch --version` matches `Cargo.toml`.

## Fresh-install acceptance

- Start a clean tmux server with no `@agent-watch-v2` option and reload the
  plugin. Confirm the Rust cockpit opens with `prefix + P`.
- Confirm the clustered status bar separates workspaces and agents, renders Git
  context, and shows overflow without creating project-controlled state.
  Confirm no sidebar pane is created by default.
- Open the grouped navigator with `prefix + w`; exercise both sections,
  filtering, jumping across sessions, and the native `prefix + C-w` fallback.
- Exercise both safe and Nerd Font icon modes and all three Rose Pine variants.
- Start one workspace with each locally installed agent: Codex, Claude Code,
  and OpenCode. Confirm the selected agent receives its initial task once.
- Exercise working, waiting, review, failed, refresh, filter, jump, and close.
- Finish a clean merged worktree. Confirm dirty and unmerged worktrees are
  refused and the retained branch remains present.

## Migration and recovery

- Upgrade an existing v1 tmux configuration without adding
  `@agent-watch-v2`; confirm v2 becomes active after reload.
- Set `@agent-watch-v2 off`, reload, and confirm the legacy cockpit, scanner,
  hooks, HUD, optional sidebar, and workspace commands remain usable.
- Test a missing binary, invalid agent option, invalid branch prefix, stale
  sidebar pane, killed agent pane, tmux client reconnect, and plugin reload.
  Each failure must remain bounded and explain the recovery action.

## Privacy acceptance

- Enter a distinctive private task and confirm it is absent from tmux options,
  buffers, server environment, process arguments, and project files after
  delivery.
- Enable `@agent-watch-redact-labels on`; confirm cockpit and start/finish forms
  hide task, repository, branch, session, and window labels while navigation
  and content-blind status signals still work.
- Confirm no unexpected network connection, telemetry, log, database, history,
  or crash artifact is created by the supervisor.

## Decision

- [ ] Artifact gate passed
- [ ] Fresh-install acceptance passed
- [ ] Migration and recovery passed
- [ ] Privacy acceptance passed
- [ ] Known issues are documented and accepted or fixed
- [ ] Approved to push the matching `v<version>` tag

Record failures with the platform, exact artifact checksum, reproduction, and
whether they block publication. Do not push the version tag while any required
box remains unchecked.
