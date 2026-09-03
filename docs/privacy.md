# Privacy and local data flow

`tmux-agent-watch` is local-only supervisor software. The project operates no
backend and receives no workspace data. It has no accounts, telemetry,
analytics, crash uploader, update ping, remote feature flags, database, task
history, or content cache.

## Version boundary

The default Rust implementation (`@agent-watch-v2 on`) is the privacy boundary
described below. The public `v1.0.0` release and the explicit
`@agent-watch-v2 off` compatibility mode classify agents by reading up to 200
lines of pane scrollback with `tmux capture-pane`; they may keep a derived
summary in tmux window options for the current session. They do not send that
content to this project, but they are not content-blind and should not be used
for sensitive or organisational workflows without a separate assessment.

## Data used by the default Rust implementation

| Local datum | Immediate purpose | Lifetime | Surface |
|-------------|-------------------|----------|---------|
| tmux session, window, and pane IDs | route navigation and lifecycle updates | current command or tmux session | internal routing and click map |
| process executable name | identify Codex, Claude Code, or OpenCode | current scan | agent label |
| working directory, Git repository, worktree, and branch | identify workspaces and enforce safe start/finish | current command or tmux session | cockpit/sidebar unless redacted |
| clean/dirty state and merge ancestry | prevent unsafe worktree removal | current command | fixed readiness state |
| lifecycle state and timestamps | show working, waiting, review, or failed state | current tmux session | cockpit, HUD, and sidebar |
| task entered in the start form | deliver the initial instruction to the selected agent | input event and delete-on-paste tmux buffer | selected third-party agent pane |

The default Rust implementation does not read terminal scrollback, prompts,
responses, permission text,
clipboard content, file content, diffs, commit bodies, or environment-variable
values. Task text is sent through stdin to a uniquely named tmux buffer, pasted
with delete-on-paste, and is never placed in arguments, options, logs, or files.

Set `@agent-watch-redact-labels on` before screen sharing to replace repository,
branch, session, and window labels while preserving lifecycle state and click
navigation.

Codex, Claude Code, OpenCode, Git hosts, package registries, and download tools
are separate products with their own data practices. Launching an agent can
send the task to services configured by that agent; `tmux-agent-watch` neither
controls nor receives that traffic.

Organisational operators should treat workspace labels and activity state as
potential personal data and assess access, employee notice or consultation,
lawful basis, and any DPIA requirements for their own deployment. Employee
scoring, performance monitoring, and decisions about people are outside this
project's intended purpose.
