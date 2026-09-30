use clap::{Parser, Subcommand};
use std::{io::Read, path::PathBuf};
use tmux_drudwyn::{
    ambient, cockpit,
    config::Config,
    discovery,
    domain::AgentKind,
    lifecycle, navigator, session_navigator, settings,
    theme::{Theme, Variant},
    workspace::{self, Start},
};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Explicit attached tmux client name (or DRUDWYN_CLIENT).
    #[arg(long, global = true)]
    client: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the live, content-blind workspace model.
    Status,
    /// Associate or return to the project coordinator.
    Coordinator {
        #[command(subcommand)]
        command: CoordinatorCommand,
    },
    /// Navigate by stable window/session ID in the requesting terminal only.
    Navigate {
        #[arg(long, required_unless_present = "session")]
        window: Option<String>,
        #[arg(long)]
        session: Option<String>,
    },
    /// Refresh live lifecycle metadata without reading terminal content.
    Scan,
    /// Receive a content-blind lifecycle event from an agent integration.
    Hook { agent: AgentArg, event: String },
    /// Render a tmux status-line projection.
    Hud {
        mode: HudMode,
        session: String,
        window_id: String,
        #[arg(long, value_enum, default_value_t = ThemeArg::Moon)]
        theme: ThemeArg,
    },
    /// Render one sidebar frame and its click map.
    Sidebar {
        session: String,
        current_window: String,
        #[arg(long)]
        expanded: bool,
        #[arg(long, value_enum, default_value_t = ThemeArg::Moon)]
        theme: ThemeArg,
    },
    /// Open the interactive fleet cockpit.
    Cockpit {
        /// Open directly in the new workspace form.
        #[arg(long)]
        start: bool,
        #[arg(long, value_enum, default_value_t = ThemeArg::Moon)]
        theme: ThemeArg,
    },
    /// Open the grouped tmux window navigator.
    Navigator {
        #[arg(long, value_enum, default_value_t = ThemeArg::Moon)]
        theme: ThemeArg,
    },
    /// Open the compact tmux session navigator.
    Sessions {
        #[arg(long, value_enum, default_value_t = ThemeArg::Moon)]
        theme: ThemeArg,
    },
    /// Open the interactive tmux options editor.
    Settings {
        #[arg(long, value_enum, default_value_t = ThemeArg::Moon)]
        theme: ThemeArg,
    },
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
}

#[derive(Debug, Subcommand)]
enum CoordinatorCommand {
    /// Choose an existing shell or agent in this project. Preserve its name.
    Set {
        #[arg(long)]
        window: String,
        #[arg(long)]
        session: Option<String>,
    },
    /// Return in this terminal only; unavailable coordinators offer recovery.
    Open {
        #[arg(long)]
        session: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum WorkspaceCommand {
    Start {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        worktree_root: Option<PathBuf>,
        /// Base branch (defaults to @drudwyn-base-branch, then main).
        #[arg(long, conflicts_with = "from_current")]
        base: Option<String>,
        /// Intentionally include the current checkout's commits.
        #[arg(long)]
        from_current: bool,
        /// Deliberate short window name; defaults to the branch.
        #[arg(long)]
        name: Option<String>,
        branch: String,
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
    Finish {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "main")]
        base: String,
        #[arg(long)]
        yes: bool,
    },
    /// Deliver a task from stdin without placing it in arguments or persistent state.
    DeliverTask { window_id: String },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ThemeArg {
    RosePine,
    Moon,
    Dawn,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum AgentArg {
    Codex,
    Claude,
    OpenCode,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum HudMode {
    Fleet,
    Selected,
}

impl From<AgentArg> for AgentKind {
    fn from(value: AgentArg) -> Self {
        match value {
            AgentArg::Codex => Self::Codex,
            AgentArg::Claude => Self::Claude,
            AgentArg::OpenCode => Self::OpenCode,
        }
    }
}

impl From<ThemeArg> for Variant {
    fn from(value: ThemeArg) -> Self {
        match value {
            ThemeArg::RosePine => Self::RosePine,
            ThemeArg::Moon => Self::Moon,
            ThemeArg::Dawn => Self::Dawn,
        }
    }
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("tmux-drudwyn: {error}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(client) = cli.client {
        // The executable is single-threaded before any UI or discovery starts.
        unsafe {
            std::env::set_var("DRUDWYN_CLIENT", client);
        }
    }
    match cli.command {
        Command::Coordinator { command } => match command {
            CoordinatorCommand::Set { window, session } => {
                tmux_drudwyn::coordinator::set(&window, session.as_deref())?
            }
            CoordinatorCommand::Open { session } => {
                let session = match session {
                    Some(session) => session,
                    None => tmux_drudwyn::navigation::current_session()?,
                };
                tmux_drudwyn::coordinator::open_project(&session)?;
            }
        },
        Command::Navigate { window, session } => {
            tmux_drudwyn::navigation::open(window.as_deref(), session.as_deref())?
        }
        Command::Status => {
            let config = Config::load_tmux()?;
            for workspace in discovery::discover()? {
                let label = workspace.lifecycle.label();
                let name = workspace.identity.window_name.as_str();
                println!(
                    "{}\t{}\t{}\t{}",
                    workspace.identity.window_id,
                    label,
                    workspace.role(),
                    if config.redact_labels {
                        "Workspace"
                    } else {
                        name
                    }
                );
            }
        }
        Command::Scan => lifecycle::scan()?,
        Command::Hook { agent, event } => lifecycle::hook(agent.into(), &event)?,
        Command::Hud {
            mode,
            session,
            window_id,
            theme,
        } => {
            let workspaces = discovery::discover_tmux()?;
            let config = Config::load_tmux()?;
            let theme = Theme::rose_pine(theme.into());
            let rendered = match mode {
                HudMode::Fleet => {
                    ambient::hud_fleet(&workspaces, &session, theme, config.redact_labels)
                }
                HudMode::Selected => {
                    ambient::hud_selected(&workspaces, &window_id, theme, config.redact_labels)
                }
            };
            print!("{rendered}");
        }
        Command::Sidebar {
            session,
            current_window,
            expanded,
            theme,
        } => {
            let workspaces = discovery::discover()?;
            let config = Config::load_tmux()?;
            let frame = ambient::sidebar(
                &workspaces,
                &session,
                &current_window,
                expanded,
                Theme::rose_pine(theme.into()),
                config.redact_labels,
            );
            print!("{}\x1c{}", frame.text, frame.click_map);
        }
        Command::Cockpit { theme, start } => cockpit::run(theme.into(), start)?,
        Command::Navigator { theme } => navigator::run(theme.into())?,
        Command::Sessions { theme } => session_navigator::run(theme.into())?,
        Command::Settings { theme } => settings::run(theme.into())?,
        Command::Workspace { command } => match command {
            WorkspaceCommand::Start {
                repo,
                worktree_root,
                base,
                from_current,
                branch,
                name,
                command,
            } => {
                let base = match base {
                    Some(base) => base,
                    None => Config::load_tmux()?.base_branch,
                };
                let point = workspace::resolve_start_point(&repo, &base, from_current)?;
                eprintln!(
                    "Starting from {} ({}) — local ref; remote freshness unknown",
                    point.reference,
                    &point.commit[..12]
                );
                let root = worktree_root
                    .or_else(|| std::env::var_os("DRUDWYN_WORKTREE_ROOT").map(PathBuf::from));
                println!(
                    "{}",
                    workspace::start(Start {
                        repo,
                        branch,
                        name,
                        start_point: point.commit,
                        root,
                        command
                    })?
                    .path
                    .display()
                );
            }
            WorkspaceCommand::Finish { path, base, yes } => {
                println!("{}", workspace::finish(&path, &base, yes)?.display())
            }
            WorkspaceCommand::DeliverTask { window_id } => {
                let mut task = String::new();
                std::io::stdin().read_to_string(&mut task)?;
                workspace::deliver_task(&window_id, &task)?;
            }
        },
    }
    Ok(())
}
