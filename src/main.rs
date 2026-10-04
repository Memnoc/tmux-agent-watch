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
    /// Preview and select live worker batches (no persistent registry).
    Batch {
        #[command(subcommand)]
        command: BatchCommand,
    },
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
    Hook {
        agent: AgentArg,
        event: String,
        /// Verify Codex hook terminal ancestry and its session working directory.
        #[arg(long)]
        session_cwd: bool,
    },
    /// Render the two-row local status bar with global attention.
    StatusBar {
        #[arg(long)]
        session: String,
        #[arg(long)]
        window: String,
        #[arg(long, default_value_t = 120)]
        width: usize,
        #[arg(long, value_enum, default_value_t = tmux_drudwyn::status_bar::Row::Both)]
        row: tmux_drudwyn::status_bar::Row,
        /// Reuse observer metadata for ambient redraws without another process scan.
        #[arg(long)]
        projection: bool,
    },
    /// Open a stable status-bar target in the requesting client.
    StatusAction { target: String },
    /// Render a tmux status-line projection.
    Hud {
        mode: HudMode,
        session: String,
        window_id: String,
        #[arg(long, value_enum, default_value_t = ThemeArg::RosePine)]
        theme: ThemeArg,
    },
    /// Render one sidebar frame and its click map.
    Sidebar {
        session: String,
        current_window: String,
        #[arg(long)]
        expanded: bool,
        #[arg(long, value_enum, default_value_t = ThemeArg::RosePine)]
        theme: ThemeArg,
    },
    /// Open the interactive fleet cockpit.
    Cockpit {
        #[command(flatten)]
        query: tmux_drudwyn::inventory::Query,
        /// Print the same global inventory without opening the terminal UI.
        #[arg(long)]
        list: bool,
        /// Open directly in the new workspace form.
        #[arg(long)]
        start: bool,
        #[arg(long, value_enum, default_value_t = ThemeArg::RosePine)]
        theme: ThemeArg,
    },
    /// Open the grouped tmux window navigator.
    Navigator {
        #[arg(long, value_enum, default_value_t = ThemeArg::RosePine)]
        theme: ThemeArg,
    },
    /// Open the compact tmux session navigator.
    Sessions {
        #[arg(long, value_enum, default_value_t = ThemeArg::RosePine)]
        theme: ThemeArg,
    },
    /// Create an ad hoc shell session in the requesting terminal.
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    /// Open the interactive tmux options editor.
    Settings {
        #[arg(long, value_enum, default_value_t = ThemeArg::RosePine)]
        theme: ThemeArg,
    },
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
}

#[derive(Debug, Subcommand)]
enum BatchCommand {
    /// Preview source/destination; --yes requires the two reviewed commits.
    Setup {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        session: Option<String>,
        #[arg(long, default_value = "base")]
        source: String,
        #[arg(long)]
        integration: Option<String>,
        #[arg(long)]
        destination_start: Option<String>,
        #[arg(long)]
        checkout: Option<PathBuf>,
        #[arg(long)]
        reuse_existing: bool,
        #[arg(long, requires_all = ["expect_source", "expect_destination"])]
        yes: bool,
        #[arg(long)]
        expect_source: Option<String>,
        #[arg(long)]
        expect_destination: Option<String>,
    },
    Show {
        id: String,
    },
    /// Select a batch for subsequent siblings from this window.
    Select {
        id: String,
        #[arg(long)]
        window: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum SessionCommand {
    /// Create a named shell; directory defaults to the invoking workspace.
    New {
        #[arg(long)]
        name: String,
        #[arg(long)]
        directory: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum CoordinatorCommand {
    /// Toggle between coordinator and the previous worker in this terminal.
    Toggle,
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
    /// Inspect live assembled evidence, or run explicitly selected checks visibly.
    Verify {
        #[arg(long)]
        path: PathBuf,
        /// Non-content check identity; commands travel only through stdin.
        #[arg(long, requires = "command_stdin")]
        check: Option<String>,
        #[arg(long, requires = "check")]
        command_stdin: bool,
    },
    /// Preview reviewed commits; apply only the token from an unchanged preview.
    Integrate {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(
            long,
            required_unless_present = "destination",
            conflicts_with = "destination"
        )]
        batch: Option<String>,
        /// Explicit existing local destination branch, when no live batch is selected.
        #[arg(long)]
        destination: Option<String>,
        /// Apply this preview token after reviewing refs, commits and checkout.
        #[arg(long)]
        apply: Option<String>,
    },
    /// Explicitly promote an assembled branch to a selected base, with fresh review.
    Promote {
        #[arg(long)]
        path: PathBuf,
        /// Existing checked-out local base branch; never inferred from another batch.
        #[arg(long)]
        base: String,
        #[arg(long)]
        apply: Option<String>,
    },
    /// Inspect a retained merge; resolve through the coordinator, Continue or Abort.
    Conflict {
        #[arg(long)]
        path: PathBuf,
        /// Start a new coordinator agent conversation in the retained destination; no task yet.
        #[arg(long, value_enum, conflicts_with_all = ["continue_merge", "abort", "retry", "open"])]
        recover_agent: Option<AgentArg>,
        /// Explicitly associate a recovered project session before inspecting/retrying.
        #[arg(long)]
        project: Option<String>,
        #[arg(long = "continue", conflicts_with_all = ["abort", "retry", "open"])]
        continue_merge: bool,
        #[arg(long, conflicts_with_all = ["continue_merge", "retry", "open"])]
        abort: bool,
        #[arg(long, conflicts_with_all = ["continue_merge", "abort", "open"])]
        retry: bool,
        #[arg(long, conflicts_with_all = ["continue_merge", "abort", "retry"])]
        open: bool,
    },
    /// Open a surviving checkout; never create/reset a worktree.
    Recover {
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        path: PathBuf,
        #[arg(long, required_unless_present = "agent", conflicts_with_all = ["agent", "task_file", "task_stdin", "use_task_reference", "batch", "unassociated"])]
        shell: bool,
        #[arg(long, value_enum, requires = "recovery_batch")]
        agent: Option<AgentArg>,
        #[arg(long, conflicts_with_all = ["task_stdin", "use_task_reference"], requires = "agent")]
        task_file: Option<String>,
        #[arg(long, conflicts_with = "use_task_reference", requires = "agent")]
        task_stdin: bool,
        #[arg(long, requires = "agent")]
        use_task_reference: bool,
        #[arg(
            long,
            group = "recovery_batch",
            conflicts_with = "unassociated",
            requires = "agent"
        )]
        batch: Option<String>,
        /// Explicitly restart without a live batch; historical choices stay unknown.
        #[arg(long, group = "recovery_batch", requires = "agent")]
        unassociated: bool,
        /// Restore the missing coordinator in this project.
        #[arg(long)]
        coordinator: bool,
    },
    /// Enumerate surviving checkouts for a selected known repository.
    RecoverList {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
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
        /// Use an explicit live batch; source/destination remain pinned.
        #[arg(long, conflicts_with_all = ["base", "from_current"])]
        batch: Option<String>,
        /// Deliberate short window name; defaults to the branch.
        #[arg(long)]
        name: Option<String>,
        /// Repository-owned task reference, checked at the pinned source; no content is read.
        #[arg(long)]
        task_file: Option<String>,
        /// Read the initial task from stdin, create the worker, then send once.
        #[arg(long, conflicts_with = "task_file")]
        task_stdin: bool,
        branch: String,
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
    Finish {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Explicit destination branch; --base remains a compatibility alias.
        #[arg(
            long,
            visible_alias = "base",
            required_unless_present = "batch",
            conflicts_with = "batch"
        )]
        destination: Option<String>,
        #[arg(long)]
        batch: Option<String>,
        /// Inspect eligibility and a comparison token without prompting/removal.
        #[arg(long, conflicts_with_all = ["yes", "apply"])]
        preview: bool,
        /// Remove only if this reviewed source/destination is unchanged.
        #[arg(long)]
        apply: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    /// Deliver a task from stdin without placing it in arguments or persistent state.
    DeliverTask {
        window_id: String,
        /// Send only a repository file reference; the agent reads the file.
        #[arg(long)]
        task_file: Option<String>,
        /// Deliberately send again after inspecting a prior sent/uncertain delivery.
        #[arg(long)]
        retry: bool,
    },
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
        Command::Batch { command } => {
            use tmux_drudwyn::{batch, navigation};
            let config = Config::load_tmux()?;
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                match command {
                    BatchCommand::Setup {
                        repo,
                        session,
                        source,
                        integration,
                        destination_start,
                        checkout,
                        reuse_existing,
                        yes,
                        expect_source,
                        expect_destination,
                    } => {
                        let session = match session {
                            Some(s) => s,
                            None => navigation::current_session()?,
                        };
                        let preview = batch::preview(batch::Request {
                            repo,
                            session,
                            base: config.base_branch.clone(),
                            source,
                            integration,
                            destination_start,
                            checkout,
                            reuse_existing,
                        })?;
                        println!("{}", preview.display(config.redact_labels));
                        if yes {
                            if expect_source.as_deref() != Some(&preview.source.commit)
                                || expect_destination.as_deref()
                                    != Some(&preview.destination_commit)
                            {
                                return Err("Source or destination changed after preview; review a fresh preview".into());
                            }
                            println!("{}", batch::create(&preview)?.display(config.redact_labels));
                        } else {
                            println!(
                                "Preview only; nothing created. Confirm with --yes --expect-source COMMIT --expect-destination COMMIT, or cancel by leaving."
                            );
                        }
                    }
                    BatchCommand::Show { id } => {
                        println!("{}", batch::load(&id)?.display(config.redact_labels))
                    }
                    BatchCommand::Select { id, window } => {
                        match window {
                            Some(w) => batch::select(&id, &w)?,
                            None => batch::select_current(&id)?,
                        };
                        println!("{}", batch::load(&id)?.display(config.redact_labels));
                    }
                }
                Ok(())
            })();
            if config.redact_labels && result.is_err() {
                return Err("Batch operation failed; details hidden by label redaction".into());
            }
            result?;
        }
        Command::Coordinator { command } => match command {
            CoordinatorCommand::Toggle => tmux_drudwyn::coordinator::toggle()?,
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
                    "{}\t{}\t{}\t{}\t{}\tevidence {}\t{}",
                    workspace.identity.window_id,
                    label,
                    workspace.role(),
                    if config.redact_labels {
                        "Workspace"
                    } else {
                        name
                    },
                    workspace.agent.label(),
                    workspace.evidence.label(),
                    workspace.process_label()
                );
            }
        }
        Command::StatusBar {
            session,
            window,
            width,
            row,
            projection,
        } => {
            print!(
                "{}",
                tmux_drudwyn::status_bar::render(&session, &window, width, row, projection)?
            );
        }
        Command::StatusAction { target } => tmux_drudwyn::status_bar::action(&target)?,
        Command::Scan => lifecycle::scan()?,
        Command::Hook {
            agent,
            event,
            session_cwd,
        } => {
            if session_cwd {
                if !matches!(agent, AgentArg::Codex) {
                    return Err("--session-cwd is only supported for Codex".into());
                }
                lifecycle::codex_session_hook(&event)?;
            } else {
                lifecycle::hook(agent.into(), &event)?;
            }
        }
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
        Command::Cockpit {
            theme,
            start,
            query,
            list,
        } => {
            if list {
                let snapshot = tmux_drudwyn::inventory::Snapshot::capture()?;
                let config = Config::load_tmux()?;
                print!("{}", snapshot.display(&query, config.redact_labels)?);
            } else {
                cockpit::run(theme.into(), start, query)?;
            }
        }
        Command::Navigator { theme } => navigator::run(theme.into())?,
        Command::Sessions { theme } => session_navigator::run(theme.into())?,
        Command::Session { command } => match command {
            SessionCommand::New { name, directory } => {
                println!(
                    "{}",
                    tmux_drudwyn::session::create(&name, directory.as_deref())?
                );
            }
        },
        Command::Settings { theme } => settings::run(theme.into())?,
        Command::Workspace { command } => match command {
            WorkspaceCommand::Verify {
                path,
                check,
                command_stdin,
            } => {
                let redact = Config::load_tmux()?.redact_labels;
                if command_stdin {
                    use std::io::Read;
                    let mut command = String::new();
                    std::io::stdin().take(65537).read_to_string(&mut command)?;
                    let (receipt, passed) = tmux_drudwyn::verification::run(
                        &path,
                        check.as_deref().unwrap(),
                        &command,
                        redact,
                    )?;
                    print!("{receipt}");
                    if !passed {
                        return Err("Assembled verification did not pass; work preserved; inspect or rerun explicitly".into());
                    }
                } else {
                    print!("{}", tmux_drudwyn::verification::inspect(&path, redact)?);
                }
            }
            WorkspaceCommand::Conflict {
                path,
                project,
                recover_agent,
                continue_merge,
                abort,
                retry,
                open,
            } => {
                use tmux_drudwyn::integration::ConflictAction;
                if let Some(project) = project {
                    tmux_drudwyn::integration::select_conflict_project(&path, &project)?;
                }
                if let Some(agent) = recover_agent {
                    println!(
                        "{}",
                        tmux_drudwyn::integration::recover_conflict_agent(&path, agent.into())?
                    );
                    return Ok(());
                }
                let action = if continue_merge {
                    ConflictAction::Continue
                } else if abort {
                    ConflictAction::Abort
                } else if retry {
                    ConflictAction::Retry
                } else if open {
                    ConflictAction::Open
                } else {
                    ConflictAction::Inspect
                };
                println!(
                    "{}",
                    tmux_drudwyn::integration::conflict(
                        &path,
                        action,
                        Config::load_tmux()?.redact_labels
                    )?
                );
            }
            WorkspaceCommand::Promote { path, base, apply } => {
                use tmux_drudwyn::integration::{self, Destination, Request};
                let redact = Config::load_tmux()?.redact_labels;
                let preview = integration::preview(Request {
                    source: path,
                    destination: Destination::Branch(base),
                })?;
                if let Some(token) = apply {
                    if token != preview.token {
                        return Err(
                            "Source or destination changed after preview; review again".into()
                        );
                    }
                    println!("{}", integration::apply(&preview)?);
                } else {
                    print!("PROMOTE PREVIEW\n{}", preview.display(redact));
                    println!(
                        "Preview only; --apply TOKEN explicitly promotes. No push or deployment."
                    );
                }
                println!(
                    "Destination verification:\n{}",
                    tmux_drudwyn::verification::inspect(&preview.target.path, redact)?
                );
            }
            WorkspaceCommand::Integrate {
                path,
                batch,
                destination,
                apply,
            } => {
                use tmux_drudwyn::integration::{self, Destination, Request};
                let redact = Config::load_tmux()?.redact_labels;
                let destination = match batch {
                    Some(id) => Destination::Batch(id),
                    None => Destination::Branch(
                        destination.ok_or("Select an explicit batch or destination")?,
                    ),
                };
                let preview = integration::preview(Request {
                    source: path,
                    destination,
                })?;
                if let Some(token) = apply {
                    if token != preview.token {
                        return Err(
                            "Source or destination changed after preview; review again".into()
                        );
                    }
                    println!("{}", integration::apply(&preview)?);
                } else {
                    print!("{}", preview.display(redact));
                    println!(
                        "Preview only; no changes. Re-run with --apply TOKEN to integrate this reviewed state."
                    );
                }
            }
            WorkspaceCommand::Recover {
                repo,
                path,
                shell: _,
                agent,
                task_file,
                task_stdin,
                use_task_reference,
                batch,
                unassociated: _,
                coordinator,
            } => {
                use tmux_drudwyn::recovery::{Request, Task};
                let task = if let Some(file) = task_file {
                    Task::File(file)
                } else if task_stdin {
                    let mut text = String::new();
                    std::io::stdin().read_to_string(&mut text)?;
                    Task::Text(text)
                } else if use_task_reference {
                    Task::RetainedReference
                } else {
                    Task::None
                };
                let agent = agent.map(Into::into);
                let started = tmux_drudwyn::recovery::recover(Request {
                    repo,
                    path,
                    agent,
                    task,
                    batch,
                    coordinator,
                })?;
                println!("{}", started.window_id);
                if agent.is_some() {
                    println!(
                        "Fresh conversation; task sent. Acceptance, historical exit and checks unknown; conversation not restored."
                    );
                }
            }
            WorkspaceCommand::RecoverList { repo } => {
                let redact = Config::load_tmux()?.redact_labels;
                for checkout in tmux_drudwyn::recovery::list(&repo)? {
                    println!("{}", checkout.display(redact));
                }
            }
            WorkspaceCommand::Start {
                repo,
                worktree_root,
                base,
                from_current,
                batch,
                branch,
                name,
                task_file,
                task_stdin,
                command,
            } => {
                let config = Config::load_tmux()?;
                let selected = if let Some(id) = batch {
                    Some(tmux_drudwyn::batch::load(&id)?)
                } else if base.is_none() && !from_current {
                    tmux_drudwyn::batch::current()?
                } else {
                    None
                };
                let base = match base {
                    Some(base) => base,
                    None => config.base_branch,
                };
                let point = match &selected {
                    Some(batch) => batch.source.clone(),
                    None => workspace::resolve_start_point(&repo, &base, from_current)?,
                };
                if config.redact_labels {
                    eprintln!("Starting from [redacted] — local ref; remote freshness unknown");
                } else {
                    eprintln!(
                        "Starting from {} ({}) — local ref; remote freshness unknown",
                        point.reference,
                        &point.commit[..12]
                    );
                }
                let root = worktree_root
                    .or_else(|| std::env::var_os("DRUDWYN_WORKTREE_ROOT").map(PathBuf::from));
                let mut task = String::new();
                if task_stdin {
                    std::io::stdin().read_to_string(&mut task)?;
                }
                let file = task_file.is_some();
                if let Some(reference) = &task_file {
                    task = reference.clone();
                }
                if (task_stdin || file) && task.trim().is_empty() {
                    return Err("Task is empty; worker not created".into());
                }
                let agent = command
                    .first()
                    .and_then(|c| AgentKind::from_command(c))
                    .or_else(|| command.is_empty().then_some(AgentKind::Codex));
                let started = workspace::start(Start {
                    repo,
                    branch,
                    name,
                    start_point: point.commit,
                    batch: selected.map(|b| b.id),
                    root,
                    command,
                    task_file,
                })?;
                println!("{}", started.path.display());
                eprintln!(
                    "Worker created: window {} pane {}; task not sent yet",
                    started.window_id, started.pane_id
                );
                if task_stdin || file {
                    workspace::send_started(&started, &task, file, agent)?;
                    println!("Task sent; acceptance and implementation unknown");
                }
            }
            WorkspaceCommand::Finish {
                path,
                destination,
                batch,
                preview,
                apply,
                yes,
            } => {
                use tmux_drudwyn::{
                    cleanup,
                    integration::{Destination, Request},
                };
                let redact = Config::load_tmux()?.redact_labels;
                let destination = match batch {
                    Some(id) => Destination::Batch(id),
                    None => {
                        Destination::Branch(destination.ok_or("Choose a destination or batch")?)
                    }
                };
                let reviewed = cleanup::preview(Request {
                    source: path,
                    destination,
                })?;
                if let Some(token) = &apply {
                    if token != &reviewed.token {
                        return Err(
                            "Source or destination changed after preview; review again".into()
                        );
                    }
                }
                if preview {
                    print!("{}", reviewed.display(redact));
                } else {
                    if !yes {
                        use std::io::Write;
                        eprint!(
                            "{}Remove this linked worktree, retaining its branch? [y/N] ",
                            reviewed.display(redact)
                        );
                        std::io::stderr().flush()?;
                        let mut answer = String::new();
                        std::io::stdin().read_line(&mut answer)?;
                        if !matches!(answer.trim(), "y" | "Y" | "yes" | "YES") {
                            return Err("cancelled".into());
                        }
                    }
                    let removed = cleanup::apply(&reviewed)?;
                    println!(
                        "{}",
                        if redact {
                            "[redacted]".into()
                        } else {
                            removed.display().to_string()
                        }
                    );
                }
            }
            WorkspaceCommand::DeliverTask {
                window_id,
                task_file,
                retry,
            } => {
                if let Some(reference) = task_file {
                    workspace::deliver_reference(&window_id, &reference, retry)?;
                } else {
                    let mut task = String::new();
                    std::io::stdin().read_to_string(&mut task)?;
                    workspace::deliver(&window_id, &task, retry)?;
                }
                println!("Task sent; acceptance and implementation unknown");
            }
        },
    }
    Ok(())
}
