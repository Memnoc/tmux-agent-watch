//! Compact, session-only replacement for tmux `choose-tree -s`.

use std::{io, process::Command, time::Duration};

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

use crate::theme::{Theme, Variant};

const SEP: char = '\u{241f}';
const FORMAT: &str = "#{session_name}␟#{session_windows}␟#{session_attached}␟#{session_id}";
const FOOTER_CONTROLS: &str =
    " j/k move  Enter switch  r rename  x kill  s save  / filter  Esc close ";

#[derive(Clone, Debug, Eq, PartialEq)]
struct Session {
    id: String,
    name: String,
    windows: usize,
    attached: usize,
}

struct App {
    sessions: Vec<Session>,
    visible: Vec<usize>,
    selected: usize,
    current: String,
    filter: String,
    filtering: bool,
    pending_kill: Option<Session>,
    pending_rename: Option<(String, String)>,
    notice: Option<String>,
    theme: Theme,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NavigationAction {
    Continue,
    Close,
    Switch,
    Kill,
    Save,
    Rename,
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
    let current = tmux_output(&["display-message", "-p", "#{session_name}"])?;
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
        notice: None,
        theme: Theme::rose_pine(variant),
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
            NavigationAction::Switch => {
                if let Some(session) = app
                    .visible
                    .get(app.selected)
                    .and_then(|index| app.sessions.get(*index))
                {
                    let status = Command::new("tmux")
                        .args(["switch-client", "-t", &session.name])
                        .status()?;
                    if status.success() {
                        return Ok(());
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
    let area = frame.area();
    let groups = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
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

    let footer = if let Some(target) = &app.pending_kill {
        format!(
            " y confirm · Esc/n cancel · Closes session\n Kill session {} {} ({} windows)?",
            target.id, target.name, target.windows
        )
    } else if let Some((_, name)) = &app.pending_rename {
        format!(
            " Rename session › {name}_\n {}",
            app.notice
                .as_deref()
                .unwrap_or("Enter apply · Esc cancel · Backspace delete")
        )
    } else if let Some(notice) = &app.notice {
        notice.clone()
    } else if app.filtering {
        format!(" Filter › {}_", app.filter)
    } else {
        FOOTER_CONTROLS.into()
    };
    frame.render_widget(
        Paragraph::new(footer)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(app.theme.muted))
            .block(Block::default().borders(Borders::TOP)),
        groups[2],
    );
}

fn discover() -> io::Result<Vec<Session>> {
    let output = tmux_output(&["list-sessions", "-F", FORMAT])?;
    let mut sessions = parse_sessions(&output);
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
            notice: None,
            theme: Theme::rose_pine(Variant::Moon),
        };
        assert_eq!(
            handle_key(&mut app, KeyCode::Enter),
            NavigationAction::Switch
        );
        assert!(!app.filtering);
    }

    #[test]
    fn footer_controls_fit_the_session_popup() {
        assert!(FOOTER_CONTROLS.chars().count() <= 72);
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
            notice: None,
            current: String::new(),
            theme: Theme::rose_pine(Variant::Moon),
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
