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
                Line::styled("WORKSPACES", Style::default().fg(theme.subtle())),
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
