use clap::{Parser, Subcommand};
use std::{io::Read, path::PathBuf};
use tmux_agent_watch::{
    ambient, cockpit,
    config::Config,
    discovery,
    domain::AgentKind,
    lifecycle, navigator, session_navigator,
    theme::{Theme, Variant},
    workspace::{self, Start},
};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the live, content-blind workspace model.
    Status,
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
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
}

#[derive(Debug, Subcommand)]
enum WorkspaceCommand {
    Start {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        worktree_root: Option<PathBuf>,
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
        eprintln!("tmux-agent-watch: {error}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Status => {
            let config = Config::load_tmux()?;
            for workspace in discovery::discover()? {
                let label = workspace.lifecycle.label();
                let name = workspace
                    .checkout
                    .branch
                    .as_deref()
                    .unwrap_or(&workspace.identity.window_name);
                println!(
                    "{}\t{}\t{}",
                    workspace.identity.window_id,
                    label,
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
        Command::Cockpit { theme } => cockpit::run(theme.into())?,
        Command::Navigator { theme } => navigator::run(theme.into())?,
        Command::Sessions { theme } => session_navigator::run(theme.into())?,
        Command::Workspace { command } => match command {
            WorkspaceCommand::Start {
                repo,
                worktree_root,
                branch,
                command,
            } => {
                let root = worktree_root
                    .or_else(|| std::env::var_os("AGENT_WATCH_WORKTREE_ROOT").map(PathBuf::from));
                println!(
                    "{}",
                    workspace::start(Start {
                        repo,
                        branch,
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
