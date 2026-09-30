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
    layout::{Constraint, Direction, Layout, Rect},
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
}

#[derive(Clone)]
struct App {
    windows: Vec<Window>,
    visible: Vec<usize>,
    selected: usize,
    filter: String,
    filtering: bool,
    pending_kill: Option<Window>,
    pending_rename: Option<(String, String)>,
    notice: Option<String>,
    agent_icon: String,
    theme: Theme,
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
}

impl App {
    fn refresh_visible(&mut self) {
        let query = self.filter.to_lowercase();
        self.visible = self
            .windows
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                query.is_empty()
                    || item.name.to_lowercase().contains(&query)
                    || item.session.to_lowercase().contains(&query)
                    || item
                        .branch
                        .as_deref()
                        .is_some_and(|branch| branch.to_lowercase().contains(&query))
                    || item.agent.label().to_lowercase().contains(&query)
                    || item.lifecycle.label().to_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
        self.selected = self.selected.min(self.visible.len().saturating_sub(1));
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
    let agent_icon = crate::icons::agent_icon();
    let windows = discover()?;
    let selected = windows
        .iter()
        .position(|item| item.id == current)
        .unwrap_or(0);
    let mut app = App {
        visible: (0..windows.len()).collect(),
        windows,
        selected,
        filter: String::new(),
        filtering: false,
        pending_kill: None,
        pending_rename: None,
        notice: None,
        agent_icon,
        theme: Theme::rose_pine(variant),
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

fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    // Redact only the projection: selection, filtering and tmux targets retain
    // their real identities for navigation and explicit user actions.
    let mut projection;
    let app = if app.redact {
        projection = app.clone();
        for item in &mut projection.windows {
            item.name = format!("Workspace {}", item.id);
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
        2
    };
    let groups = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(42),
            Constraint::Min(5),
            Constraint::Length(footer_height),
        ])
        .split(area);
    let agents = app.windows.iter().filter(|item| item.managed).count();
    let attention = app
        .windows
        .iter()
        .filter(|item| item.lifecycle.needs_attention())
        .count();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " WORKSPACE NAVIGATOR ",
                Style::default()
                    .fg(app.theme.rose)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "{} windows · {} agents · {} need you",
                    app.windows.len(),
                    agents,
                    attention
                ),
                Style::default().fg(app.theme.muted),
            ),
        ]))
        .block(Block::default().borders(Borders::BOTTOM)),
        groups[0],
    );

    render_group(frame, app, groups[1], false, " WORKSPACES ");
    render_group(frame, app, groups[2], true, " AGENTS ");
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
        ui::render_action_bar(
            frame,
            groups[3],
            app.theme,
            &[NAVIGATION_ACTIONS, WORKSPACE_ACTIONS, CLOSE_ACTION],
        );
    }
}

fn render_group(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect, agents: bool, title: &str) {
    let indices = app
        .visible
        .iter()
        .copied()
        .filter(|index| app.windows[*index].managed == agents)
        .collect::<Vec<_>>();
    let selected_window = app.visible.get(app.selected).copied();
    let selected =
        selected_window.and_then(|target| indices.iter().position(|index| *index == target));
    let items = indices.iter().map(|index| {
        let item = &app.windows[*index];
        let branch = item.branch.as_deref().unwrap_or("");
        if agents {
            let color = state_color(item.lifecycle, app.theme);
            let agent_icon = app.agent_icon.as_str();
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(
                        " {agent_icon} {} {} · {}",
                        item.id,
                        item.role,
                        truncate(&item.name, 18)
                    ),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!(
                        " {:<8} {} · {} ",
                        item.agent.label(),
                        item.lifecycle.label(),
                        item.evidence
                    ),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} {}", age(item.since), branch),
                    Style::default().fg(app.theme.muted),
                ),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} {} · {}", item.id, item.role, truncate(&item.name, 22)),
                    Style::default().fg(app.theme.text),
                ),
                Span::styled(
                    format!(" {} {}", item.session, branch),
                    Style::default().fg(app.theme.muted),
                ),
            ]))
        }
    });
    let mut state = ListState::default().with_selected(selected);
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::BOTTOM))
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .fg(app.theme.rose)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, area, &mut state);
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
            window.agent = workspace.agent;
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
        let group = if item.managed { 1 } else { 0 };
        let priority = match item.lifecycle {
            Lifecycle::Failed => 0,
            Lifecycle::Waiting => 1,
            Lifecycle::Review => 2,
            _ => 3,
        };
        (
            group,
            priority,
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

fn truncate(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
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

fn state_color(state: Lifecycle, theme: Theme) -> ratatui::style::Color {
    match state {
        Lifecycle::Waiting => theme.gold,
        Lifecycle::Review => theme.pine,
        Lifecycle::Failed => theme.love,
        Lifecycle::Running | Lifecycle::Working | Lifecycle::Starting => theme.rose,
        Lifecycle::Unknown => theme.muted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_and_ids_remain_visible_at_narrow_width_with_redaction() {
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
                app.windows[0].managed = true;
                app.redact = true;
                let mut terminal =
                    Terminal::new(ratatui::backend::TestBackend::new(width, 24)).unwrap();
                terminal.draw(|frame| render(frame, &app)).unwrap();
                let content = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|c| c.symbol())
                    .collect::<String>();
                assert!(
                    content.contains(role),
                    "missing {role} at {width}: {content}"
                );
                assert!(content.contains("@1"));
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
            filter: "star".into(),
            filtering: true,
            pending_kill: None,
            pending_rename: None,
            notice: None,
            agent_icon: "󰀀".into(),
            theme: Theme::rose_pine(Variant::Moon),
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
            filter: String::new(),
            filtering: false,
            pending_kill: None,
            pending_rename: None,
            notice: None,
            agent_icon: "A".into(),
            theme: Theme::rose_pine(Variant::Moon),
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
