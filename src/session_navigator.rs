//! Compact, session-only replacement for tmux `choose-tree -s`.

use std::{io, path::Path, process::Command, time::Duration};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    theme::{Theme, Variant},
    ui::{self, FooterTone},
};

const SEP: char = '\u{241f}';
const FORMAT: &str = "#{session_name}␟#{session_windows}␟#{session_attached}␟#{session_id}";
const NAVIGATION_ACTIONS: &[(&str, &str)] = &[("j/k", "Move"), ("Enter", "Switch")];
const SESSION_ACTIONS: &[(&str, &str)] = &[
    ("n", "New Session"),
    ("c", "Coordinator"),
    ("r", "Rename"),
    ("x", "Kill"),
    ("s", "Save"),
    ("/", "Filter"),
];
const NEW_ACTIONS: &[(&str, &str)] = &[
    ("Tab", "Field"),
    ("Enter", "Next/Create"),
    ("Backspace", "Delete"),
    ("Esc", "Cancel"),
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Session {
    id: String,
    name: String,
    windows: usize,
    attached: usize,
}

#[derive(Clone)]
struct App {
    sessions: Vec<Session>,
    visible: Vec<usize>,
    selected: usize,
    current: String,
    filter: String,
    filtering: bool,
    pending_kill: Option<Session>,
    pending_rename: Option<(String, String)>,
    new_session: Option<NewSession>,
    notice: Option<String>,
    theme: Theme,
    redact: bool,
}

#[derive(Clone)]
struct NewSession {
    name: String,
    directory: String,
    editing_directory: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NavigationAction {
    Continue,
    Close,
    Switch,
    Coordinator,
    Kill,
    Save,
    Rename,
    New,
    Create,
}

impl App {
    fn refresh_visible(&mut self) {
        let query = self.filter.to_lowercase();
        self.visible = self
            .sessions
            .iter()
            .enumerate()
            .filter(|(_, session)| session.name.to_lowercase().contains(&query))
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
    if let Some(form) = &mut app.new_session {
        match code {
            KeyCode::Esc => {
                app.new_session = None;
                app.notice = None;
            }
            KeyCode::Tab | KeyCode::BackTab => form.editing_directory = !form.editing_directory,
            KeyCode::Enter if form.editing_directory => return NavigationAction::Create,
            KeyCode::Enter => form.editing_directory = true,
            KeyCode::Backspace => {
                if form.editing_directory {
                    form.directory.pop();
                } else {
                    form.name.pop();
                }
                app.notice = None;
            }
            KeyCode::Char(character) => {
                if form.editing_directory {
                    form.directory.push(character);
                } else {
                    form.name.push(character);
                }
                app.notice = None;
            }
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
                return NavigationAction::Switch;
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
                .and_then(|index| app.sessions.get(*index))
                .map(|item| (item.id.clone(), item.name.clone()));
            NavigationAction::Continue
        }
        KeyCode::Char('c') => NavigationAction::Coordinator,
        KeyCode::Char('n') => NavigationAction::New,
        KeyCode::Char('s') => NavigationAction::Save,
        KeyCode::Char('x') => {
            app.pending_kill = app
                .visible
                .get(app.selected)
                .and_then(|index| app.sessions.get(*index))
                .cloned();
            NavigationAction::Continue
        }
        KeyCode::Enter => NavigationAction::Switch,
        _ => NavigationAction::Continue,
    }
}

pub fn run(variant: Variant) -> io::Result<()> {
    let current = crate::navigation::context("#{session_name}")?;
    let aliases = crate::navigation::view_names()?;
    let current = aliases.get(&current).cloned().unwrap_or(current);
    let sessions = discover()?;
    let selected = sessions
        .iter()
        .position(|session| session.name == current)
        .unwrap_or(0);
    let mut app = App {
        visible: (0..sessions.len()).collect(),
        sessions,
        selected,
        current,
        filter: String::new(),
        filtering: false,
        pending_kill: None,
        pending_rename: None,
        new_session: None,
        notice: None,
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
            NavigationAction::New => match crate::session::directory() {
                Ok(directory) => {
                    app.new_session = Some(NewSession {
                        name: String::new(),
                        directory,
                        editing_directory: false,
                    });
                }
                Err(error) => app.notice = Some(format!("New Session failed: {error}")),
            },
            NavigationAction::Create => {
                if let Some(form) = &app.new_session {
                    match crate::session::create(&form.name, Some(Path::new(&form.directory))) {
                        Ok(_) => return Ok(()),
                        Err(error) => app.notice = Some(format!("Create failed: {error}")),
                    }
                }
            }
            NavigationAction::Switch => {
                if let Some(session) = app
                    .visible
                    .get(app.selected)
                    .and_then(|index| app.sessions.get(*index))
                {
                    match crate::navigation::open(None, Some(&session.id)) {
                        Ok(()) => return Ok(()),
                        Err(error) => app.notice = Some(format!("Switch failed: {error}")),
                    }
                }
            }
            NavigationAction::Coordinator => {
                if let Some(session) = app
                    .visible
                    .get(app.selected)
                    .and_then(|i| app.sessions.get(*i))
                {
                    match crate::coordinator::open_project(&session.id) {
                        Ok(()) => return Ok(()),
                        Err(error) => app.notice = Some(error.to_string()),
                    }
                }
            }
            NavigationAction::Rename => {
                if let Some((id, name)) = app.pending_rename.clone() {
                    match tmux_output(&["rename-session", "-t", &id, "--", &name]) {
                        Ok(_) => {
                            for item in app.sessions.iter_mut().filter(|item| item.id == id) {
                                if app.current == item.name {
                                    app.current = name.clone();
                                }
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
                    match tmux_output(&["kill-session", "-t", &target.id]) {
                        Ok(_) => {
                            app.notice = Some("Killed · press s to save cleanup".into());
                            app.sessions.retain(|item| item.id != target.id);
                            match discover() {
                                Ok(items) => app.sessions = items,
                                Err(error) if !app.sessions.is_empty() => {
                                    app.notice = Some(format!("Refresh failed: {error}"));
                                }
                                Err(_) => {}
                            }
                            app.refresh_visible();
                            if app.sessions.is_empty() {
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
    let mut projection;
    let app = if app.redact {
        projection = app.clone();
        for session in &mut projection.sessions {
            if session.name == projection.current {
                projection.current = format!("Session {}", session.id);
            }
            session.name = format!("Session {}", session.id);
        }
        if let Some(target) = &mut projection.pending_kill {
            target.name = "private".into();
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
    if let Some(form) = &app.new_session {
        let mut help = action_lines(NEW_ACTIONS, app.theme, area.width);
        help.push(Line::styled(
            app.notice
                .as_deref()
                .unwrap_or("Enter a name and starting directory")
                .to_owned(),
            Style::default().fg(app.theme.muted),
        ));
        let groups = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(help.len() as u16 + 1),
        ])
        .split(area);
        frame.render_widget(
            Paragraph::new(" NEW SESSION · Shell")
                .style(
                    Style::default()
                        .fg(app.theme.rose)
                        .add_modifier(Modifier::BOLD),
                )
                .block(Block::default().borders(Borders::BOTTOM)),
            groups[0],
        );
        let field = |label: &str, value: &str, active: bool| {
            Line::styled(
                format!(
                    " {} {label}: {value}{}",
                    if active { "›" } else { " " },
                    if active { "_" } else { "" }
                ),
                Style::default().fg(if active {
                    app.theme.rose
                } else {
                    app.theme.text
                }),
            )
        };
        frame.render_widget(
            Paragraph::new(vec![
                field("Name", &form.name, !form.editing_directory),
                Line::default(),
                field("Directory", &form.directory, form.editing_directory),
                Line::default(),
                Line::from(" Opens a shell in this terminal."),
            ])
            .wrap(Wrap { trim: false }),
            groups[1],
        );
        frame.render_widget(
            Paragraph::new(help).block(Block::default().borders(Borders::TOP)),
            groups[2],
        );
        return;
    }
    let actions: Vec<_> = [NAVIGATION_ACTIONS, SESSION_ACTIONS, CLOSE_ACTION].concat();
    let action_lines = action_lines(&actions, app.theme, area.width);
    let footer_height =
        if app.pending_kill.is_some() || app.pending_rename.is_some() || app.filtering {
            3
        } else {
            action_lines.len() as u16 + 1 + u16::from(app.notice.is_some())
        };
    let groups = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(footer_height),
        ])
        .split(area);
    let attached = app
        .sessions
        .iter()
        .filter(|session| session.attached > 0)
        .count();

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " SESSION NAVIGATOR ",
                Style::default()
                    .fg(app.theme.rose)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} sessions · {} attached", app.sessions.len(), attached),
                Style::default().fg(app.theme.muted),
            ),
        ]))
        .block(Block::default().borders(Borders::BOTTOM)),
        groups[0],
    );

    let items = app.visible.iter().map(|index| {
        let session = &app.sessions[*index];
        let is_current = session.name == app.current;
        let marker = if session.attached > 0 { "●" } else { " " };
        let state = match (session.attached > 0, is_current) {
            (true, true) => "attached · current".to_owned(),
            (true, false) => "attached".to_owned(),
            (false, true) => "current".to_owned(),
            (false, false) => String::new(),
        };
        let window_label = if session.windows == 1 {
            "window"
        } else {
            "windows"
        };
        ListItem::new(Line::from(vec![
            Span::styled(
                format!(" {marker} {:<26}", truncate(&session.name, 26)),
                Style::default().fg(if session.attached > 0 {
                    app.theme.pine
                } else {
                    app.theme.text
                }),
            ),
            Span::styled(
                format!("{:>2} {window_label:<7}  ", session.windows),
                Style::default().fg(app.theme.muted),
            ),
            Span::styled(state, Style::default().fg(app.theme.muted)),
        ]))
    });
    let mut state = ListState::default().with_selected(Some(app.selected));
    let list = List::new(items).highlight_symbol("▶ ").highlight_style(
        Style::default()
            .fg(app.theme.rose)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_stateful_widget(list, groups[1], &mut state);

    if let Some(target) = &app.pending_kill {
        let message = format!(
            "Kill session {} {} ({} windows)?",
            target.id, target.name, target.windows
        );
        ui::render_footer(
            frame,
            groups[2],
            app.theme,
            &[CONFIRM_ACTION, CANCEL_ACTION],
            ("CONFIRM", &message, FooterTone::Warning),
        );
    } else if let Some((_, name)) = &app.pending_rename {
        let message = app
            .notice
            .as_deref()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Rename session › {name}_"));
        ui::render_footer(
            frame,
            groups[2],
            app.theme,
            &[EDIT_ACTIONS, CANCEL_ACTION],
            ("RENAME", &message, FooterTone::Info),
        );
    } else if let Some(notice) = &app.notice {
        let mut lines = action_lines;
        lines.push(Line::styled(
            format!(" STATUS   {notice}"),
            Style::default().fg(app.theme.muted),
        ));
        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::TOP)),
            groups[2],
        );
    } else if app.filtering {
        let message = format!("› {}_", app.filter);
        ui::render_footer(
            frame,
            groups[2],
            app.theme,
            &[FILTER_ACTIONS],
            ("FILTER", &message, FooterTone::Info),
        );
    } else {
        frame.render_widget(
            Paragraph::new(action_lines).block(Block::default().borders(Borders::TOP)),
            groups[2],
        );
    }
}

fn action_lines(actions: &[(&str, &str)], theme: Theme, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut start = 0;
    for end in 1..=actions.len() {
        if end > start + 1
            && ui::action_line(&[&actions[start..end]], theme).width() > usize::from(width)
        {
            lines.push(ui::action_line(&[&actions[start..end - 1]], theme));
            start = end - 1;
        }
    }
    lines.push(ui::action_line(&[&actions[start..]], theme));
    lines
}

fn discover() -> io::Result<Vec<Session>> {
    let output = tmux_output(&["list-sessions", "-F", FORMAT])?;
    let mut sessions = parse_sessions(&output);
    let aliases = crate::navigation::view_names()?;
    let views = sessions.clone();
    sessions.retain(|session| !aliases.contains_key(&session.name));
    for view in views {
        if let Some(anchor) = aliases.get(&view.name) {
            if let Some(session) = sessions.iter_mut().find(|session| &session.name == anchor) {
                session.attached += view.attached;
            }
        }
    }
    sessions.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(sessions)
}

fn parse_sessions(output: &str) -> Vec<Session> {
    output
        .lines()
        .filter_map(|line| {
            let fields = line.split(SEP).collect::<Vec<_>>();
            (fields.len() == 4).then(|| Session {
                id: fields[3].into(),
                name: fields[0].into(),
                windows: fields[1].parse().unwrap_or_default(),
                attached: fields[2].parse().unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_session_counts_and_attachment_state() {
        let sep = SEP;
        let sessions = parse_sessions(&format!(
            "dev{sep}7{sep}1{sep}$1\narchive{sep}2{sep}0{sep}$2\n"
        ));
        assert_eq!(
            sessions,
            vec![
                Session {
                    id: "$1".into(),
                    name: "dev".into(),
                    windows: 7,
                    attached: 1,
                },
                Session {
                    id: "$2".into(),
                    name: "archive".into(),
                    windows: 2,
                    attached: 0,
                },
            ]
        );
    }

    #[test]
    fn filtering_enter_switches_the_selected_session() {
        let mut app = App {
            sessions: Vec::new(),
            visible: Vec::new(),
            selected: 0,
            current: String::new(),
            filter: "dev".into(),
            filtering: true,
            pending_kill: None,
            pending_rename: None,
            new_session: None,
            notice: None,
            theme: Theme::rose_pine(Variant::Moon),
            redact: false,
        };
        assert_eq!(
            handle_key(&mut app, KeyCode::Enter),
            NavigationAction::Switch
        );
        assert!(!app.filtering);
    }

    fn populated_app() -> App {
        App {
            sessions: parse_sessions("first␟1␟0␟$1\nsecond␟1␟0␟$2"),
            visible: vec![0, 1],
            selected: 0,
            filter: String::new(),
            filtering: false,
            pending_kill: None,
            pending_rename: None,
            new_session: None,
            notice: None,
            current: String::new(),
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
        assert_eq!(target, app.sessions[1].id);
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
