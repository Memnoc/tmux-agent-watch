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
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use thiserror::Error;

use crate::{
    batch,
    config::Config,
    discovery,
    domain::{AgentKind, GitState, Lifecycle, Workspace},
    theme::{Theme, Variant},
    ui::{self, FooterTone},
    workspace::{self, Start},
};

const COCKPIT_NAVIGATION: &[(&str, &str)] = &[("j/k", "Move"), ("Enter", "Open")];
const COCKPIT_ACTIONS: &[(&str, &str)] = &[
    ("n", "New"),
    ("b", "Batch setup"),
    ("c", "Coordinator"),
    ("f", "Finish"),
    ("/", "Filter"),
    ("r", "Refresh"),
];
const CLOSE_ACTION: &[(&str, &str)] = &[("q/Esc", "Close")];
const FILTER_ACTIONS: &[(&str, &str)] = &[
    ("text", "Filter"),
    ("Backspace", "Delete"),
    ("Enter/Esc", "Done"),
];
const CREATE_ACTIONS: &[(&str, &str)] = &[
    ("Enter", "Create"),
    ("Tab", "Agent"),
    ("F2", "Base"),
    ("F3", "Batch"),
];
const CREATE_BLOCKED_ACTIONS: &[(&str, &str)] = &[("Tab", "Agent"), ("F2", "Base")];
const CANCEL_ACTION: &[(&str, &str)] = &[("Esc", "Cancel")];
const FINISH_ACTION: &[(&str, &str)] = &[("y", "Finish")];
const FINISH_CANCEL_ACTION: &[(&str, &str)] = &[("n/Esc", "Cancel")];

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
    from_current: bool,
    start_point: Option<workspace::StartPoint>,
    start_error: Option<String>,
    active_batch: Option<batch::Batch>,
    batch_form: Option<BatchForm>,
    batches: HashMap<String, String>,
    finishing: bool,
    git_diffs: HashMap<PathBuf, GitDiff>,
    agent_icon: String,
    config: Config,
    theme: Theme,
}

struct BatchForm {
    fields: [String; 4],
    selected: usize,
    reuse: bool,
    preview: Option<batch::Preview>,
    error: Option<String>,
}

fn batch_details(workspaces: &[Workspace], redacted: bool) -> HashMap<String, String> {
    workspaces
        .iter()
        .map(|w| {
            let detail = match batch::for_window(&w.identity.window_id) {
                Ok(Some(batch)) => batch.display(redacted),
                _ => "Batch: unknown; source/destination unknown".into(),
            };
            (w.identity.window_id.clone(), detail)
        })
        .collect()
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
        let batches = HashMap::new();
        Self {
            batches,
            active_batch: None,
            batch_form: None,
            workspaces,
            visible,
            selected: 0,
            filter: String::new(),
            filtering: false,
            error: None,
            task: None,
            from_current: false,
            start_point: None,
            start_error: None,
            start_agent: agents()
                .iter()
                .position(|agent| *agent == config.default_agent)
                .unwrap_or(0),
            finishing: false,
            git_diffs,
            agent_icon: "A".into(),
            config,
            theme: Theme::rose_pine(variant),
        }
    }

    fn begin_start(&mut self) {
        self.task = Some(String::new());
        self.from_current = false;
        self.active_batch = None;
        match batch::current() {
            Ok(Some(batch)) => {
                self.start_point = Some(batch.source.clone());
                self.active_batch = Some(batch);
                self.start_error = None;
            }
            Ok(None) => {
                self.start_point = None;
                self.start_error = Some("Batch unknown; F3 choose source and destination".into());
                self.begin_batch();
            }
            Err(error) => {
                self.start_point = None;
                self.start_error = Some(error.to_string());
            }
        }
        self.error = None;
    }

    fn begin_batch(&mut self) {
        self.batch_form = Some(BatchForm {
            fields: ["base".into(), String::new(), String::new(), String::new()],
            selected: 0,
            reuse: false,
            preview: None,
            error: None,
        });
    }

    fn batch_key(&mut self, key: KeyCode) {
        if key == KeyCode::Esc {
            self.batch_form = None;
            return;
        }
        let Some(form) = &mut self.batch_form else {
            return;
        };
        match key {
            KeyCode::Tab | KeyCode::Down => form.selected = (form.selected + 1) % 4,
            KeyCode::BackTab | KeyCode::Up => form.selected = (form.selected + 3) % 4,
            KeyCode::F(2) => {
                form.reuse = !form.reuse;
                form.preview = None;
            }
            KeyCode::Backspace => {
                form.fields[form.selected].pop();
                form.preview = None;
            }
            KeyCode::Char(c) => {
                form.fields[form.selected].push(c);
                form.preview = None;
            }
            KeyCode::Enter => {
                let result = if let Some(preview) = &form.preview {
                    batch::create(preview).and_then(|batch| {
                        batch::select_current(&batch.id)?;
                        Ok(Some(batch))
                    })
                } else {
                    (|| {
                        let request = batch::Request {
                            repo: std::env::current_dir()?,
                            session: crate::navigation::current_session()?,
                            base: self.config.base_branch.clone(),
                            source: form.fields[0].clone(),
                            integration: (!form.fields[1].is_empty())
                                .then(|| form.fields[1].clone()),
                            destination_start: (!form.fields[2].is_empty())
                                .then(|| form.fields[2].clone()),
                            checkout: (!form.fields[3].is_empty())
                                .then(|| PathBuf::from(&form.fields[3])),
                            reuse_existing: form.reuse,
                        };
                        form.preview = Some(batch::preview(request)?);
                        Ok(None)
                    })()
                };
                match result {
                    Ok(Some(batch)) => {
                        self.start_point = Some(batch.source.clone());
                        self.start_error = None;
                        self.active_batch = Some(batch);
                        self.batch_form = None;
                        self.batches = batch_details(&self.workspaces, self.config.redact_labels);
                    }
                    Ok(None) => form.error = None,
                    Err(error) => {
                        form.error = Some(error.to_string());
                        form.preview = None;
                    }
                }
            }
            _ => {}
        }
    }

    fn resolve_start_point(&mut self) {
        let result = std::env::current_dir()
            .map_err(workspace::Error::from)
            .and_then(|repo| {
                workspace::resolve_start_point(&repo, &self.config.base_branch, self.from_current)
            });
        match result {
            Ok(point) => {
                self.start_point = Some(point);
                self.start_error = None;
            }
            Err(error) => {
                self.start_point = None;
                self.start_error = Some(error.to_string());
            }
        }
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
            &workspace.identity.window_name
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

pub fn run(variant: Variant, start: bool) -> Result<(), CockpitError> {
    let config = Config::load_tmux().map_err(|error| io::Error::other(error.to_string()))?;
    let mut app = App::new(discovery::discover()?, variant, config);
    app.batches = batch_details(&app.workspaces, app.config.redact_labels);
    if start {
        app.begin_start();
    }
    app.agent_icon = crate::icons::agent_icon();
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
        if app.batch_form.is_some() {
            app.batch_key(key.code);
            continue;
        }
        if let Some(task) = &mut app.task {
            match key.code {
                KeyCode::Esc => app.task = None,
                KeyCode::Backspace => {
                    task.pop();
                }
                KeyCode::Tab | KeyCode::Right => app.next_start_agent(),
                KeyCode::F(3) => app.begin_batch(),
                KeyCode::F(2) if app.active_batch.is_none() => {
                    app.from_current = !app.from_current;
                    app.resolve_start_point();
                }
                KeyCode::Char(character) => task.push(character),
                KeyCode::Enter if !task.trim().is_empty() => {
                    let Some(point) = app.start_point.clone() else {
                        continue;
                    };
                    let task_text = std::mem::take(task);
                    let slug = slug(&task_text);
                    app.task = None;
                    match workspace::start(Start {
                        name: None,
                        batch: app.active_batch.as_ref().map(|batch| batch.id.clone()),
                        repo: std::env::current_dir()?,
                        branch: format!("{}{slug}", app.config.branch_prefix),
                        start_point: point.commit,
                        root: None,
                        command: vec![app.start_agent().command().into()],
                    }) {
                        Ok(started) => {
                            if let Err(error) =
                                workspace::deliver_task(&started.window_id, &task_text)
                            {
                                app.error = Some(error.to_string());
                            } else {
                                match crate::navigation::open(Some(&started.window_id), None) {
                                    Ok(()) => return Ok(()),
                                    Err(error) => app.error = Some(format!("Open failed: {error}")),
                                }
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
            KeyCode::Char('b') => app.begin_batch(),
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
            KeyCode::Char('c') => {
                if let Some(workspace) = app.selected_workspace() {
                    match crate::coordinator::open_window(&workspace.identity.window_id) {
                        Ok(()) => return Ok(()),
                        Err(error) => app.error = Some(error.to_string()),
                    }
                }
            }
            KeyCode::Char('r') => {
                app.workspaces = discovery::discover()?;
                app.git_diffs = collect_git_diffs(&app.workspaces);
                app.batches = batch_details(&app.workspaces, app.config.redact_labels);
                app.refresh_filter();
            }
            KeyCode::Enter => {
                if let Some(workspace) = app.selected_workspace() {
                    match crate::navigation::open(Some(&workspace.identity.window_id), None) {
                        Ok(()) => return Ok(()),
                        Err(error) => app.error = Some(format!("Open failed: {error}")),
                    }
                }
            }
            _ => {}
        }
    }
}

fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    let footer_height = if let Some(error) = &app.error {
        let width = frame.area().width.max(1) as usize;
        // Leave room for word wrapping so retained launch resources are visible.
        let lines = (error.chars().count() + 10).div_ceil((width / 2).max(1));
        (lines as u16 + 2)
            .min(frame.area().height.saturating_sub(5))
            .max(3)
    } else if app.filtering || !app.filter.is_empty() {
        3
    } else {
        2
    };
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if area.width >= 70 && area.height >= 24 {
                9
            } else {
                3
            }),
            Constraint::Min(5),
            Constraint::Length(footer_height),
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
    if let Some(form) = &app.batch_form {
        let mut lines = vec![
            Line::from("BATCH SETUP · Tab field · Enter preview/confirm · Esc cancel"),
            Line::from(format!(
                "Direct destination: {} (blank branch); F2 reuse existing: {}",
                if app.config.redact_labels {
                    "[redacted]"
                } else {
                    &app.config.base_branch
                },
                form.reuse
            )),
        ];
        for (i, label) in [
            "Source (base / current / local ref)",
            "Integration branch (blank = direct)",
            "Destination start (blank = base)",
            "Dedicated checkout (blank = reuse)",
        ]
        .iter()
        .enumerate()
        {
            lines.push(Line::from(format!(
                "{} {}: {}",
                if i == form.selected { ">" } else { " " },
                label,
                if app.config.redact_labels {
                    "[redacted]"
                } else {
                    &form.fields[i]
                }
            )));
        }
        if let Some(preview) = &form.preview {
            lines.extend(
                preview
                    .display(app.config.redact_labels)
                    .lines()
                    .map(|line| Line::from(line.to_owned())),
            );
            lines.push(Line::from(
                "Enter: confirm this source and destination · Esc: cancel",
            ));
        } else {
            lines.push(Line::from(
                "Enter previews before any creation. Original branch is preserved.",
            ));
        }
        if let Some(error) = &form.error {
            lines.push(Line::from(if app.config.redact_labels {
                "Batch operation failed; details redacted".into()
            } else {
                error.clone()
            }));
        }
        let modal = centered(area, 110, 23);
        frame.render_widget(Clear, modal);
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().bg(app.theme.base).fg(app.theme.text)),
            modal,
        );
    } else if let Some(task) = &app.task {
        let modal = centered(area, 70, 15);
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
        let mut text = vec![
            Line::styled(
                "START WORKSPACE",
                Style::default()
                    .fg(app.theme.rose)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::from(""),
            Line::from(format!("Task   {task_label}")),
            Line::from(format!("Branch {branch_label}")),
            Line::from(if app.active_batch.is_some() {
                "Source Pinned batch (F3 new batch)"
            } else if app.from_current {
                "Base   Continue from current branch (F2 change)"
            } else {
                "Base   Independent task (F2 change)"
            }),
            Line::from(ellipsize(
                &if app.config.redact_labels {
                    "       [redacted]".to_owned()
                } else if let Some(point) = &app.start_point {
                    format!("       {} ({})", point.reference, &point.commit[..12])
                } else {
                    app.start_error
                        .clone()
                        .unwrap_or_else(|| "Base unavailable".into())
                },
                label_width,
            )),
            Line::from("Local ref; remote freshness unknown"),
            Line::from(format!(
                "Agent  {}  (Tab change)",
                app.start_agent().label()
            )),
            Line::from(""),
            if app.start_point.is_some() {
                if app.active_batch.is_some() {
                    ui::action_line(
                        &[
                            &[("Enter", "Create"), ("Tab", "Agent"), ("F3", "Batch")],
                            CANCEL_ACTION,
                        ],
                        app.theme,
                    )
                } else {
                    ui::action_line(&[CREATE_ACTIONS, CANCEL_ACTION], app.theme)
                }
            } else {
                ui::action_line(&[CREATE_BLOCKED_ACTIONS, CANCEL_ACTION], app.theme)
            },
        ];
        if let Some(batch) = &app.active_batch {
            text.insert(
                8,
                Line::from(if app.config.redact_labels {
                    "Batch pinned · Destination [redacted]".into()
                } else {
                    format!(
                        "Batch pinned · Destination {} · F3 new batch",
                        batch.destination
                    )
                }),
            );
        } else {
            text.insert(
                8,
                Line::from("Batch unknown · F3 choose source and destination"),
            );
        }
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
            ui::action_line(&[FINISH_ACTION, FINISH_CANCEL_ACTION], app.theme),
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
    if area.height >= 9 {
        frame.render_widget(Block::default().borders(Borders::BOTTOM), area);
        crate::brand::render(
            frame,
            Rect::new(area.x + 1, area.y, 16, 8),
            // A dark tile keeps the white artwork visible in the Dawn theme too.
            ratatui::style::Color::Rgb(35, 33, 54),
        );
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "Drudwyn",
                    Style::default()
                        .fg(app.theme.rose)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from("WORKSPACE COCKPIT"),
                Line::from(Span::styled(
                    format!(
                        "{} live · {} need attention",
                        app.workspaces.iter().filter(|w| w.is_agent()).count(),
                        attention
                    ),
                    Style::default().fg(app.theme.muted),
                )),
            ]),
            Rect::new(area.x + 19, area.y + 2, area.width.saturating_sub(19), 4),
        );
        return;
    }
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
                app.workspaces.iter().filter(|w| w.is_agent()).count(),
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
            Span::raw(format!(
                "{} {} · ",
                workspace.role(),
                workspace.identity.window_id
            )),
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
                format!(" {} {} ", app.agent_icon.as_str(), workspace.agent.label()),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(workspace.lifecycle.label(), Style::default().fg(color)),
            Span::styled(
                format!(" · {}", age(workspace.state_since)),
                Style::default().fg(app.theme.muted),
            ),
        ]),
        Line::from(""),
        detail_line("ROLE", workspace.role().into(), app.theme.muted),
        detail_line(
            "IDENTITY",
            format!(
                "{} · project {}",
                workspace.identity.window_id,
                workspace.project.as_deref().unwrap_or("unknown")
            ),
            app.theme.muted,
        ),
        detail_line(
            "COORD",
            match &workspace.coordinator {
                Some(id) if workspace.coordinator_available => format!("{id} · c return"),
                Some(id) => format!("{id} unavailable · recover with coordinator set"),
                None => "unknown · coordinator set to associate".into(),
            },
            app.theme.muted,
        ),
        detail_line("TMUX", format!("{session} · {window}"), app.theme.muted),
        detail_line("PATH", path, app.theme.muted),
        detail_line("BRANCH", branch.to_owned(), app.theme.muted),
        detail_line("CHECKOUT", format!("{checkout} · {git}"), app.theme.muted),
    ];
    lines.extend(
        app.batches
            .get(&workspace.identity.window_id)
            .map(String::as_str)
            .unwrap_or("Batch: unknown; source/destination unknown")
            .lines()
            .map(|line| Line::from(format!(" {line}"))),
    );
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
    if app.filtering {
        let message = format!("› {}_", app.filter);
        ui::render_footer(
            frame,
            area,
            app.theme,
            &[FILTER_ACTIONS],
            ("FILTER", &message, FooterTone::Info),
        );
        return;
    }

    if app.error.is_none() && app.filter.is_empty() {
        ui::render_action_bar(
            frame,
            area,
            app.theme,
            &[COCKPIT_NAVIGATION, COCKPIT_ACTIONS, CLOSE_ACTION],
        );
        return;
    }

    if let Some(error) = &app.error {
        let message = if app.config.redact_labels {
            if error.starts_with("launch failed:") {
                "launch failed; retained resources [redacted]. Disable label redaction to inspect recovery details."
            } else {
                "Operation failed; details hidden by label redaction."
            }
        } else {
            error.as_str()
        };
        frame.render_widget(
            Paragraph::new(vec![
                ui::action_line(
                    &[COCKPIT_NAVIGATION, COCKPIT_ACTIONS, CLOSE_ACTION],
                    app.theme,
                ),
                Line::from(format!(" ERROR  {message}")),
            ])
            .style(Style::default().fg(app.theme.love))
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::TOP)),
            area,
        );
        return;
    }
    let (label, message, tone) = ("FILTER", app.filter.as_str(), FooterTone::Info);
    ui::render_footer(
        frame,
        area,
        app.theme,
        &[COCKPIT_NAVIGATION, COCKPIT_ACTIONS, CLOSE_ACTION],
        (label, message, tone),
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
                sessions: vec!["dev".into()],
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
            project: None,
            coordinator: None,
            coordinator_available: false,
            agent: AgentKind::Codex,
            lifecycle: state,
            evidence: EvidenceSource::Hook,
            state_since: Some(1),
            attention_since: None,
        }
    }

    #[test]
    fn batch_start_labels_the_pinned_source_without_claiming_a_base_choice() {
        let mut app = App::new(vec![], Variant::Moon, Config::default());
        let source = workspace::StartPoint {
            reference: "refs/heads/planning".into(),
            commit: "123456789abcdef0123456789abcdef0123456789a".into(),
        };
        app.start_point = Some(source.clone());
        app.task = Some("example".into());
        app.active_batch = Some(batch::Batch {
            id: "$1/123".into(),
            project: "$1".into(),
            coordinator: "@1".into(),
            repository: "/repo/.git".into(),
            source,
            destination: "assembled".into(),
            destination_commit: "abcdef".into(),
            checkout: "/assembled".into(),
        });
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let content = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("Source Pinned batch"));
        assert!(!content.contains("Independent task"));
    }

    #[test]
    fn batch_destination_is_visible_in_normal_height_details() {
        let mut app = App::new(
            vec![workspace("planning", Lifecycle::Working)],
            Variant::Moon,
            Config::default(),
        );
        app.batches.insert(
            "@1".into(),
            "Batch: $1/123
Source: refs/heads/planning abcdef
Destination: assembled 123456 (at setup)
Checkout: /repo/assembled"
                .into(),
        );
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let content = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("Destination: assembled"));
    }

    #[test]
    fn batch_setup_keys_preview_and_cancel_without_changing_the_launch() {
        let mut app = App::new(vec![], Variant::Moon, Config::default());
        app.begin_batch();
        app.batch_key(KeyCode::Tab);
        for c in "integration/private".chars() {
            app.batch_key(KeyCode::Char(c));
        }
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let content = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("BATCH SETUP"));
        assert!(content.contains("integration/private"));
        assert!(content.contains("base / current / local ref"));
        assert!(content.contains("Dedicated checkout"));
        app.config.redact_labels = true;
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let content = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(!content.contains("integration/private"));
        app.batch_key(KeyCode::Esc);
        assert!(app.batch_form.is_none());
        assert!(app.active_batch.is_none());
    }

    #[test]
    fn coordinator_details_keep_role_identity_recovery_and_redacted_names() {
        let mut coordinator = workspace("private-plan", Lifecycle::Unknown);
        coordinator.identity.window_name = "private-name".into();
        coordinator.agent = AgentKind::Unknown;
        coordinator.project = Some("$7".into());
        coordinator.coordinator = Some("@1".into());
        coordinator.coordinator_available = true;
        let mut worker = workspace("private-worker", Lifecycle::Working);
        worker.identity.window_id = "@2".into();
        worker.project = Some("$7".into());
        worker.coordinator = Some("@1".into());
        worker.coordinator_available = false;
        for redacted in [false, true] {
            let mut app = App::new(
                vec![coordinator.clone(), worker.clone()],
                Variant::Moon,
                Config::default(),
            );
            app.config.redact_labels = redacted;
            for selected in [0, 1] {
                app.selected = selected;
                let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
                terminal.draw(|frame| render(frame, &app)).unwrap();
                let content = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>();
                assert!(
                    content.contains("1 live"),
                    "coordinator shell counted as an agent"
                );
                assert!(content.contains("project $7"));
                if selected == 0 {
                    assert!(content.contains("Coordinator shell"));
                    assert!(content.contains("@1 · c return"));
                } else {
                    assert!(content.contains("@1 unavailable"));
                    assert!(content.contains("recover with coordinator set"));
                }
                if redacted {
                    assert!(!content.contains("private-"));
                }
            }
        }
    }

    #[test]
    fn launch_failure_displays_retained_identity_and_respects_redaction() {
        let mut app = App::new(vec![], Variant::Moon, Config::default());
        app.error = Some("launch failed: tmux operation failed: injected metadata attachment failure; retained worktree /private/repository/worktrees/worker-files on branch work/private-worker. Inspect the retained checkout before retrying".into());
        for redacted in [false, true] {
            app.config.redact_labels = redacted;
            let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
            terminal.draw(|frame| render(frame, &app)).unwrap();
            let content = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(content.contains("launch failed"));
            assert_eq!(
                content.contains("/private/repository/worktrees/worker-files"),
                !redacted
            );
            assert_eq!(content.contains("work/private-worker"), !redacted);
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

        let modal = centered(Rect::new(0, 0, 100, 24), 70, 15);
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

        let modal = centered(Rect::new(0, 0, 100, 24), 70, 15);
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
    #[test]
    fn start_form_shows_base_commit_and_hides_it_when_redacted() {
        let mut app = App::new(vec![], Variant::Moon, Config::default());
        app.task = Some("Example task".into());
        app.start_point = Some(workspace::StartPoint {
            reference: "refs/remotes/origin/private-base".into(),
            commit: "123456789abcdef0123456789abcdef0123456789a".into(),
        });
        for redacted in [false, true] {
            app.config.redact_labels = redacted;
            let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
            terminal.draw(|frame| render(frame, &app)).unwrap();
            let content = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(content.contains("Independent task (F2 change)"));
            assert!(content.contains("remote freshness unknown"));
            assert_eq!(content.contains("private-base"), !redacted);
            assert_eq!(content.contains("123456789abc"), !redacted);
        }
        app.from_current = true;
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let content = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(content.contains("Continue from current branch"));
    }
}
