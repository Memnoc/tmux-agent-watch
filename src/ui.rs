//! Shared presentation primitives for Drudwyn's interactive terminal surfaces.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::theme::Theme;

#[derive(Clone, Copy)]
pub(crate) enum FooterTone {
    Info,
    Warning,
    Error,
}

/// Bound labels by terminal cells, retaining an explicit truncation marker.
pub(crate) fn ellipsize(value: &str, width: usize) -> String {
    let clean: String = value.chars().filter(|c| !c.is_control()).collect();
    if Line::from(clean.as_str()).width() <= width {
        return clean;
    }
    let mut result = String::new();
    for c in clean.chars() {
        if Line::from(format!("{result}{c}…")).width() > width {
            break;
        }
        result.push(c);
    }
    if width > 0 {
        result.push('…');
    }
    result
}

/// Wrap only between actions so a key never gets separated from its label.
pub(crate) fn action_lines(
    groups: &[&[(&str, &str)]],
    theme: Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::default()];
    for action in groups.iter().flat_map(|group| group.iter()) {
        let item = action_line(&[&[*action]], theme);
        let line = lines.last_mut().unwrap();
        if line.width() + item.width() + 1 > width as usize && !line.spans.is_empty() {
            lines.push(item);
        } else {
            if !line.spans.is_empty() {
                line.spans.push(Span::raw(" "));
            }
            line.spans.extend(item.spans);
        }
    }
    lines
}

pub(crate) fn render_action_bar(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    theme: Theme,
    groups: &[&[(&str, &str)]],
) {
    frame.render_widget(
        Paragraph::new(action_line(groups, theme)).block(Block::default().borders(Borders::TOP)),
        area,
    );
}

pub(crate) fn render_footer(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    theme: Theme,
    groups: &[&[(&str, &str)]],
    context: (&str, &str, FooterTone),
) {
    frame.render_widget(
        Paragraph::new(vec![
            action_line(groups, theme),
            context_line(context, theme),
        ])
        .block(Block::default().borders(Borders::TOP)),
        area,
    );
}

pub(crate) fn action_line(groups: &[&[(&str, &str)]], theme: Theme) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    for (group_index, group) in groups.iter().enumerate() {
        if group_index > 0 {
            spans.push(Span::styled("  │  ", Style::default().fg(theme.subtle())));
        }
        for (action_index, (key, label)) in group.iter().enumerate() {
            if action_index > 0 {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(
                format!("[{key}]"),
                Style::default().fg(theme.rose).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {label}"),
                Style::default().fg(theme.text),
            ));
        }
    }
    Line::from(spans)
}

fn context_line(context: (&str, &str, FooterTone), theme: Theme) -> Line<'static> {
    let (label, message, tone) = context;
    let colour = match tone {
        FooterTone::Info => theme.pine,
        FooterTone::Warning => theme.gold,
        FooterTone::Error => theme.love,
    };
    Line::from(vec![
        Span::styled(
            format!(" {label:<9}"),
            Style::default().fg(colour).add_modifier(Modifier::BOLD),
        ),
        Span::styled(message.to_owned(), Style::default().fg(theme.subtle())),
    ])
}

/// A common compact masthead: the bundled hound, identity, and live summary.
pub(crate) fn masthead(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    theme: Theme,
    title: &str,
    summary: &str,
) {
    frame.render_widget(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme.line())),
        area,
    );
    if area.width < 40 || area.height < 5 {
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(title.to_owned(), Style::default().fg(theme.accent())),
                Line::from(ellipsize(summary, area.width as usize)),
            ]),
            area,
        );
        return;
    }
    let background = if theme.base == Theme::rose_pine(crate::theme::Variant::Dawn).base {
        Theme::rose_pine(crate::theme::Variant::Moon).base
    } else {
        theme.base
    };
    crate::brand::render(frame, Rect::new(area.x + 2, area.y, 8, 4), background);
    let wide = area.width >= 100;
    let title_x = if wide { 32 } else { 12 };
    if wide {
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(
                    "Drudwyn",
                    Style::default()
                        .fg(theme.accent())
                        .add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    if title.starts_with("SESSION") {
                        "SESSIONS"
                    } else {
                        "WORKSPACES"
                    },
                    Style::default().fg(theme.subtle()),
                ),
            ]),
            Rect::new(area.x + 12, area.y + 1, 18, 2),
        );
    }
    let text = Rect::new(
        area.x + title_x,
        area.y,
        area.width.saturating_sub(title_x + 2),
        4,
    );
    let lines = if wide {
        vec![
            Line::styled(
                title.to_owned(),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Line::default(),
            Line::styled(
                ellipsize(summary, text.width as usize),
                Style::default().fg(theme.subtle()),
            ),
        ]
    } else {
        vec![
            Line::styled(
                "Drudwyn",
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Line::styled(
                ellipsize(title, text.width as usize),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Line::default(),
            Line::styled(
                ellipsize(summary, text.width as usize),
                Style::default().fg(theme.subtle()),
            ),
        ]
    };
    frame.render_widget(Paragraph::new(lines), text);
}

pub(crate) fn cell(value: &str, width: usize, style: Style) -> Span<'static> {
    let value = ellipsize(value, width.saturating_sub(2));
    let pad = width.saturating_sub(Line::from(value.as_str()).width());
    Span::styled(format!("{value}{}", " ".repeat(pad)), style)
}

pub(crate) fn activity(state: crate::domain::Lifecycle) -> &'static str {
    use crate::domain::Lifecycle::*;
    match state {
        Working => "WORKING",
        Running => "RUNNING",
        Waiting => "NEEDS INPUT",
        Review => "REVIEW",
        Failed => "FAILED",
        Starting => "STARTING",
        Unknown => "UNCONFIRMED",
    }
}

pub(crate) fn activity_style(state: crate::domain::Lifecycle, theme: Theme) -> Style {
    use crate::domain::Lifecycle::*;
    match state {
        Failed => Style::default().fg(theme.base).bg(theme.love),
        Waiting => Style::default().fg(theme.base).bg(theme.gold),
        Review => Style::default().fg(theme.base).bg(theme.pine),
        Working | Starting => Style::default().fg(theme.accent()).bg(theme.surface()),
        Unknown => Style::default().fg(theme.subtle()),
        _ => Style::default().fg(theme.pine).bg(theme.surface()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_actions_use_keycaps_and_group_separators() {
        let theme = Theme::rose_pine(crate::theme::Variant::Moon);
        let line = action_line(&[&[("j/k", "Move")], &[("Esc", "Close")]], theme);
        let text = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert_eq!(text, " [j/k] Move  │  [Esc] Close");
    }
}

/// Consistent selected-item panel. Expanded details wrap and scroll; sidebar
/// values truncate explicitly and never leave a label without its value.
pub(crate) fn inspection(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    theme: Theme,
    title: &str,
    name: &str,
    fields: &[(&str, String)],
    scroll: Option<u16>,
) -> u16 {
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(theme.line()));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let body = Rect::new(
        inner.x + 2,
        inner.y + 1,
        inner.width.saturating_sub(4),
        inner.height.saturating_sub(3),
    );
    let mut lines = vec![
        Line::styled(title.to_owned(), Style::default().fg(theme.accent())),
        Line::default(),
        Line::styled(
            if scroll.is_some() {
                name.to_owned()
            } else {
                ellipsize(name, body.width as usize)
            },
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Line::default(),
    ];
    for (label, value) in fields {
        if scroll.is_none() && lines.len() + 2 > body.height as usize {
            break;
        }
        lines.push(Line::styled(
            (*label).to_owned(),
            Style::default().fg(theme.subtle()),
        ));
        lines.push(Line::from(if scroll.is_some() {
            value.clone()
        } else {
            ellipsize(value, body.width as usize)
        }));
        if scroll.is_some() || body.height >= 27 {
            lines.push(Line::default());
        }
    }
    let paragraph = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false });
    let max_scroll = paragraph
        .line_count(body.width)
        .saturating_sub(body.height as usize)
        .min(u16::MAX as usize) as u16;
    frame.render_widget(
        paragraph.scroll((scroll.unwrap_or(0).min(max_scroll), 0)),
        body,
    );
    frame.render_widget(
        Paragraph::new(if scroll.is_some() {
            "[j/k] Scroll  [d/Esc] Back"
        } else {
            "[d] Full details"
        })
        .style(Style::default().fg(theme.accent())),
        Rect::new(
            inner.x + 2,
            inner.bottom().saturating_sub(1),
            inner.width.saturating_sub(4),
            1,
        ),
    );
    scroll.unwrap_or(0).min(max_scroll)
}

pub(crate) fn navigation_toolbar(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    theme: Theme,
    summary: &str,
) {
    let action = "[n] New shell session";
    let right = (action.len() as u16 + 2).min(area.width);
    frame.render_widget(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme.line())),
        area,
    );
    frame.render_widget(
        Paragraph::new(ellipsize(
            summary,
            area.width.saturating_sub(right + 2) as usize,
        ))
        .style(Style::default().fg(theme.subtle())),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(right + 2), 1),
    );
    frame.render_widget(
        Paragraph::new(action).style(Style::default().fg(theme.accent())),
        Rect::new(area.right().saturating_sub(right), area.y, right, 1),
    );
}

#[derive(Clone)]
pub(crate) struct ShellForm {
    pub name: String,
    pub directory: String,
    pub editing_directory: bool,
}
pub(crate) enum FormAction {
    Edit,
    Cancel,
    Create,
}
impl ShellForm {
    pub fn from_current() -> std::io::Result<Self> {
        Ok(Self {
            name: String::new(),
            directory: crate::session::directory()?,
            editing_directory: false,
        })
    }
    pub fn edit(&mut self, code: crossterm::event::KeyCode) -> FormAction {
        use crossterm::event::KeyCode;
        match code {
            KeyCode::Esc => return FormAction::Cancel,
            KeyCode::Tab | KeyCode::BackTab => self.editing_directory = !self.editing_directory,
            KeyCode::Enter if self.editing_directory => return FormAction::Create,
            KeyCode::Enter => self.editing_directory = true,
            KeyCode::Backspace => {
                if self.editing_directory {
                    self.directory.pop();
                } else {
                    self.name.pop();
                }
            }
            KeyCode::Char(c) => {
                if self.editing_directory {
                    self.directory.push(c);
                } else {
                    self.name.push(c);
                }
            }
            _ => {}
        }
        FormAction::Edit
    }
}
pub(crate) fn shell_form(
    frame: &mut ratatui::Frame<'_>,
    form: &ShellForm,
    theme: Theme,
    notice: Option<&str>,
) {
    use ratatui::layout::{Constraint, Layout};
    let area = frame.area();
    let actions = &[
        ("Tab", "Field"),
        ("Enter", "Next/Create"),
        ("Backspace", "Delete"),
        ("Esc", "Cancel"),
    ];
    let help = action_lines(&[actions], theme, area.width);
    let groups = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(4),
        Constraint::Length(help.len() as u16 + 2),
    ])
    .split(area);
    masthead(
        frame,
        groups[0],
        theme,
        "NEW SESSION / SHELL",
        "Name your shell and choose its starting directory",
    );
    let field = |label: &str, value: &str, active: bool| {
        Line::styled(
            format!(
                " {} {label}: {value}{}",
                if active { "›" } else { " " },
                if active { "_" } else { "" }
            ),
            Style::default().fg(if active { theme.accent() } else { theme.text }),
        )
    };
    let lines = vec![
        Line::default(),
        field("Name", &form.name, !form.editing_directory),
        Line::default(),
        field("Directory", &form.directory, form.editing_directory),
        Line::default(),
        Line::from(" Opens a shell in this terminal."),
        Line::from(" Other terminals keep their selection."),
    ];
    frame.render_widget(
        Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false }),
        groups[1],
    );
    let mut help = help;
    help.push(Line::styled(
        notice
            .unwrap_or("Enter a name and starting directory")
            .to_owned(),
        Style::default().fg(theme.subtle()),
    ));
    frame.render_widget(
        Paragraph::new(help).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme.line())),
        ),
        groups[2],
    );
}
