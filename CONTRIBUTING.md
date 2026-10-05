# Contributing

[Project home](README.md) · [Documentation](docs/README.md)

## Report a bug or suggest a change

Use [GitHub issues](https://github.com/Memnoc/tmux-drudwyn/issues). For a bug,
include your OS, `tmux -V`, the plugin commit (`git rev-parse --short HEAD`), install
method, steps to reproduce, and expected versus actual behavior. Add relevant
configuration or a screenshot when useful; remove private task and workspace details.

For a feature, describe the workflow you want to improve and what the user should
be able to do. For a pull request, explain the resulting behavior and how you
verified it.

## Set up development

You need Rust and Cargo, Bash, Git, tmux, Python 3, and ripgrep (`rg`). The shell suite also uses
standard Unix tools and the release installer's `curl`, `tar`, and checksum tools.
Some shell checks use GNU utility options; Linux is the straightforward environment
for the full suite.

```sh
git clone https://github.com/Memnoc/tmux-drudwyn.git
cd tmux-drudwyn
cargo build --locked
cargo test --locked
```

The first build downloads dependencies. Several shell tests build with Cargo's
`--offline` option, so complete that initial build before running them.

## Repository map

| Path | Purpose |
| --- | --- |
| `src/` | Rust domain model, discovery, lifecycle, workspace actions, and terminal UI |
| `scripts/` | tmux wiring, agent hooks, worktree commands, and legacy implementation |
| `integrations/` | Optional OpenCode event integration |
| `tests/` | Shell integration checks and navigator keyboard tests |
| `tmux-drudwyn.tmux` | Plugin entrypoint and key bindings |
| `install.sh` | Release download and checksum verification |
| `tooling/skills/` | Reusable repository-authoring skills |
| `docs/` | User guides and project background |
| `.github/workflows/` | Release builds, verification, and publication |

Read [domain context](CONTEXT.md) and [architecture decisions](docs/adr/README.md)
when changing workspace behavior. Preserve the documented [data boundary](docs/privacy.md)
and the checks that prevent removing dirty or unintegrated worktrees.

## Verify changes

Run checks relevant to the behavior you changed. Rust checks:

```sh
cargo fmt --check
cargo test --locked
```

For shell, tmux integration, packaging, and interactive navigator coverage:

```sh
bash tests/run.sh
```

The integration tests create disposable tmux servers. Run them in an environment
that permits local tmux sockets. For the opt-in installed-Codex input regression, run
`DRUDWYN_CODEX_TEST_BIN=/absolute/path/to/codex python3 tests/codex_startup_test.py`.
It uses disposable configuration and a local dummy provider, without credentials
or model calls. The normal suite skips this check unless that variable is set.

For a UI change, also exercise the affected action
in tmux and describe the result in your PR. For documentation-only changes, check
relative links, heading anchors, and `git diff --check`.

To preview the plugin locally, build with `cargo build --release --locked` and use
the [manual checkout installation](docs/installation.md#install-a-checkout-without-tpm).
The plugin prefers that release build; rerun it after changing Rust code.

## Releases

Release tags must match the Cargo package version. GitHub Actions builds and
checksums Linux and macOS archives for x86_64 and ARM64. Before tagging, run the
release workflow manually and complete the [preflight checklist](docs/preflight-checklist.md)
against its `release-bundle`.

## README presentation

Follow the [README standard](docs/readme-style.md) for substantial presentation changes.
