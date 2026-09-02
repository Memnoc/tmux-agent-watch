use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    domain::{GitState, Lifecycle, Workspace},
    theme::Theme,
};

pub struct SidebarFrame {
    pub text: String,
    pub click_map: String,
}

pub fn hud_fleet(workspaces: &[Workspace], session: &str, theme: Theme, _redact: bool) -> String {
    let scoped = workspaces
        .iter()
        .filter(|workspace| workspace.identity.session == session);
    let (mut total, mut working, mut waiting, mut review, mut failed) = (0, 0, 0, 0, 0);
    for workspace in scoped {
        total += 1;
        match workspace.lifecycle {
            Lifecycle::Starting | Lifecycle::Working => working += 1,
            Lifecycle::Waiting => waiting += 1,
            Lifecycle::Review => review += 1,
            Lifecycle::Failed => failed += 1,
            Lifecycle::Unknown => {}
        }
    }
    let attention = waiting + review + failed;
    let (color, label, count) = if failed > 0 {
        (theme.love, "FAILED", failed)
    } else if waiting > 0 {
        (theme.gold, "WAITING", waiting)
    } else if review > 0 {
        (theme.pine, "REVIEW", review)
    } else if working > 0 {
        (theme.rose, "WORKING", working)
    } else {
        (theme.muted, "COCKPIT", total)
    };
    let badge_count = if attention > 0 { attention } else { count };
    format!(
        "#[fg={},bg={},bold] {} {} #[default]",
        hex(theme.base),
        hex(color),
        label,
        badge_count
    )
}

pub fn hud_selected(
    workspaces: &[Workspace],
    window_id: &str,
    theme: Theme,
    redact: bool,
) -> String {
    let Some(workspace) = workspaces
        .iter()
        .find(|workspace| workspace.identity.window_id == window_id)
    else {
        return format!("#[fg={},bold] shell#[default]", hex(theme.text));
    };
    let color = state_color(workspace.lifecycle, theme);
    format!(
        "#[fg={},bold] {}#[default]  #[fg={},bold]{}#[default] #[fg={}]· {}#[default]",
        hex(theme.text),
        if redact {
            "Workspace"
        } else {
            &workspace.identity.window_name
        },
        hex(color),
        workspace.lifecycle.label(),
        hex(theme.muted),
        age(workspace.state_since),
    )
}

pub fn sidebar(
    workspaces: &[Workspace],
    session: &str,
    current_window: &str,
    expanded: bool,
    theme: Theme,
    redact: bool,
) -> SidebarFrame {
    let scoped = workspaces
        .iter()
        .filter(|workspace| workspace.identity.session == session);
    let mut rows = scoped.collect::<Vec<_>>();
    rows.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
    let mut text = String::new();
    let mut click_map = String::new();
    let mut row = 0usize;
    if expanded {
        text.push_str(&format!(
            "\x1b[1m WORKSPACES\x1b[0m  \x1b[38;2;{}m{}\x1b[0m\n\n",
            rgb(theme.muted),
            if redact { "tmux" } else { session }
        ));
        row = 2;
    }
    let mut previous_attention = None;
    for workspace in rows {
        if expanded {
            let attention = workspace.lifecycle.needs_attention();
            if previous_attention != Some(attention) {
                let heading = if attention { "NEEDS YOU" } else { "ACTIVE" };
                text.push_str(&format!(" \x1b[1m{heading}\x1b[0m\n"));
                row += 1;
                previous_attention = Some(attention);
            }
        }
        let selected = workspace.identity.window_id == current_window;
        let marker = if workspace.checkout.is_linked_worktree {
            '◆'
        } else {
            '●'
        };
        let background = if selected { "\x1b[48;2;57;53;82m" } else { "" };
        let color = state_color(workspace.lifecycle, theme);
        if expanded {
            text.push_str(&format!(
                "{background} \x1b[38;2;{}m{marker}\x1b[0m{background} {:<27.27}\x1b[0m\n",
                rgb(color),
                if redact {
                    "Workspace"
                } else {
                    &workspace.identity.window_name
                }
            ));
            map_row(&mut click_map, row, &workspace.identity.window_id);
            row += 1;
            let branch = if redact {
                "private"
            } else {
                workspace.checkout.branch.as_deref().unwrap_or("detached")
            };
            let git = match workspace.checkout.git_state {
                GitState::Clean => "CLEAN",
                GitState::Dirty => "DIRTY",
                GitState::Unknown => "GIT ?",
            };
            text.push_str(&format!(
                "    \x1b[38;2;{}m{} · {} · {}\x1b[0m\n",
                rgb(theme.muted),
                workspace.agent.label(),
                branch,
                git
            ));
            map_row(&mut click_map, row, &workspace.identity.window_id);
            row += 1;
            text.push_str(&format!(
                "    \x1b[38;2;{}m{} · {}\x1b[0m\n\n",
                rgb(color),
                workspace.lifecycle.label(),
                age(workspace.state_since)
            ));
            map_row(&mut click_map, row, &workspace.identity.window_id);
            row += 2;
        } else {
            text.push_str(&format!(
                "{background} \x1b[38;2;{}m{marker}\x1b[0m{background} \x1b[0m\n",
                rgb(color)
            ));
            map_row(&mut click_map, row, &workspace.identity.window_id);
            row += 1;
        }
    }
    SidebarFrame { text, click_map }
}

fn map_row(map: &mut String, row: usize, window_id: &str) {
    map.push_str(&format!("{row}={window_id};"));
}

fn age(since: Option<u64>) -> String {
    let Some(since) = since else {
        return "-".into();
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let elapsed = now.saturating_sub(since);
    if elapsed < 60 {
        "<1m".into()
    } else if elapsed < 3600 {
        format!("{}m", elapsed / 60)
    } else {
        format!("{}h", elapsed / 3600)
    }
}

fn state_color(state: Lifecycle, theme: Theme) -> ratatui::style::Color {
    match state {
        Lifecycle::Waiting => theme.gold,
        Lifecycle::Review => theme.pine,
        Lifecycle::Failed => theme.love,
        Lifecycle::Starting | Lifecycle::Working => theme.rose,
        Lifecycle::Unknown => theme.muted,
    }
}

fn rgb(color: ratatui::style::Color) -> String {
    match color {
        ratatui::style::Color::Rgb(r, g, b) => format!("{r};{g};{b}"),
        _ => "144;140;170".into(),
    }
}

fn hex(color: ratatui::style::Color) -> String {
    match color {
        ratatui::style::Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        _ => "#908caa".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{AgentKind, Checkout, EvidenceSource, GitState, WorkspaceIdentity},
        theme::{Theme, Variant},
    };
    use std::path::PathBuf;

    fn item(state: Lifecycle) -> Workspace {
        Workspace {
            identity: WorkspaceIdentity {
                session: "dev".into(),
                window_id: "@1".into(),
                window_name: "api".into(),
                pane_id: "%1".into(),
            },
            checkout: Checkout {
                working_directory: PathBuf::from("/repo"),
                repository: None,
                worktree: None,
                branch: Some("work/api".into()),
                git_state: GitState::Clean,
                is_linked_worktree: false,
            },
            agent: AgentKind::Codex,
            lifecycle: state,
            evidence: EvidenceSource::Hook,
            state_since: None,
            attention_since: None,
        }
    }

    #[test]
    fn ambient_surfaces_use_fixed_metadata_only() {
        let theme = Theme::rose_pine(Variant::Moon);
        let workspaces = vec![item(Lifecycle::Waiting)];
        assert!(hud_fleet(&workspaces, "dev", theme, false).contains("WAITING 1"));
        let frame = sidebar(&workspaces, "dev", "@1", true, theme, false);
        assert!(frame.text.contains("WAITING"));
        assert!(frame.click_map.contains("@1"));
    }

    #[test]
    fn redacted_surfaces_hide_workspace_labels_but_keep_navigation() {
        let theme = Theme::rose_pine(Variant::Moon);
        let workspaces = vec![item(Lifecycle::Waiting)];
        let selected = hud_selected(&workspaces, "@1", theme, true);
        let frame = sidebar(&workspaces, "dev", "@1", true, theme, true);

        assert!(!selected.contains("api"));
        assert!(!frame.text.contains("api"));
        assert!(!frame.text.contains("work/api"));
        assert!(frame.text.contains("Workspace"));
        assert!(frame.click_map.contains("@1"));
    }
}
