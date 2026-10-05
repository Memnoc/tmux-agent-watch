# Documentation

[Project home](../README.md)

## Start here

| I want to… | Read |
| --- | --- |
| Migrate from tmux-agent-watch without losing settings | [Upgrade to Drudwyn](upgrading-to-drudwyn.md) |
| Install the plugin or update an existing checkout | [Installation and updates](installation.md) |
| Navigate agents, manage worktrees, or save sessions | [Daily use](usage.md) |
| Start from one branch and merge into another | [Choose starting and merge branches](usage.md#choose-starting-and-merge-branches) |
| Connect agent lifecycle hooks | [Agent integrations](agents.md) |
| Change themes, shortcuts, or defaults | [Configuration](configuration.md) |
| Fix a missing binary, stale state, or shortcut conflict | [Troubleshooting](troubleshooting.md) |
| Understand what data the plugin uses | [Privacy and local data flow](privacy.md) |

For a first run, follow the [README quickstart](../README.md#install), open the
cockpit with `prefix + P`, then try the workspace navigator with `prefix + w`.

## Development and project background

- [Update agenda](agenda.md): work planned for the next update.
- [Worker workflow specification](specs/2026-09-30-worktree-worker-workflow.md) and [implementation breakdown](specs/2026-09-30-worktree-worker-ticket-plan.md): the planned launch, supervision, recovery, and integration improvements.
- [README standard](readme-style.md): reusable layout and authoring skill.
- [Contributing](../CONTRIBUTING.md): setup, repository map, and checks.
- [Domain context](../CONTEXT.md): workspace and lifecycle terminology.
- [Architecture decisions](adr/README.md): why the project is structured this way.
- [Release preflight](preflight-checklist.md): build and acceptance checklist.
- [Compliance record](compliance/tmux-drudwyn-v2.md) and [research notes](research/): supporting project records.
