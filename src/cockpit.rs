use std::{
    collections::HashMap,
    io,
    path::PathBuf,
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use thiserror::Error;

use crate::{
    config::Config,
    discovery,
    domain::{AgentKind, GitState, Lifecycle, Workspace},
    theme::{Theme, Variant},
    workspace::{self, Start},
};

#[derive(Debug, Error)]
pub enum CockpitError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Discovery(#[from] discovery::DiscoveryError),
}

pub struct App {
    workspaces: Vec<Workspace>,
    visible: Vec<usize>,
    selected: usize,
    filter: String,
    filtering: bool,
    error: Option<String>,
    task: Option<String>,
    start_agent: usize,
    finishing: bool,
    git_diffs: HashMap<PathBuf, GitDiff>,
    nerd_icons: bool,
    config: Config,
    theme: Theme,
}

#[derive(Clone, Copy, Debug, Default)]
struct GitDiff {
    added: u64,
    deleted: u64,
    files: usize,
    untracked: usize,
}

impl App {
    pub fn new(workspaces: Vec<Workspace>, variant: Variant, config: Config) -> Self {
        let visible = (0..workspaces.len()).collect();
        let git_diffs = collect_git_diffs(&workspaces);
        Self {
            workspaces,
            visible,
            selected: 0,
            filter: String::new(),
            filtering: false,
            error: None,
            task: None,
            start_agent: agents()
                .iter()
                .position(|agent| *agent == config.default_agent)
                .unwrap_or(0),
            finishing: false,
            git_diffs,
            nerd_icons: false,
            config,
            theme: Theme::rose_pine(variant),
        }
    }

    fn begin_start(&mut self) {
        self.task = Some(String::new());
        self.error = None;
    }

    fn start_agent(&self) -> AgentKind {
        agents()[self.start_agent]
    }

    fn next_start_agent(&mut self) {
        self.start_agent = (self.start_agent + 1) % agents().len();
    }

    fn workspace_label<'a>(&self, workspace: &'a Workspace) -> &'a str {
        if self.config.redact_labels {
            "Workspace"
        } else {
            workspace
                .checkout
                .branch
                .as_deref()
                .unwrap_or(&workspace.identity.window_name)
        }
    }

    fn project_label<'a>(&self, workspace: &'a Workspace) -> &'a str {
        if self.config.redact_labels {
            "Project"
        } else {
            workspace
                .checkout
                .repository
                .as_deref()
                .and_then(std::path::Path::file_name)
                .and_then(|name| name.to_str())
                .unwrap_or("project")
        }
    }

    fn selected_workspace(&self) -> Option<&Workspace> {
        self.visible
            .get(self.selected)
            .and_then(|index| self.workspaces.get(*index))
    }

    fn move_selection(&mut self, delta: isize) {
        if self.visible.is_empty() {
            self.selected = 0;
            return;
        }
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.visible.len() - 1);
    }

    fn refresh_filter(&mut self) {
        let query = self.filter.to_lowercase();
        self.visible = self
            .workspaces
            .iter()
            .enumerate()
            .filter(|(_, workspace)| {
                query.is_empty()
                    || workspace.identity.session.to_lowercase().contains(&query)
                    || workspace
                        .identity
                        .window_name
                        .to_lowercase()
                        .contains(&query)
                    || workspace
                        .checkout
                        .branch
                        .as_deref()
                        .is_some_and(|branch| branch.to_lowercase().contains(&query))
                    || workspace.lifecycle.label().to_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
        self.selected = self.selected.min(self.visible.len().saturating_sub(1));
    }
}

pub fn run(variant: Variant) -> Result<(), CockpitError> {
    let config = Config::load_tmux().map_err(|error| io::Error::other(error.to_string()))?;
    let mut app = App::new(discovery::discover()?, variant, config);
    app.nerd_icons = Command::new("tmux")
        .args(["show-option", "-gqv", "@agent-watch-icon-mode"])
        .output()
        .is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "nerd"
        });
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = event_loop(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), CockpitError> {
    loop {
        terminal.draw(|frame| render(frame, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if let Some(task) = &mut app.task {
            match key.code {
                KeyCode::Esc => app.task = None,
                KeyCode::Backspace => {
                    task.pop();
                }
                KeyCode::Tab | KeyCode::Right => app.next_start_agent(),
                KeyCode::Char(character) => task.push(character),
                KeyCode::Enter if !task.trim().is_empty() => {
                    let task_text = std::mem::take(task);
                    let slug = slug(&task_text);
                    app.task = None;
                    match workspace::start(Start {
                        repo: std::env::current_dir()?,
                        branch: format!("{}{slug}", app.config.branch_prefix),
                        root: None,
                        command: vec![app.start_agent().command().into()],
                    }) {
                        Ok(started) => {
                            if let Err(error) =
                                workspace::deliver_task(&started.window_id, &task_text)
                            {
                                app.error = Some(error.to_string());
                            } else {
                                Command::new("tmux")
                                    .args(["select-window", "-t", &started.window_id])
                                    .status()
                                    .ok();
                                return Ok(());
                            }
                        }
                        Err(error) => app.error = Some(error.to_string()),
                    }
                }
                _ => {}
            }
            continue;
        }
        if app.finishing {
            match key.code {
                KeyCode::Char('y') => {
                    app.finishing = false;
                    if let Some(path) = app
                        .selected_workspace()
                        .map(|item| item.checkout.working_directory.clone())
                    {
                        match workspace::finish(&path, &app.config.base_branch, true) {
                            Ok(_) => {
                                app.workspaces = discovery::discover()?;
                                app.refresh_filter();
                            }
                            Err(error) => app.error = Some(error.to_string()),
                        }
                    }
                }
                KeyCode::Esc | KeyCode::Char('n') => app.finishing = false,
                _ => {}
            }
            continue;
        }
        if app.filtering {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => app.filtering = false,
                KeyCode::Backspace => {
                    app.filter.pop();
                    app.refresh_filter();
                }
                KeyCode::Char(character) => {
                    app.filter.push(character);
                    app.refresh_filter();
                }
                _ => {}
            }
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
            KeyCode::Char('/') => app.filtering = true,
            KeyCode::Char('n') => {
                app.begin_start();
            }
            KeyCode::Char('f')
                if app.selected_workspace().is_some_and(|item| {
                    item.checkout.is_linked_worktree && item.checkout.git_state == GitState::Clean
                }) =>
            {
                app.finishing = true
            }
            KeyCode::Char('r') => {
                app.workspaces = discovery::discover()?;
                app.git_diffs = collect_git_diffs(&app.workspaces);
                app.refresh_filter();
            }
            KeyCode::Enter => {
                if let Some(workspace) = app.selected_workspace() {
                    let switched = Command::new("tmux")
                        .args(["switch-client", "-t", &workspace.identity.session])
                        .status()
                        .is_ok_and(|status| status.success());
                    let selected = Command::new("tmux")
                        .args(["select-window", "-t", &workspace.identity.window_id])
                        .status()
                        .is_ok_and(|status| status.success());
                    if switched && selected {
                        return Ok(());
                    }
                    app.error = Some("Could not jump to that workspace".into());
                }
            }
            _ => {}
        }
    }
}

fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(4),
        ])
        .split(area);
    render_header(frame, app, layout[0]);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(43), Constraint::Percentage(57)])
        .split(layout[1]);
    render_list(frame, app, body[0]);
    render_detail(frame, app, body[1]);
    render_footer(frame, app, layout[2]);
    if let Some(task) = &app.task {
        let modal = centered(area, 70, 11);
        let label_width = usize::from(modal.width.saturating_sub(10));
        let branch = slug(task);
        let task_label = if app.config.redact_labels {
            "[redacted]".to_owned()
        } else {
            format!("{}_", ellipsize(task, label_width.saturating_sub(1)))
        };
        let branch_label = if app.config.redact_labels {
            "[redacted]".to_owned()
        } else {
            ellipsize(
                &format!("{}{branch}", app.config.branch_prefix),
                label_width,
            )
        };
        let text = vec![
            Line::styled(
                "START WORKSPACE",
                Style::default()
                    .fg(app.theme.rose)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::from(""),
            Line::from(format!("Task   {task_label}")),
            Line::from(format!("Branch {branch_label}")),
            Line::from(format!(
                "Agent  {}  (Tab change)",
                app.start_agent().label()
            )),
            Line::from(""),
            Line::styled(
                "Enter create · Esc cancel",
                Style::default().fg(app.theme.muted),
            ),
        ];
        frame.render_widget(Clear, modal);
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().bg(app.theme.base).fg(app.theme.text)),
            modal,
        );
    } else if app.finishing
        && let Some(workspace) = app.selected_workspace()
    {
        let branch = app.workspace_label(workspace);
        let text = vec![
            Line::styled(
                "FINISH WORKSPACE",
                Style::default()
                    .fg(app.theme.love)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::from(""),
            Line::from(branch.to_owned()),
            Line::from("Requires a clean worktree integrated into its base."),
            Line::from("The branch will be retained."),
            Line::from(""),
            Line::styled(
                "y finish · n/Esc cancel",
                Style::default().fg(app.theme.muted),
            ),
        ];
        let modal = centered(area, 70, 11);
        frame.render_widget(Clear, modal);
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().bg(app.theme.base).fg(app.theme.text)),
            modal,
        );
    }
}

fn slug(value: &str) -> String {
    let mut output = String::new();
    for character in value.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character);
        } else if !output.ends_with('-') && !output.is_empty() {
            output.push('-');
        }
    }
    output.trim_end_matches('-').to_owned()
}

fn agents() -> &'static [AgentKind] {
    &[AgentKind::Codex, AgentKind::Claude, AgentKind::OpenCode]
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn render_header(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let attention = app
        .workspaces
        .iter()
        .filter(|workspace| workspace.lifecycle.needs_attention())
        .count();
    let title = Line::from(vec![
        Span::styled(
            " WORKSPACE COCKPIT ",
            Style::default()
                .fg(app.theme.rose)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "{} live · {} need attention",
                app.workspaces.len(),
                attention
            ),
            Style::default().fg(app.theme.muted),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(title).block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn render_list(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    if app.visible.is_empty() {
        let message = if app.workspaces.is_empty() {
            "No live agent workspaces. Start an agent in tmux, then press r."
        } else {
            "No workspaces match this filter."
        };
        frame.render_widget(
            Paragraph::new(message).style(Style::default().fg(app.theme.muted)),
            area,
        );
        return;
    }
    let items = app.visible.iter().map(|index| {
        let workspace = &app.workspaces[*index];
        let state_color = match workspace.lifecycle {
            Lifecycle::Waiting => app.theme.gold,
            Lifecycle::Review => app.theme.pine,
            Lifecycle::Failed => app.theme.love,
            Lifecycle::Working | Lifecycle::Starting => app.theme.rose,
            Lifecycle::Unknown => app.theme.muted,
        };
        let project = app.project_label(workspace);
        let branch = app.workspace_label(workspace);
        let git = match workspace.checkout.git_state {
            GitState::Clean => "clean",
            GitState::Dirty => "dirty",
            GitState::Unknown => "no git",
        };
        ListItem::new(Line::from(vec![
            Span::styled(
                format!(" {:<9} ", workspace.lifecycle.label()),
                Style::default()
                    .fg(state_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(project.to_owned(), Style::default().fg(app.theme.text)),
            Span::styled(" · ", Style::default().fg(app.theme.muted)),
            Span::styled(branch.to_owned(), Style::default().fg(app.theme.text)),
            Span::styled(format!("  {git}"), Style::default().fg(app.theme.muted)),
        ]))
    });
    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    let mut state = ListState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_detail(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let Some(workspace) = app.selected_workspace() else {
        frame.render_widget(
            Paragraph::new("Select a workspace to inspect it.")
                .style(Style::default().fg(app.theme.muted))
                .block(Block::default().title(" SELECTED ").borders(Borders::LEFT)),
            area,
        );
        return;
    };
    let color = match workspace.lifecycle {
        Lifecycle::Waiting => app.theme.gold,
        Lifecycle::Review => app.theme.pine,
        Lifecycle::Failed => app.theme.love,
        Lifecycle::Working | Lifecycle::Starting => app.theme.rose,
        Lifecycle::Unknown => app.theme.muted,
    };
    let private = app.config.redact_labels;
    let session = if private {
        "tmux"
    } else {
        &workspace.identity.session
    };
    let window = if private {
        "workspace"
    } else {
        &workspace.identity.window_name
    };
    let branch = if private {
        "Workspace"
    } else {
        workspace.checkout.branch.as_deref().unwrap_or("detached")
    };
    let path = if private {
        "[redacted]".to_owned()
    } else {
        workspace.checkout.working_directory.display().to_string()
    };
    let checkout = if workspace.checkout.is_linked_worktree {
        "linked worktree"
    } else {
        "primary checkout"
    };
    let git = match workspace.checkout.git_state {
        GitState::Clean => "clean",
        GitState::Dirty => "dirty",
        GitState::Unknown => "no Git metadata",
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!(
                    " {} {} ",
                    if app.nerd_icons { "󰚩" } else { "A" },
                    workspace.agent.label()
                ),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(workspace.lifecycle.label(), Style::default().fg(color)),
            Span::styled(
                format!(" · {}", age(workspace.state_since)),
                Style::default().fg(app.theme.muted),
            ),
        ]),
        Line::from(""),
        detail_line("TMUX", format!("{session} · {window}"), app.theme.muted),
        detail_line("PATH", path, app.theme.muted),
        detail_line("BRANCH", branch.to_owned(), app.theme.muted),
        detail_line("CHECKOUT", format!("{checkout} · {git}"), app.theme.muted),
    ];
    if let Some(diff) = app.git_diffs.get(&workspace.checkout.working_directory) {
        if diff.files == 0 {
            lines.push(Line::from(vec![
                Span::styled(" CHANGES   ", Style::default().fg(app.theme.muted)),
                Span::styled("✓ clean", Style::default().fg(app.theme.pine)),
            ]));
        } else {
            let mut changes = vec![
                Span::styled(" CHANGES   ", Style::default().fg(app.theme.muted)),
                Span::styled(
                    format!("+{}", diff.added),
                    Style::default().fg(app.theme.pine),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("−{}", diff.deleted),
                    Style::default().fg(app.theme.love),
                ),
                Span::styled(
                    format!(" · {} files", diff.files),
                    Style::default().fg(app.theme.muted),
                ),
            ];
            if diff.untracked > 0 {
                changes.push(Span::styled(
                    format!(" · ?{}", diff.untracked),
                    Style::default().fg(app.theme.gold),
                ));
            }
            lines.push(Line::from(changes));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::styled(
        " NEXT",
        Style::default()
            .fg(app.theme.rose)
            .add_modifier(Modifier::BOLD),
    ));
    let next = match workspace.lifecycle {
        Lifecycle::Waiting => " Enter  open · input required",
        Lifecycle::Review => " Enter  open · review changes",
        Lifecycle::Failed => " Enter  open · inspect failure",
        Lifecycle::Working | Lifecycle::Starting => " Enter  open · agent active",
        Lifecycle::Unknown => " Enter  open workspace",
    };
    lines.push(Line::styled(next, Style::default().fg(app.theme.text)));
    let finish = if !workspace.checkout.is_linked_worktree {
        " f      unavailable · primary checkout"
    } else if workspace.checkout.git_state != GitState::Clean {
        " f      unavailable · worktree dirty"
    } else {
        " f      finish · verifies merged branch"
    };
    lines.push(Line::styled(finish, Style::default().fg(app.theme.muted)));
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().title(" SELECTED ").borders(Borders::LEFT)),
        area,
    );
}

fn ellipsize(value: &str, width: usize) -> String {
    let count = value.chars().count();
    if count <= width {
        return value.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    value.chars().take(width - 1).chain(['…']).collect()
}

fn detail_line(label: &'static str, value: String, muted: ratatui::style::Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {label:<9}"), Style::default().fg(muted)),
        Span::raw(value),
    ])
}

fn collect_git_diffs(workspaces: &[Workspace]) -> HashMap<PathBuf, GitDiff> {
    workspaces
        .iter()
        .filter_map(|workspace| {
            git_diff(&workspace.checkout.working_directory)
                .map(|diff| (workspace.checkout.working_directory.clone(), diff))
        })
        .collect()
}

fn git_diff(path: &std::path::Path) -> Option<GitDiff> {
    let diff = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["diff", "--numstat", "HEAD", "--"])
        .output()
        .ok()?;
    if !diff.status.success() {
        return None;
    }
    let mut result = GitDiff::default();
    for line in String::from_utf8_lossy(&diff.stdout).lines() {
        let mut fields = line.split_whitespace();
        result.added += fields
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        result.deleted += fields
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
    }
    let status = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !status.status.success() {
        return None;
    }
    let status = String::from_utf8_lossy(&status.stdout);
    result.files = status.lines().count();
    result.untracked = status.lines().filter(|line| line.starts_with("??")).count();
    Some(result)
}

fn age(since: Option<u64>) -> String {
    let Some(since) = since else {
        return "now".into();
    };
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(since);
    if elapsed < 60 {
        format!("{elapsed}s")
    } else if elapsed < 3600 {
        format!("{}m", elapsed / 60)
    } else if elapsed < 86400 {
        format!("{}h", elapsed / 3600)
    } else {
        format!("{}d", elapsed / 86400)
    }
}

fn render_footer(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let prompt = if app.filtering {
        format!("Filter › {}_", app.filter)
    } else if app.filter.is_empty() {
        "Enter review · n new · f finish · j/k move · / filter · r refresh · q close".into()
    } else {
        format!(
            "Filter: {} · Enter jump · / edit · r refresh · q close",
            app.filter
        )
    };
    let error = app.error.as_deref().unwrap_or("");
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(error, Style::default().fg(app.theme.love)),
            Line::styled(prompt, Style::default().fg(app.theme.muted)),
        ])
        .block(Block::default().borders(Borders::TOP)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AgentKind, Checkout, EvidenceSource, WorkspaceIdentity};
    use ratatui::backend::TestBackend;
    use std::path::PathBuf;

    fn workspace(branch: &str, state: Lifecycle) -> Workspace {
        Workspace {
            identity: WorkspaceIdentity {
                session: "dev".into(),
                window_id: "@1".into(),
                window_name: "agent".into(),
                pane_id: "%1".into(),
            },
            checkout: Checkout {
                working_directory: PathBuf::from("/repo"),
                repository: Some(PathBuf::from("/repo")),
                worktree: None,
                branch: Some(branch.into()),
                git_state: GitState::Clean,
                is_linked_worktree: false,
            },
            agent: AgentKind::Codex,
            lifecycle: state,
            evidence: EvidenceSource::Hook,
            state_since: Some(1),
            attention_since: None,
        }
    }

    #[test]
    fn filter_matches_branch_and_state() {
        let mut app = App::new(
            vec![
                workspace("work/api", Lifecycle::Working),
                workspace("work/ui", Lifecycle::Review),
            ],
            Variant::Moon,
            Config::default(),
        );
        app.filter = "review".into();
        app.refresh_filter();
        assert_eq!(app.visible, vec![1]);
        app.filter = "api".into();
        app.refresh_filter();
        assert_eq!(app.visible, vec![0]);
    }

    #[test]
    fn start_form_cycles_through_configured_agents() {
        let mut app = App::new(vec![], Variant::Moon, Config::default());
        app.begin_start();
        assert_eq!(app.start_agent(), AgentKind::Codex);
        app.next_start_agent();
        assert_eq!(app.start_agent(), AgentKind::Claude);
        app.next_start_agent();
        assert_eq!(app.start_agent(), AgentKind::OpenCode);
        app.next_start_agent();
        assert_eq!(app.start_agent(), AgentKind::Codex);
    }

    #[test]
    fn redacted_workspace_label_hides_branch_and_window_name() {
        let config = Config {
            redact_labels: true,
            ..Config::default()
        };
        let app = App::new(
            vec![workspace("private/customer-name", Lifecycle::Working)],
            Variant::Moon,
            config,
        );
        assert_eq!(app.workspace_label(&app.workspaces[0]), "Workspace");
        assert_eq!(app.project_label(&app.workspaces[0]), "Project");
    }

    #[test]
    fn start_form_clears_the_detail_panel_beneath_its_modal() {
        let mut app = App::new(
            vec![workspace("main", Lifecycle::Review)],
            Variant::Moon,
            Config::default(),
        );
        app.task = Some("preflight workspace smoke test".into());
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();

        let modal = centered(Rect::new(0, 0, 100, 24), 70, 11);
        let buffer = terminal.backend().buffer();
        let content = (modal.y..modal.bottom())
            .flat_map(|y| {
                (modal.x..modal.right()).map(move |x| buffer.cell((x, y)).unwrap().symbol())
            })
            .collect::<String>();
        assert!(!content.contains("CHECKOUT"));
    }

    #[test]
    fn start_form_truncates_long_labels_inside_its_border() {
        let mut app = App::new(vec![], Variant::Moon, Config::default());
        app.task = Some(
            "Audit workspace lifecycle and worktree edge cases; add regression tests and fix any failures"
                .into(),
        );
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();

        let modal = centered(Rect::new(0, 0, 100, 24), 70, 11);
        let buffer = terminal.backend().buffer();
        for y in [modal.y + 3, modal.y + 4] {
            assert_eq!(buffer.cell((modal.right() - 2, y)).unwrap().symbol(), " ");
        }
        let content = (modal.y..modal.bottom())
            .flat_map(|y| {
                (modal.x..modal.right()).map(move |x| buffer.cell((x, y)).unwrap().symbol())
            })
            .collect::<String>();
        assert!(content.contains('…'));
    }
}
