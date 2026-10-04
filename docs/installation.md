# Installation and updates

[Documentation](README.md) · [Project home](../README.md)

Previously installed tmux-agent-watch? Start with the [rename upgrade guide](upgrading-to-drudwyn.md) to preserve your settings and update hook paths.

## Requirements

- Linux or macOS, with tmux supporting `display-popup`.
- Bash and Git available on your PATH.
- Codex, Claude Code, or OpenCode installed separately to run agent tasks.
- Rust and Cargo for a source build, or `curl`, `tar`, and `sha256sum`/`shasum` for the release installer.
- [TPM](https://github.com/tmux-plugins/tpm) if using the plugin-manager route.

Run commands below in a terminal inside a running tmux session. If your tmux
configuration is somewhere other than `~/.tmux.conf`, use that path when reloading.
The scripts must remain in their checkout after installation.

## Install with TPM

Follow the [README quickstart](../README.md#install). It builds the binary from
the same checkout as the scripts, so both contain the same changes.

## Install a checkout without TPM

Clone into a directory you intend to keep:

```sh
git clone https://github.com/Memnoc/tmux-drudwyn.git ~/tmux-drudwyn
cargo build --release --locked --manifest-path ~/tmux-drudwyn/Cargo.toml
```

Add this to `~/.tmux.conf`, after other plugins or themes that configure the status bar:

```tmux
run-shell '~/tmux-drudwyn/tmux-drudwyn.tmux'
```

Reload and open the cockpit:

```sh
tmux source-file ~/.tmux.conf
```

Press `prefix + P` (normally `Ctrl+b`, then `Shift+p`). The Workspace Cockpit
should open. Close it with `Esc`, then press `prefix + w` to find an existing
window. If either popup fails, see [troubleshooting](troubleshooting.md).

## Use a released binary

Older releases retain their original asset names. Use a source build until the
[releases page](https://github.com/Memnoc/tmux-drudwyn/releases) provides a matching
`tmux-drudwyn-*` archive.

The bundled installer downloads the latest published release for Linux or macOS
on x86_64 or ARM64, verifies its checksum, and installs it to `~/.local/bin`:

```sh
~/.tmux/plugins/tmux-drudwyn/install.sh
```

Use your actual checkout path if you did not install with TPM. Ensure
`~/.local/bin` is on the PATH available to tmux. Check the installed binary with:

```sh
~/.local/bin/tmux-drudwyn --version
```

A published release can lag behind a pulled branch. Check the
[release notes](https://github.com/Memnoc/tmux-drudwyn/releases) and use scripts
from the matching release. For the features in a development checkout, build
from that checkout instead.

## Update a pulled checkout

Pull your intended branch if you have not already done so, then rebuild and reload.
For a TPM checkout:

```sh
cd ~/.tmux/plugins/tmux-drudwyn
git pull --ff-only
cargo build --release --locked
tmux source-file ~/.tmux.conf
```

For a manual checkout, change the first command to its directory. If you have
already pulled, start with the build command. TPM updates do not rebuild Rust.
Close and reopen any navigator or cockpit popup that was open before the update.

## Which binary runs?

The plugin launcher checks these locations in order:

1. `DRUDWYN_V2_BIN`, if explicitly set.
2. `target/release/tmux-drudwyn` inside the plugin checkout.
3. `tmux-drudwyn` on PATH.

A local release build therefore takes precedence over a downloaded binary.
Rebuild it after pulling scripts; replacing only the PATH binary will not update
that local build.

If you also want the command available in your shell, install from source with:

```sh
cargo install --locked --path ~/.tmux/plugins/tmux-drudwyn
```

Cargo normally installs to `~/.cargo/bin`. This does not replace an existing
`target/release` build in the checkout. See [missing or old binaries](troubleshooting.md#missing-or-old-binary)
if the shell and plugin appear to run different versions.

## Keep the runtime independent of development branches

A checkout installation uses scripts from that checkout. Switching it to a branch
without those scripts can break live tmux bindings. To test development versions
without this dependency, build and install a complete local snapshot:

```sh
cargo build --release --locked
bash scripts/install-runtime.sh
```

The installer prints the stable tmux loader and agent-hook paths under
`~/.local/share/drudwyn/runtime/current` (or `$XDG_DATA_HOME`). Use the printed
loader in your tmux configuration in place of the development-checkout loader,
and point existing Drudwyn agent hooks at the printed scripts directory. Reload
tmux and restart Drudwyn's watcher after updating the snapshot. Reopen existing
popups. The script does not edit your configuration or start agent services.

Snapshots contain the matching binary, scripts and assets, with an atomic
`current` link. Failed installation leaves the previous snapshot selected.
Previous snapshots are retained for rollback and for already-running processes.
This stores installed application files only; it adds no workspace/task registry.
