//! Grouped, content-blind replacement for tmux `choose-tree`.

use std::{
    io,
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
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    domain::{AgentKind, Lifecycle},
    theme::{Theme, Variant},
    ui::{self, FooterTone},
};

const SEP: char = '\u{241f}';
const FORMAT: &str = "#{session_name}␟#{window_id}␟#{window_index}␟#{window_name}␟#{pane_current_command}␟#{@drudwyn_state}␟#{@drudwyn_since}␟#{@drudwyn_branch}";
const NAVIGATION_ACTIONS: &[(&str, &str)] = &[("j/k", "Move"), ("Enter", "Jump")];
const WORKSPACE_ACTIONS: &[(&str, &str)] = &[
    ("n", "New Session"),
    ("t", "Show"),
    ("d", "Details"),
    ("c", "Coordinator"),
    ("r", "Rename"),
    ("x", "Kill"),
    ("s", "Save"),
    ("/", "Filter"),
];
const CLOSE_ACTION: &[(&str, &str)] = &[("Esc", "Close")];
const CONFIRM_ACTION: &[(&str, &str)] = &[("y", "Confirm")];
const CANCEL_ACTION: &[(&str, &str)] = &[("Esc/n", "Cancel")];
const EDIT_ACTIONS: &[(&str, &str)] = &[("Enter", "Apply"), ("Backspace", "Delete")];
const FILTER_ACTIONS: &[(&str, &str)] = &[
    ("text", "Filter"),
    ("Backspace", "Delete"),
    ("Enter/Esc", "Done"),
];

#[derive(Clone)]
struct Window {
    session: String,
    id: String,
    index: String,
    name: String,
    agent: AgentKind,
    managed: bool,
    lifecycle: Lifecycle,
    since: Option<u64>,
    branch: Option<String>,
    role: String,
    evidence: String,
    tool: String,
}

#[derive(Clone)]
struct App {
    windows: Vec<Window>,
    visible: Vec<usize>,
    selected: usize,
    kind: u8,
    details: Option<u16>,
    new_session: Option<ui::ShellForm>,
    filter: String,
    filtering: bool,
    pending_kill: Option<Window>,
    pending_rename: Option<(String, String)>,
    notice: Option<String>,
    theme: Theme,
    agent_icon: String,
    shell_icon: String,
    redact: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NavigationAction {
    Continue,
    Close,
    Jump,
    Coordinator,
    Kill,
    Save,
    Rename,
    New,
    Create,
}

impl App {
    fn refresh_visible(&mut self) {
        let selected = self.visible.get(self.selected).copied();
        let query = self.filter.to_lowercase();
        self.visible = self
            .windows
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                (self.kind == 0 || (self.kind == 1) == item.managed)
                    && (query.is_empty()
                        || item.name.to_lowercase().contains(&query)
                        || item.session.to_lowercase().contains(&query)
                        || item
                            .branch
                            .as_deref()
                            .is_some_and(|branch| branch.to_lowercase().contains(&query))
                        || item.agent.label().to_lowercase().contains(&query)
                        || item.lifecycle.label().to_lowercase().contains(&query))
            })
            .map(|(index, _)| index)
            .collect();
        self.selected = selected
            .and_then(|index| self.visible.iter().position(|item| *item == index))
            .unwrap_or(0);
    }

    fn move_selection(&mut self, delta: isize) {
        if !self.visible.is_empty() {
            self.selected = self
                .selected
                .saturating_add_signed(delta)
                .min(self.visible.len() - 1);
        }
    }
}

fn handle_key(app: &mut App, code: KeyCode) -> NavigationAction {
    if let Some(form) = &mut app.new_session {
        match form.edit(code) {
            ui::FormAction::Cancel => {
                app.new_session = None;
                app.notice = None;
            }
            ui::FormAction::Create => return NavigationAction::Create,
            ui::FormAction::Edit => app.notice = None,
        }
        return NavigationAction::Continue;
    }
    if let Some(scroll) = &mut app.details {
        match code {
            KeyCode::Esc | KeyCode::Char('d') | KeyCode::Char('q') => app.details = None,
            KeyCode::Char('j') | KeyCode::Down => *scroll = scroll.saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => *scroll = scroll.saturating_sub(1),
            _ => {}
        }
        return NavigationAction::Continue;
    }
    if app.pending_kill.is_some() {
        return match code {
            KeyCode::Char('y') => NavigationAction::Kill,
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => {
                app.pending_kill = None;
                NavigationAction::Continue
            }
            _ => NavigationAction::Continue,
        };
    }
    if let Some((_, name)) = &mut app.pending_rename {
        match code {
            KeyCode::Esc => {
                app.pending_rename = None;
                app.notice = None;
            }
            KeyCode::Enter if !name.trim().is_empty() => return NavigationAction::Rename,
            KeyCode::Backspace => {
                name.pop();
                app.notice = None;
            }
            KeyCode::Char(character) => {
                name.push(character);
                app.notice = None;
            }
            _ => {}
        }
        return NavigationAction::Continue;
    }
    app.notice = None;
    if app.filtering {
        match code {
            KeyCode::Esc => app.filtering = false,
            KeyCode::Enter => {
                app.filtering = false;
                return NavigationAction::Jump;
            }
            KeyCode::Backspace => {
                app.filter.pop();
                app.refresh_visible();
            }
            KeyCode::Char(character) => {
                app.filter.push(character);
                app.refresh_visible();
            }
            _ => {}
        }
        return NavigationAction::Continue;
    }
    match code {
        KeyCode::Esc | KeyCode::Char('q') => NavigationAction::Close,
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_selection(1);
            NavigationAction::Continue
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_selection(-1);
            NavigationAction::Continue
        }
        KeyCode::Char('/') => {
            app.filtering = true;
            NavigationAction::Continue
        }
        KeyCode::Char('n') => NavigationAction::New,
        KeyCode::Char('d') => {
            app.details = Some(0);
            NavigationAction::Continue
        }
        KeyCode::Char('t') => {
            app.kind = (app.kind + 1) % 3;
            app.refresh_visible();
            NavigationAction::Continue
        }
        KeyCode::Char('r') => {
            app.pending_rename = app
                .visible
                .get(app.selected)
                .and_then(|index| app.windows.get(*index))
                .map(|item| (item.id.clone(), item.name.clone()));
            NavigationAction::Continue
        }
        KeyCode::Char('c') => NavigationAction::Coordinator,
        KeyCode::Char('s') => NavigationAction::Save,
        KeyCode::Char('x') => {
            app.pending_kill = app
                .visible
                .get(app.selected)
                .and_then(|index| app.windows.get(*index))
                .cloned();
            NavigationAction::Continue
        }
        KeyCode::Enter => NavigationAction::Jump,
        _ => NavigationAction::Continue,
    }
}

pub fn run(variant: Variant) -> io::Result<()> {
    let current = crate::navigation::context("#{window_id}")?;
    let windows = discover()?;
    let selected = windows
        .iter()
        .position(|item| item.id == current)
        .unwrap_or(0);
    let (agent_icon, shell_icon) = crate::icons::workspace_icons();
    let mut app = App {
        visible: (0..windows.len()).collect(),
        windows,
        selected,
        kind: 0,
        details: None,
        new_session: None,
        filter: String::new(),
        filtering: false,
        pending_kill: None,
        pending_rename: None,
        notice: None,
        theme: Theme::rose_pine(variant),
        agent_icon,
        shell_icon,
        redact: tmux_output(&["show-option", "-gqv", "@drudwyn-redact-labels"])? == "on",
    };
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let result = event_loop(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
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
        match handle_key(app, key.code) {
            NavigationAction::Close => return Ok(()),
            NavigationAction::New => match ui::ShellForm::from_current() {
                Ok(form) => app.new_session = Some(form),
                Err(error) => app.notice = Some(format!("New Session failed: {error}")),
            },
            NavigationAction::Create => {
                if let Some(form) = &app.new_session {
                    match crate::session::create(
                        &form.name,
                        Some(std::path::Path::new(&form.directory)),
                    ) {
                        Ok(_) => return Ok(()),
                        Err(error) => app.notice = Some(format!("Create failed: {error}")),
                    }
                }
            }
            NavigationAction::Jump => {
                if let Some(item) = app
                    .visible
                    .get(app.selected)
                    .and_then(|index| app.windows.get(*index))
                {
                    match crate::navigation::open(Some(&item.id), None) {
                        Ok(()) => return Ok(()),
                        Err(error) => app.notice = Some(format!("Jump failed: {error}")),
                    }
                }
            }
            NavigationAction::Coordinator => {
                if let Some(item) = app
                    .visible
                    .get(app.selected)
                    .and_then(|i| app.windows.get(*i))
                {
                    match crate::coordinator::open_window(&item.id) {
                        Ok(()) => return Ok(()),
                        Err(error) => app.notice = Some(error.to_string()),
                    }
                }
            }
            NavigationAction::Rename => {
                if let Some((id, name)) = app.pending_rename.clone() {
                    match tmux_output(&["rename-window", "-t", &id, "--", &name]) {
                        Ok(_) => {
                            for item in app.windows.iter_mut().filter(|item| item.id == id) {
                                item.name = name.clone();
                            }
                            app.pending_rename = None;
                            app.refresh_visible();
                            app.notice = Some("Renamed · press s to save".into());
                        }
                        Err(error) => app.notice = Some(format!("Rename failed: {error}")),
                    }
                }
            }
            NavigationAction::Save => {
                app.notice = Some("Saving via tmux-resurrect…".into());
                terminal.draw(|frame| render(frame, app))?;
                app.notice = Some(crate::persistence::save());
            }
            NavigationAction::Kill => {
                if let Some(target) = app.pending_kill.take() {
                    match tmux_output(&["kill-window", "-t", &target.id]) {
                        Ok(_) => {
                            app.notice = Some("Killed · press s to save cleanup".into());
                            app.windows.retain(|item| item.id != target.id);
                            match discover() {
                                Ok(items) => app.windows = items,
                                Err(error) if !app.windows.is_empty() => {
                                    app.notice = Some(format!("Refresh failed: {error}"));
                                }
                                Err(_) => {}
                            }
                            app.refresh_visible();
                            if app.windows.is_empty() {
                                return Ok(());
                            }
                        }
                        Err(error) => app.notice = Some(format!("Kill failed: {error}")),
                    }
                }
            }
            NavigationAction::Continue => {}
        }
    }
}

fn render(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    // Redact only the projection: selection, filtering and tmux targets retain
    // their real identities for navigation and explicit user actions.
    let original = app;
    let app = &*original;
    let mut projection;
    let app = if app.redact {
        projection = app.clone();
        for item in &mut projection.windows {
            item.name = format!("Workspace {}", item.index);
            item.session = "private".into();
            item.branch = item.branch.as_ref().map(|_| "private".into());
        }
        if let Some(target) = &mut projection.pending_kill {
            target.name = "Workspace".into();
            target.session = "private".into();
        }
        if let Some((_, name)) = &mut projection.pending_rename {
            *name = "[redacted]".into();
        }
        if let Some(form) = &mut projection.new_session {
            form.name = "[redacted]".into();
            form.directory = "[redacted]".into();
        }
        if projection.filtering {
            projection.filter = "[redacted]".into();
        }
        if projection.notice.is_some() {
            projection.notice = Some(
                if app
                    .notice
                    .as_deref()
                    .is_some_and(|message| message.to_lowercase().contains("fail"))
                {
                    "Action failed; details hidden while labels are redacted".into()
                } else {
                    "Status details hidden while labels are redacted".into()
                },
            );
        }
        &projection
    } else {
        app
    };
    let area = frame.area();
    let footer_height = if app.pending_kill.is_some()
        || app.pending_rename.is_some()
        || app.notice.is_some()
        || app.filtering
    {
        3
    } else {
        ui::action_lines(
            &[NAVIGATION_ACTIONS, WORKSPACE_ACTIONS, CLOSE_ACTION],
            app.theme,
            area.width,
        )
        .len() as u16
            + 1
    };
    frame.render_widget(
        Block::default().style(Style::default().fg(app.theme.text).bg(app.theme.base)),
        area,
    );
    if let Some(form) = &app.new_session {
        ui::shell_form(frame, form, app.theme, app.notice.as_deref());
        return;
    }
    let groups = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(footer_height),
    ])
    .split(area);
    let agents = app.windows.iter().filter(|item| item.managed).count();
    let attention = app
        .windows
        .iter()
        .filter(|item| item.lifecycle.needs_attention())
        .count();
    ui::masthead(
        frame,
        groups[0],
        app.theme,
        "WORKSPACE NAVIGATOR",
        &format!(
            "{} windows · {} agents · {} need you",
            app.windows.len(),
            agents,
            attention
        ),
    );

    ui::navigation_toolbar(
        frame,
        groups[1],
        app.theme,
        &format!(
            "{} shown / {} total · [/] Search · {}",
            app.visible.len(),
            app.windows.len(),
            ["All workspaces", "Agents & workers", "Shells & editors"][app.kind as usize]
        ),
    );
    let body = groups[2];
    let mut clamped_scroll = None;
    if app.details.is_some() {
        clamped_scroll = Some(render_inspection(frame, app, body));
    } else if body.width >= 112 {
        let panes = Layout::horizontal([Constraint::Min(60), Constraint::Length(34)]).split(body);
        render_windows(frame, app, panes[0]);
        render_inspection(frame, app, panes[1]);
    } else {
        render_windows(frame, app, body);
    }
    if let Some(target) = &app.pending_kill {
        let message = format!(
            "Kill {} {}:{} ({})? Stops all panes and processes.",
            target.id, target.session, target.index, target.name
        );
        ui::render_footer(
            frame,
            groups[3],
            app.theme,
            &[CONFIRM_ACTION, CANCEL_ACTION],
            ("CONFIRM", &message, FooterTone::Warning),
        );
    } else if let Some((_, name)) = &app.pending_rename {
        let message = app
            .notice
            .as_deref()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Rename window › {name}_"));
        ui::render_footer(
            frame,
            groups[3],
            app.theme,
            &[EDIT_ACTIONS, CANCEL_ACTION],
            ("RENAME", &message, FooterTone::Info),
        );
    } else if let Some(notice) = &app.notice {
        ui::render_footer(
            frame,
            groups[3],
            app.theme,
            &[NAVIGATION_ACTIONS, WORKSPACE_ACTIONS, CLOSE_ACTION],
            ("STATUS", notice, FooterTone::Info),
        );
    } else if app.filtering {
        let message = format!("› {}_", app.filter);
        ui::render_footer(
            frame,
            groups[3],
            app.theme,
            &[FILTER_ACTIONS],
            ("FILTER", &message, FooterTone::Info),
        );
    } else {
        frame.render_widget(
            Paragraph::new(ui::action_lines(
                &[NAVIGATION_ACTIONS, WORKSPACE_ACTIONS, CLOSE_ACTION],
                app.theme,
                area.width,
            ))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(app.theme.line())),
            ),
            groups[3],
        );
    }
    if let Some(offset) = clamped_scroll {
        original.details = Some(offset);
    }
}

fn role(item: &Window) -> &str {
    if item.role.starts_with("Coordinator") {
        "COORD"
    } else if item.role == "Worktree worker" {
        "WT"
    } else if item.managed {
        "AGENT"
    } else if matches!(item.tool.as_str(), "vim" | "nvim" | "emacs" | "nano" | "hx") {
        "EDIT"
    } else {
        "SH"
    }
}
fn window_columns(width: usize) -> (usize, usize, usize, usize) {
    let tool = if width >= 72 { 14 } else { 0 };
    let activity = if width >= 48 { 16 } else { 0 };
    let branch = if width >= 100 { (width / 3).min(42) } else { 0 };
    let name = width.saturating_sub(tool + activity + branch + 2);
    (name, tool, activity, branch)
}
fn render_windows(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let theme = app.theme;
    let muted = Style::default().fg(theme.subtle());
    let (name, tool, activity, branch) = window_columns(area.width as usize);
    let mut headers = vec![Span::raw("  "), ui::cell("WORKSPACE", name, muted)];
    if tool > 0 {
        headers.push(ui::cell("AGENT / TOOL", tool, muted));
    }
    if activity > 0 {
        headers.push(ui::cell("ACTIVITY", activity, muted));
    }
    if branch > 0 {
        headers.push(ui::cell("BRANCH", branch, muted));
    }
    frame.render_widget(
        Paragraph::new(Line::from(headers)).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.line())),
        ),
        Rect::new(area.x, area.y, area.width, 2),
    );
    let body = Rect::new(
        area.x,
        area.y + 2,
        area.width,
        area.height.saturating_sub(2),
    );
    let agents: Vec<_> = app
        .visible
        .iter()
        .copied()
        .filter(|i| app.windows[*i].managed)
        .collect();
    let manual: Vec<_> = app
        .visible
        .iter()
        .copied()
        .filter(|i| !app.windows[*i].managed)
        .collect();
    if agents.is_empty() && manual.is_empty() {
        frame.render_widget(
            Paragraph::new(" No matching workspaces. [/] Change search; [t] Show all.")
                .style(muted),
            body,
        );
    } else if agents.is_empty() {
        render_window_group(frame, app, body, &manual, "MANUAL SHELLS / EDITORS");
    } else if manual.is_empty() {
        render_window_group(frame, app, body, &agents, "AGENTS / WORKERS");
    } else {
        // Separate scroll regions keep agents visible even when a manual shell
        // is selected. Keyboard order still follows the same agents-first list.
        let height = (agents.len() as u16 * 3 + 2)
            .min(body.height * 2 / 3)
            .max(3)
            .min(body.height);
        let groups = Layout::vertical([Constraint::Length(height), Constraint::Min(3)]).split(body);
        render_window_group(frame, app, groups[0], &agents, "AGENTS / WORKERS");
        render_window_group(frame, app, groups[1], &manual, "MANUAL SHELLS / EDITORS");
    }
}
fn render_window_group(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    area: Rect,
    indices: &[usize],
    title: &str,
) {
    let theme = app.theme;
    let muted = Style::default().fg(theme.subtle());
    let width = area.width as usize;
    let (name, tool, activity, branch) = window_columns(width);
    let block = Block::default()
        .borders(Borders::TOP)
        .title(format!(" {title} "))
        .title_style(Style::default().fg(theme.accent()))
        .border_style(Style::default().fg(theme.line()));
    let body = block.inner(area);
    frame.render_widget(block, area);
    let compact = body.height < 3;
    let mut items = Vec::new();
    let mut selected = None;
    let mut session = None;
    for index in indices {
        let item = &app.windows[*index];
        if !compact && session != Some(item.session.as_str()) {
            session = Some(&item.session);
            let count = indices
                .iter()
                .filter(|i| app.windows[**i].session == item.session)
                .count();
            items.push(ListItem::new(Line::styled(
                ui::ellipsize(
                    &format!(" session {} · {count} shown", item.session),
                    width.saturating_sub(2),
                ),
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            )));
        }
        if app.visible.get(app.selected) == Some(index) {
            selected = Some(items.len());
        }
        let label = format!(
            "{}  {} {}  {}",
            item.index,
            if item.managed {
                &app.agent_icon
            } else {
                &app.shell_icon
            },
            role(item),
            item.name
        );
        let mut cells = vec![ui::cell(
            &label,
            name,
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        )];
        if tool > 0 {
            cells.push(ui::cell(&item.tool, tool, muted));
        }
        if activity > 0 {
            let label = if item.managed {
                ui::activity(item.lifecycle)
            } else if role(item) == "EDIT" {
                "EDITOR"
            } else {
                "SHELL"
            };
            cells.push(Span::styled(
                format!(" {label} "),
                if item.managed {
                    ui::activity_style(item.lifecycle, theme)
                } else {
                    muted
                },
            ));
            cells.push(Span::raw(
                " ".repeat(activity.saturating_sub(label.len() + 2)),
            ));
        }
        if branch > 0 {
            cells.push(ui::cell(
                item.branch.as_deref().unwrap_or("—"),
                branch,
                muted,
            ));
        }
        let mut lines = vec![Line::from(cells)];
        if !compact {
            lines.push(Line::default());
        }
        items.push(ListItem::new(lines));
    }
    let mut state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(
        List::new(items)
            .highlight_spacing(ratatui::widgets::HighlightSpacing::Always)
            .highlight_symbol("▎ ")
            .highlight_style(Style::default().bg(theme.selection())),
        body,
        &mut state,
    );
}
fn render_inspection(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) -> u16 {
    if let Some(item) = app.visible.get(app.selected).map(|i| &app.windows[*i]) {
        ui::inspection(
            frame,
            area,
            app.theme,
            "SELECTED WORKSPACE",
            &item.name,
            &[
                ("Role", item.role.clone()),
                ("Session", item.session.clone()),
                ("Agent / tool", item.tool.clone()),
                (
                    "Branch",
                    item.branch
                        .clone()
                        .unwrap_or_else(|| "Not available".into()),
                ),
                (
                    "Activity evidence",
                    if item.managed {
                        format!(
                            "{} · {} · {}",
                            ui::activity(item.lifecycle),
                            item.evidence,
                            age(item.since)
                        )
                    } else {
                        "Shell/editor; no agent activity inferred".into()
                    },
                ),
            ],
            app.details,
        )
    } else {
        0
    }
}

fn discover() -> io::Result<Vec<Window>> {
    let output = tmux_output(&["list-windows", "-a", "-F", FORMAT])?;
    let mut windows = parse_windows(&output);
    let workspaces = crate::discovery::discover().map_err(io::Error::other)?;
    for window in &mut windows {
        if let Some(workspace) = workspaces
            .iter()
            .find(|w| w.identity.window_id == window.id)
        {
            window.role = workspace.role().into();
            window.branch = workspace.checkout.branch.clone();
            window.agent = workspace.agent;
            if workspace.is_agent() {
                window.tool = workspace.agent.label().into();
            }
            window.lifecycle = workspace.lifecycle;
            window.since = workspace.state_since;
            window.managed = workspace.is_agent();
            window.evidence = format!(
                "{}{}",
                workspace.evidence.label(),
                match workspace.process.as_str() {
                    "exited" => format!(
                        " · EXIT {}",
                        workspace
                            .exit_code
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "?".into())
                    ),
                    "ambiguous" => " · ambiguous".into(),
                    _ => String::new(),
                }
            );
        }
    }
    let aliases = crate::navigation::view_names()?;
    for window in &mut windows {
        if let Some(name) = aliases.get(&window.session) {
            window.session = name.clone();
        }
    }
    let mut seen = std::collections::HashSet::new();
    windows.retain(|window| seen.insert(window.id.clone()));
    windows.sort_by_key(|item| {
        (
            !item.managed,
            item.session.clone(),
            item.index.parse::<u32>().unwrap_or(u32::MAX),
        )
    });
    Ok(windows)
}

fn parse_windows(output: &str) -> Vec<Window> {
    output
        .lines()
        .filter_map(|line| {
            let fields = line.split(SEP).collect::<Vec<_>>();
            (fields.len() == 8).then(|| {
                let agent = AgentKind::from_command(fields[4]).unwrap_or(AgentKind::Unknown);
                let lifecycle = Lifecycle::from_tmux(fields[5]);
                Window {
                    session: fields[0].into(),
                    id: fields[1].into(),
                    index: fields[2].into(),
                    name: fields[3].into(),
                    agent,
                    role: if agent != AgentKind::Unknown || lifecycle != Lifecycle::Unknown {
                        "Ordinary agent"
                    } else {
                        "Shell"
                    }
                    .into(),
                    managed: agent != AgentKind::Unknown || lifecycle != Lifecycle::Unknown,
                    evidence: "unknown".into(),
                    tool: fields[4].into(),
                    lifecycle,
                    since: fields[6].parse().ok(),
                    branch: (!fields[7].is_empty()).then(|| fields[7].into()),
                }
            })
        })
        .collect()
}

fn tmux_output(args: &[&str]) -> io::Result<String> {
    let output = Command::new("tmux").args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim_end().into())
    } else {
        Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

fn age(since: Option<u64>) -> String {
    let Some(since) = since else {
        return String::new();
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let elapsed = now.saturating_sub(since);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_and_window_numbers_remain_visible_at_narrow_width_with_redaction() {
        for role in [
            "Worktree worker",
            "Ordinary agent",
            "Coordinator agent",
            "Coordinator shell",
            "Shell",
        ] {
            for width in [48, 64, 80, 120, 160] {
                let mut app = populated_app();
                app.windows.truncate(1);
                app.visible = vec![0];
                app.windows[0].role = role.into();
                app.windows[0].managed = !matches!(role, "Shell" | "Coordinator shell");
                app.redact = true;
                let mut terminal =
                    Terminal::new(ratatui::backend::TestBackend::new(width, 24)).unwrap();
                terminal.draw(|frame| render(frame, &mut app)).unwrap();
                let content = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|c| c.symbol())
                    .collect::<String>();
                assert!(
                    content.contains(match role {
                        "Worktree worker" => "WT",
                        "Coordinator shell" | "Coordinator agent" => "COORD",
                        "Ordinary agent" => "AGENT",
                        _ => "SH",
                    }),
                    "missing {role} at {width}: {content}"
                );
                assert!(!content.contains("@1"));
                assert!(content.contains("1  "));
                assert!(!content.contains("first"));
            }
        }
    }

    #[test]
    fn discovery_includes_shells_and_groups_lifecycle_owned_windows_as_agents() {
        let sep = SEP;
        let input = format!(
            "dev{sep}@1{sep}1{sep}shell{sep}zsh{sep}{sep}{sep}\n\
             dev{sep}@2{sep}2{sep}review{sep}zsh{sep}done{sep}100{sep}work/ui\n\
             dev{sep}@3{sep}3{sep}agent{sep}codex{sep}working{sep}101{sep}work/api\n"
        );
        let windows = parse_windows(&input);
        assert_eq!(windows.len(), 3);
        assert!(!windows[0].managed);
        assert!(windows[1].managed);
        assert!(windows[2].managed);
        assert_eq!(windows[2].agent, AgentKind::Codex);
    }

    #[test]
    fn navigator_discovery_format_is_content_blind() {
        for forbidden in ["@drudwyn_message", "capture-pane", "pane_title"] {
            assert!(!FORMAT.contains(forbidden));
        }
    }

    #[test]
    fn enter_activates_the_selected_filtered_result_in_one_action() {
        let mut app = App {
            windows: Vec::new(),
            visible: Vec::new(),
            selected: 0,
            kind: 0,
            details: None,
            new_session: None,
            filter: "star".into(),
            filtering: true,
            pending_kill: None,
            pending_rename: None,
            notice: None,
            theme: Theme::rose_pine(Variant::Moon),
            agent_icon: "A".into(),
            shell_icon: ">_".into(),
            redact: false,
        };

        assert_eq!(handle_key(&mut app, KeyCode::Enter), NavigationAction::Jump);
        assert!(!app.filtering);
    }
    fn populated_app() -> App {
        App {
            windows: parse_windows("dev␟@1␟1␟first␟zsh␟␟␟\ndev␟@2␟2␟second␟zsh␟␟␟"),
            visible: vec![0, 1],
            selected: 0,
            kind: 0,
            details: None,
            new_session: None,
            filter: String::new(),
            filtering: false,
            pending_kill: None,
            pending_rename: None,
            notice: None,
            theme: Theme::rose_pine(Variant::Moon),
            agent_icon: "A".into(),
            shell_icon: ">_".into(),
            redact: false,
        }
    }

    #[test]
    fn kill_requires_explicit_confirmation_and_keeps_the_filtered_target() {
        let mut app = populated_app();
        app.filter = "second".into();
        app.refresh_visible();
        assert_eq!(
            handle_key(&mut app, KeyCode::Char('x')),
            NavigationAction::Continue
        );
        let target = app.pending_kill.as_ref().unwrap().id.clone();
        assert_eq!(target, app.windows[1].id);
        for key in [
            KeyCode::Enter,
            KeyCode::Down,
            KeyCode::Char('x'),
            KeyCode::Char('s'),
        ] {
            assert_eq!(handle_key(&mut app, key), NavigationAction::Continue);
            assert_eq!(app.pending_kill.as_ref().unwrap().id, target);
        }
        assert_eq!(
            handle_key(&mut app, KeyCode::Char('y')),
            NavigationAction::Kill
        );
    }

    #[test]
    fn kill_can_be_cancelled_and_cannot_target_an_empty_result() {
        for key in [KeyCode::Esc, KeyCode::Char('n'), KeyCode::Char('q')] {
            let mut app = populated_app();
            handle_key(&mut app, KeyCode::Char('x'));
            assert_eq!(handle_key(&mut app, key), NavigationAction::Continue);
            assert!(app.pending_kill.is_none());
            assert_eq!(
                handle_key(&mut app, KeyCode::Char('y')),
                NavigationAction::Continue
            );
        }
        let mut app = populated_app();
        app.filter = "no-match".into();
        app.refresh_visible();
        handle_key(&mut app, KeyCode::Char('x'));
        assert!(app.pending_kill.is_none());
    }

    #[test]
    fn kill_keys_are_text_while_filtering() {
        let mut app = populated_app();
        handle_key(&mut app, KeyCode::Char('/'));
        handle_key(&mut app, KeyCode::Char('x'));
        handle_key(&mut app, KeyCode::Char('y'));
        handle_key(&mut app, KeyCode::Char('s'));
        assert_eq!(app.filter, "xys");
        assert!(app.pending_kill.is_none());
    }
}
