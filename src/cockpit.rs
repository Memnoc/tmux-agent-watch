use std::{io, process::Command, time::Duration};

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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
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
    config: Config,
    theme: Theme,
}

impl App {
    pub fn new(workspaces: Vec<Workspace>, variant: Variant, config: Config) -> Self {
        let visible = (0..workspaces.len()).collect();
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
                if app
                    .selected_workspace()
                    .is_some_and(|item| item.checkout.is_linked_worktree) =>
            {
                app.finishing = true
            }
            KeyCode::Char('r') => {
                app.workspaces = discovery::discover()?;
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
    render_list(frame, app, layout[1]);
    render_footer(frame, app, layout[2]);
    if let Some(task) = &app.task {
        let branch = slug(task);
        let task_label = if app.config.redact_labels {
            "[redacted]".to_owned()
        } else {
            format!("{task}_")
        };
        let branch_label = if app.config.redact_labels {
            "[redacted]".to_owned()
        } else {
            format!("{}{branch}", app.config.branch_prefix)
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
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().bg(app.theme.base).fg(app.theme.text)),
            centered(area, 70, 11),
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
            Line::from("Requires a clean worktree merged into main."),
            Line::from("The branch will be retained."),
            Line::from(""),
            Line::styled(
                "y finish · n/Esc cancel",
                Style::default().fg(app.theme.muted),
            ),
        ];
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().bg(app.theme.base).fg(app.theme.text)),
            centered(area, 70, 11),
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
            Span::styled(branch.to_owned(), Style::default().fg(app.theme.text)),
            Span::styled(
                format!(
                    "  {} · {}:{} · {}",
                    workspace.agent.label(),
                    if app.config.redact_labels {
                        "tmux"
                    } else {
                        &workspace.identity.session
                    },
                    if app.config.redact_labels {
                        "workspace"
                    } else {
                        &workspace.identity.window_name
                    },
                    git
                ),
                Style::default().fg(app.theme.muted),
            ),
        ]))
    });
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(app.theme.pine)
                .fg(app.theme.base)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("›");
    let mut state = ListState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut state);
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
    }
}
