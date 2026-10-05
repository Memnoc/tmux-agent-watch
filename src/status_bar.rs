//! Bounded local navigation and global supervision using one metadata snapshot
//! per row, never process or Git probes per tab. Installed rows reuse the observer
//! projection, with explicit freshness; all navigation ranges contain stable IDs.
use crate::{
    discovery,
    domain::{AgentKind, Lifecycle, Workspace},
    inventory,
    navigation::{self, tmux},
    theme::{Theme, Variant},
};
use ratatui::{style::Color, text::Line};
use std::{collections::HashMap, io, process::Command};

#[derive(Clone, Copy, Debug, clap::ValueEnum, PartialEq, Eq)]
pub enum Row {
    Both,
    Tabs,
    Context,
    Focus,
    Dense,
    Balanced,
}
fn cells(s: &str) -> usize {
    Line::from(s).width()
}
fn cut(s: &str, max: usize) -> String {
    let s: String = s.chars().filter(|c| !c.is_control()).collect();
    if cells(&s) <= max {
        return s;
    }
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    for c in s.chars() {
        let next = format!("{out}{c}");
        if cells(&next) + 1 > max {
            break;
        }
        out.push(c);
    }
    out.push('…');
    out
}
fn escape(s: &str) -> String {
    s.replace('#', "##")
}
fn hex(color: Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        _ => "default".into(),
    }
}
fn stable(s: &str, prefix: char) -> bool {
    s.starts_with(prefix) && s.len() > 1 && s[1..].bytes().all(|c| c.is_ascii_digit())
}
struct Style {
    theme: Theme,
    selected: &'static str,
    options: HashMap<String, String>,
    redact: bool,
    nerd: bool,
    fallback_icon: String,
    stale: bool,
}
impl Style {
    fn load() -> io::Result<Self> {
        // Read only appearance options, not arbitrary user options/content.
        let names = [
            "@drudwyn-theme",
            "@drudwyn-redact-labels",
            "@drudwyn-icon-mode",
            "@drudwyn-agent-icon",
            "@drudwyn-codex-icon",
            "@drudwyn-claude-icon",
            "@drudwyn-opencode-icon",
            "@drudwyn-visible-tabs",
            "@drudwyn-working-color",
            "@drudwyn-needs-input-color",
            "@drudwyn-done-color",
            "@drudwyn-failed-color",
            "@drudwyn_scan_at",
            "@drudwyn-interval",
        ];
        let format = names
            .iter()
            .map(|name| format!("#{{{name}}}"))
            .collect::<Vec<_>>()
            .join("␟");
        let raw = tmux(&["display-message", "-p", &format])?;
        let values: Vec<_> = raw.split('␟').collect();
        if values.len() != names.len() {
            return Err(io::Error::other("Invalid status appearance options"));
        }
        let options: HashMap<String, String> = names
            .into_iter()
            .zip(values)
            .filter(|(_, value)| !value.is_empty())
            .map(|(name, value)| (name.into(), value.into()))
            .collect();
        let variant = match options.get("@drudwyn-theme").map(String::as_str) {
            Some("dawn") => Variant::Dawn,
            Some("rose-pine") => Variant::RosePine,
            Some("moon") => Variant::Moon,
            _ => Variant::RosePine,
        };
        let redact = options
            .get("@drudwyn-redact-labels")
            .is_some_and(|v| matches!(v.as_str(), "on" | "true" | "1"));
        let nerd = match options.get("@drudwyn-icon-mode").map(String::as_str) {
            Some("nerd") => true,
            Some("safe") => false,
            None | Some("auto") => crate::icons::policy().0 == "nerd",
            _ => false,
        };
        let fallback_icon = if nerd {
            match options.get("@drudwyn-agent-icon").map(String::as_str) {
                Some("hound") => "󰀀".into(),
                Some("bot") => "󰚩".into(),
                Some("auto") | None => String::new(),
                Some(value) => value.into(),
            }
        } else {
            String::new()
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let interval = options
            .get("@drudwyn-interval")
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| (1..=3600).contains(v))
            .unwrap_or(2);
        let stale = options
            .get("@drudwyn_scan_at")
            .and_then(|v| v.parse::<u64>().ok())
            .is_none_or(|since| since > now || now.saturating_sub(since) > interval * 2 + 5);
        Ok(Self {
            stale,
            nerd,
            fallback_icon,
            theme: Theme::rose_pine(variant),
            selected: match variant {
                Variant::Dawn => "#dfdad9",
                Variant::RosePine => "#403d52",
                _ => "#44415a",
            },
            options,
            redact,
        })
    }
    fn color(&self, state: Lifecycle) -> String {
        let (key, color) = match state {
            Lifecycle::Failed => ("failed", self.theme.love),
            Lifecycle::Waiting => ("needs-input", self.theme.gold),
            Lifecycle::Review => ("done", self.theme.pine),
            Lifecycle::Working => ("working", self.theme.rose),
            _ => ("", self.theme.muted),
        };
        self.options
            .get(&format!("@drudwyn-{key}-color"))
            .filter(|v| {
                v.len() == 7 && v.starts_with('#') && v[1..].bytes().all(|b| b.is_ascii_hexdigit())
            })
            .cloned()
            .unwrap_or_else(|| hex(color))
    }
    fn cap(&self) -> usize {
        match self
            .options
            .get("@drudwyn-visible-tabs")
            .map(String::as_str)
        {
            Some("1") => 1,
            Some("3") => 3,
            Some("6") => 6,
            Some("auto") => usize::MAX,
            _ => 4,
        }
    }
    fn icon(&self, w: &Workspace) -> &str {
        if !self.nerd {
            return match w.agent {
                AgentKind::Codex => "C",
                AgentKind::Claude => "A",
                AgentKind::OpenCode => "O",
                _ => "",
            };
        }
        if w.agent == AgentKind::Unknown {
            return "";
        }
        if !self.fallback_icon.is_empty() {
            return &self.fallback_icon;
        }
        let (option, fallback) = match w.agent {
            AgentKind::Codex => ("@drudwyn-codex-icon", "✣"),
            AgentKind::Claude => ("@drudwyn-claude-icon", "✦"),
            AgentKind::OpenCode => ("@drudwyn-opencode-icon", "⌬"),
            _ => ("", ""),
        };
        self.options
            .get(option)
            .filter(|v| !v.is_empty())
            .map(String::as_str)
            .unwrap_or(fallback)
    }
}
fn role(w: &Workspace) -> &str {
    if w.coordinator.as_deref() == Some(w.identity.window_id.as_str()) {
        "COORD"
    } else if w.checkout.is_linked_worktree {
        "WT"
    } else if w.is_agent() {
        "AGENT"
    } else {
        "SH"
    }
}
fn activity(w: &Workspace) -> &str {
    match w.lifecycle {
        Lifecycle::Working => "WORK",
        Lifecycle::Running => "RUN",
        Lifecycle::Starting => "START",
        Lifecycle::Waiting => "INPUT",
        Lifecycle::Review => "REVIEW",
        Lifecycle::Failed => "FAIL",
        Lifecycle::Unknown if w.is_agent() => "?",
        Lifecycle::Unknown => "",
    }
}
fn exit(w: &Workspace) -> String {
    if let Some(signal) = &w.exit_signal {
        format!("SIG {signal}")
    } else {
        w.exit_code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "?".into())
    }
}
fn state(w: &Workspace) -> String {
    if w.process == "exited" {
        let attention = if w.lifecycle.needs_attention() {
            format!("{} / ", activity(w))
        } else {
            String::new()
        };
        format!("{attention}EXIT {}", exit(w))
    } else {
        activity(w).into()
    }
}
fn context_state(w: &Workspace, compact: bool) -> String {
    if w.process != "exited" {
        if state(w).is_empty() {
            return String::new();
        }
        return if compact {
            state(w)
        } else {
            format!("{} ({})", state(w), w.evidence.label())
        };
    }
    if compact {
        let attention = match w.lifecycle {
            Lifecycle::Review => "REV/",
            Lifecycle::Waiting => "IN/",
            Lifecycle::Failed => "FAIL/",
            _ => "",
        };
        let receipt = if w.exit_signal.is_some() {
            "SIG".into()
        } else {
            exit(w)
        };
        return format!("{attention}X{receipt}");
    }
    if w.lifecycle.needs_attention() {
        format!(
            "{} ({}) / EXIT {}",
            activity(w),
            w.evidence.label(),
            exit(w)
        )
    } else {
        state(w)
    }
}

fn range(target: &str, text: &str) -> String {
    format!("#[range=user|{target}]{text}#[norange]")
}
struct Tab<'a> {
    index: usize,
    workspace: &'a Workspace,
}
fn tab_parts(tab: &Tab, selected: bool, style: &Style, name_width: usize) -> (String, String) {
    let w = tab.workspace;
    let name = if style.redact {
        "Workspace"
    } else {
        &w.identity.window_name
    };
    let icon = style.icon(w);
    let prefix = format!(
        " {} {} {}{}{} ",
        if selected { "●" } else { " " },
        tab.index,
        role(w),
        if icon.is_empty() { "" } else { " " },
        icon
    );
    let title = format!("{prefix}{}  ", cut(name, name_width));
    let badge = if state(w).is_empty() {
        " ".into()
    } else {
        format!("  {}  ", state(w))
    };
    (title, badge)
}
fn tabs(rows: &[Tab], current: &str, session: &str, width: usize, style: &Style) -> String {
    let mut count = rows.len().min(style.cap());
    let choose = |count: usize| {
        let mut chosen: Vec<_> = (0..count).collect();
        if let Some(index) = rows
            .iter()
            .position(|r| r.workspace.identity.window_id == current)
        {
            if !chosen.contains(&index) && count > 0 {
                chosen[count - 1] = index;
                chosen.sort_unstable();
            }
        }
        chosen
    };
    loop {
        let chosen = choose(count);
        let hidden = rows.len() - count;
        let overflow = if hidden > 0 {
            format!("  +{hidden} ")
        } else {
            String::new()
        };
        let size: usize = chosen
            .iter()
            .map(|i| {
                let (a, b) = tab_parts(
                    &rows[*i],
                    rows[*i].workspace.identity.window_id == current,
                    style,
                    24,
                );
                cells(&a) + cells(&b)
            })
            .sum::<usize>()
            + count.saturating_sub(1) * 2
            + cells(&overflow);
        if size <= width || count <= 1 {
            let mut out = String::new();
            for i in chosen {
                if !out.is_empty() {
                    out.push_str("  ");
                }
                let row = &rows[i];
                let selected = row.workspace.identity.window_id == current;
                let (base, badge) = tab_parts(row, selected, style, 0);
                let name_width = if count == 1 {
                    width.saturating_sub(cells(&base) + cells(&badge) + cells(&overflow))
                } else {
                    24
                };
                let (title, badge) = tab_parts(row, selected, style, name_width);
                let bg = if selected { style.selected } else { "default" };
                let title = format!(
                    "#[bg={bg},fg={},{}]{}",
                    hex(style.theme.text),
                    if selected { "bold" } else { "nobold" },
                    escape(&title)
                );
                let badge = if state(row.workspace).is_empty() {
                    badge
                } else {
                    format!(
                        "#[bg={},fg={},bold]{}",
                        style.color(inventory::attention_state(row.workspace)),
                        hex(style.theme.base),
                        escape(&badge)
                    )
                };
                out.push_str(&range(
                    &format!("window:{session}:{}", row.workspace.identity.window_id),
                    &format!("{title}{badge}#[default]"),
                ));
            }
            if hidden > 0 {
                out.push_str(&range(
                    &format!("windows:{session}"),
                    &format!("#[fg={},bold]{overflow}#[default]", hex(style.theme.rose)),
                ));
            }
            return out;
        }
        count -= 1;
    }
}
fn context(workspaces: &[Workspace], current: &str, width: usize, style: &Style) -> String {
    let totals = inventory::Totals::from_workspaces(workspaces);
    let labels = [
        ("failed", "FAIL", totals.failed, Lifecycle::Failed),
        ("input", "INPUT", totals.input, Lifecycle::Waiting),
        ("review", "REVIEW", totals.review, Lifecycle::Review),
    ];
    let global = format!(
        "{}{} {}{} ",
        if width < 64 { "ATTN" } else { "NEED YOU" },
        if style.stale { " STALE" } else { "" },
        totals.attention,
        if totals.categories_overlap() {
            if width >= 80 { " (overlap)" } else { "*" }
        } else {
            ""
        }
    );
    let mut right = range("attention", &global);
    let mut right_width = cells(&global);
    for (index, (target, label, n, state)) in labels.into_iter().enumerate() {
        let badge = format!(" {label} {n} ");
        if index > 0 {
            right.push(' ');
            right_width += 1;
        }
        right_width += cells(&badge);
        right.push_str(&range(
            target,
            &format!(
                "#[bg={},fg={},bold]{badge}#[default]",
                style.color(state),
                hex(style.theme.base)
            ),
        ));
    }
    let available = width.saturating_sub(right_width + 2);
    let left = workspaces
        .iter()
        .find(|w| w.identity.window_id == current)
        .map(|w| {
            let name = if style.redact {
                "Workspace"
            } else {
                &w.identity.window_name
            };
            let reference = if style.redact {
                "[private]".into()
            } else {
                selected_branch(w).unwrap_or_else(|| "ref ?".into())
            };
            let status = context_state(w, false);
            // The selected tab already names an agent. Reserve its context row
            // for the checkout and activity, with a bounded branch label.
            let mut full = if w.is_agent() {
                format!("{} · {}", role(w), cut(&reference, 36))
            } else {
                format!("{} {name} · {reference}", role(w))
            };
            if !status.is_empty() {
                full.push_str(&format!(" · {status}"));
            }
            if cells(&full) <= available {
                full
            } else if available >= 20 {
                if status.is_empty() {
                    cut(&format!("{} {name}", role(w)), available)
                } else if cells(&status) <= available {
                    status
                } else {
                    cut(&context_state(w, true), available)
                }
            } else {
                cut(
                    &if state(w).is_empty() {
                        role(w).into()
                    } else {
                        context_state(w, true)
                    },
                    available,
                )
            }
        })
        .unwrap_or_else(|| cut("Selected unavailable", available));
    format!(
        "#[fg={}]{}{}{}#[default]",
        hex(style.theme.text),
        escape(&left),
        " ".repeat(width.saturating_sub(cells(&left) + right_width)),
        right
    )
}
/// Probe only the selected, bound checkout. Counts are tracked text lines
/// against HEAD, including staged and unstaged edits; never task progress.
fn selected_git(w: &Workspace) -> Option<(String, u64, u64)> {
    let path =
        crate::recovery::selected_checkout(&w.identity.window_id, &w.identity.pane_id).ok()?;
    let branch = crate::workspace::checkout_git(&path, &["branch", "--show-current"])
        .output()
        .ok()?;
    if !branch.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&branch.stdout).trim().to_owned();
    let diff = crate::workspace::checkout_git(
        &path,
        &[
            "diff",
            "--shortstat",
            "--no-ext-diff",
            "--no-textconv",
            "--ignore-submodules=all",
            "HEAD",
            "--",
        ],
    )
    .env("LC_ALL", "C")
    .output()
    .ok()?;
    if !diff.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&diff.stdout);
    let words: Vec<_> = text.split_whitespace().collect();
    let count = |label: &str| -> Option<u64> {
        match words.iter().position(|word| word.starts_with(label)) {
            Some(i) => i.checked_sub(1).and_then(|n| words[n].parse().ok()),
            None => Some(0),
        }
    };
    Some((
        if branch.is_empty() {
            "detached".into()
        } else {
            branch
        },
        count("insertion")?,
        count("deletion")?,
    ))
}

fn focus(workspaces: &[Workspace], current: &str, width: usize, style: &Style) -> String {
    let totals = inventory::Totals::from_workspaces(workspaces);
    let attention = format!(
        " NEED {}{}{} ",
        totals.attention,
        if totals.categories_overlap() { "*" } else { "" },
        if style.stale { " STALE" } else { "" }
    );
    let mut right_width = cells(&attention);
    let mut right = range(
        "attention",
        &format!(
            "#[fg={},bold]{}#[nobold]",
            hex(if totals.attention > 0 {
                style.theme.gold
            } else {
                style.theme.muted
            }),
            attention
        ),
    );
    let selected = workspaces.iter().find(|w| w.identity.window_id == current);
    let git = selected.and_then(selected_git);
    if width >= 40 {
        let (plain, styled) = if let Some((_, added, deleted)) = &git {
            (
                format!(" +{added} -{deleted}  "),
                format!(
                    "#[fg={}] +{added}#[fg={}] -{deleted}  ",
                    hex(style.theme.pine),
                    hex(style.theme.love)
                ),
            )
        } else {
            (
                String::from(" Git ?  "),
                format!("#[fg={}] Git ?  ", hex(style.theme.muted)),
            )
        };
        // Huge change counts must not consume the selected workspace identity.
        if cells(&plain) + right_width + 14 <= width {
            right_width += cells(&plain);
            right = format!("{styled}{right}");
        }
    }
    let available = width.saturating_sub(right_width);
    let left = selected
        .map(|w| {
            let name = if style.redact {
                "Workspace"
            } else {
                &w.identity.window_name
            };
            let status = context_state(w, width < 100);
            let status = if status.is_empty() {
                String::new()
            } else {
                format!(" {status}")
            };
            let identity = format!(" {} {name}", role(w));
            let mut label = cut(&identity, available.saturating_sub(cells(&status) + 1));
            if width >= 100 && !style.redact {
                if let Some((branch, _, _)) = &git {
                    let room = available.saturating_sub(cells(&label) + cells(&status) + 4);
                    if room >= 8 {
                        label.push_str(&format!(" · {}", cut(branch, room)));
                    }
                }
            }
            label.push_str(&status);
            cut(&label, available.saturating_sub(1))
        })
        .unwrap_or_else(|| cut(" Selected unavailable", available));
    format!(
        "#[bg={},fg={}]{}{}{}#[default]",
        hex(style.theme.base),
        hex(style.theme.text),
        escape(&left),
        " ".repeat(width.saturating_sub(cells(&left) + right_width)),
        right
    )
}

/// Three anchored regions: one current workspace, centered Git, global attention.
fn balanced(
    workspaces: &[Workspace],
    current: &str,
    session: &str,
    width: usize,
    style: &Style,
) -> String {
    if width < 40 {
        return focus(workspaces, current, width, style);
    }
    let totals = inventory::Totals::from_workspaces(workspaces);
    let attention = format!(
        "{} NEED{}{}",
        totals.attention,
        if totals.categories_overlap() { "*" } else { "" },
        if style.stale { " STALE" } else { "" }
    );
    let selected = workspaces.iter().find(|w| w.identity.window_id == current);
    let git = selected.and_then(selected_git);
    let changes = git
        .as_ref()
        .map(|(_, added, deleted)| format!("+{added} -{deleted}"))
        .filter(|text| cells(text) <= width / 3)
        .unwrap_or_else(|| "Git ?".into());
    let agents = format!(
        " · {} AGENTS",
        workspaces.iter().filter(|w| w.is_agent()).count()
    );
    let full_right = cells(&attention) + cells(&agents) + 1;
    // Reserve totals when they fit beside the minimum Git context; branch text
    // gets the remaining space, so a long ref cannot prematurely hide agents.
    let show_agents = 2 * (full_right + 2) + cells(&changes) <= width;
    let right_width = cells(&attention) + if show_agents { cells(&agents) } else { 0 } + 1;
    let middle_budget = (width / 3).min(width.saturating_sub(2 * (right_width + 2)));
    let room = middle_budget.saturating_sub(cells(&changes) + 1);
    let middle = if let Some((branch, _, _)) = &git {
        if !style.redact && room >= 2 {
            format!("{} {changes}", cut(branch, room))
        } else {
            cut(&changes, middle_budget)
        }
    } else {
        cut(&changes, middle_budget)
    };
    let middle_start = (width - cells(&middle)) / 2;
    let left_budget = middle_start.saturating_sub(2);
    let left = selected
        .map(|w| {
            let icon = if w.is_agent() {
                let configured = style.icon(w);
                if configured.is_empty() {
                    if style.nerd { "󰚩" } else { "A" }
                } else {
                    configured
                }
            } else if style.nerd {
                ""
            } else {
                ">_"
            };
            let role = match role(w) {
                "WT" => "WT ",
                "COORD" => "COORD ",
                _ => "",
            };
            let name = if style.redact {
                "Workspace"
            } else {
                &w.identity.window_name
            };
            let status = context_state(w, true);
            let suffix = if status.is_empty() {
                String::new()
            } else {
                format!(" {status}")
            };
            let identity = cut(
                &format!(" {icon} {role}{name}"),
                left_budget.saturating_sub(cells(&suffix)),
            );
            cut(&format!("{identity}{suffix}"), left_budget)
        })
        .unwrap_or_else(|| cut(" Selected unavailable", left_budget));
    let bg = hex(style.theme.base);
    let mut out = format!("#[bg={bg},fg={}]", hex(style.theme.accent()));
    out.push_str(&range(&format!("windows:{session}"), &escape(&left)));
    out.push_str(&" ".repeat(middle_start.saturating_sub(cells(&left))));
    out.push_str(&format!(
        "#[fg={}]{}",
        hex(style.theme.foam()),
        escape(&middle)
    ));
    out.push_str(&" ".repeat(width.saturating_sub(middle_start + cells(&middle) + right_width)));
    out.push_str(&range(
        "attention",
        &format!(
            "#[fg={}]{}",
            hex(if totals.attention > 0 {
                style.theme.gold
            } else {
                style.theme.subtle()
            }),
            attention
        ),
    ));
    if show_agents {
        out.push_str(&range(
            "agents",
            &format!("#[fg={}]{}", hex(style.theme.subtle()), agents),
        ));
    }
    out.push_str(" #[default]");
    out
}

fn selected_branch(w: &Workspace) -> Option<String> {
    // Presentation cwd may be escaped or empty after exit. Reuse the stable
    // pane/known-checkout resolver; never let an empty path mean our own cwd.
    let path =
        crate::recovery::selected_checkout(&w.identity.window_id, &w.identity.pane_id).ok()?;
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["branch", "--show-current"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|v| !v.is_empty())
}
fn dense(
    rows: &[Tab],
    workspaces: &[Workspace],
    current: &str,
    session: &str,
    width: usize,
    style: &Style,
) -> String {
    let totals = inventory::Totals::from_workspaces(workspaces);
    let attention = format!(
        " NEED {}{}{} ",
        totals.attention,
        if totals.categories_overlap() { "*" } else { "" },
        if style.stale { " STALE" } else { "" }
    );
    let selected = rows
        .iter()
        .find(|t| t.workspace.identity.window_id == current);
    let git = selected.and_then(|t| selected_git(t.workspace));
    let changes = if width >= 64 {
        git.map(|(_, a, d)| format!(" +{a} -{d} "))
            .unwrap_or_else(|| " Git ? ".into())
    } else {
        String::new()
    };
    let changes = if cells(&changes) + cells(&attention) + 20 <= width {
        changes
    } else {
        String::new()
    };
    let right_width = cells(&changes) + cells(&attention);
    let mut count = rows
        .len()
        .min(style.cap())
        .min(width.saturating_sub(right_width) / 17)
        .max(1)
        .min(rows.len());
    // Reserve the navigator overflow link before allocating equally padded tabs.
    while count > 1
        && width.saturating_sub(right_width + if rows.len() > count { 5 } else { 0 }) / count < 17
    {
        count -= 1;
    }
    let mut chosen: Vec<usize> = (0..count).collect();
    if let Some(index) = rows
        .iter()
        .position(|r| r.workspace.identity.window_id == current)
    {
        if !chosen.contains(&index) && count > 0 {
            chosen[count - 1] = index;
            chosen.sort_unstable();
        }
    }
    let overflow = if rows.len() > count {
        format!(" +{} ", rows.len() - count)
    } else {
        String::new()
    };
    let available = width.saturating_sub(right_width + cells(&overflow));
    if available < count {
        return focus(workspaces, current, width, style);
    }
    let tab_width = if count > 0 {
        available / count
    } else {
        available
    };
    let bg = hex(style.theme.surface());
    let mut out = format!("#[bg={bg},fg={}]", hex(style.theme.text));
    for index in chosen {
        let tab = &rows[index];
        let w = tab.workspace;
        let active = w.identity.window_id == current;
        let icon = if w.is_agent() {
            if style.nerd {
                style.fallback_icon.as_str()
            } else {
                "A"
            }
        } else {
            ">_"
        };
        let icon = if icon.is_empty() { "󰚩" } else { icon };
        let name = if style.redact {
            "Workspace"
        } else {
            &w.identity.window_name
        };
        let role = if w.coordinator.as_deref() == Some(w.identity.window_id.as_str()) {
            "COORD "
        } else if w.checkout.is_linked_worktree {
            "WT "
        } else {
            ""
        };
        let badge = if active {
            context_state(w, true)
        } else {
            String::new()
        };
        let label = cut(
            &format!("{icon} {} {role}{name}", tab.index),
            tab_width.saturating_sub(cells(&badge) + 4),
        );
        let content = format!(
            " {label}{}{} ",
            " ".repeat(tab_width.saturating_sub(cells(&label) + cells(&badge) + 3)),
            badge
        );
        let content = cut(&content, tab_width.saturating_sub(1));
        let tab_bg = if active { style.selected } else { &bg };
        let tab_text = format!(
            "#[bg={tab_bg},fg={},{}]{}{}#[bg={bg},fg={},nobold]│",
            hex(style.theme.text),
            if active { "bold" } else { "nobold" },
            escape(&content),
            " ".repeat(tab_width.saturating_sub(cells(&content) + 1)),
            hex(style.theme.line())
        );
        out.push_str(&range(
            &format!("window:{session}:{}", w.identity.window_id),
            &tab_text,
        ));
    }
    out.push_str(&range(
        &format!("windows:{session}"),
        &format!("#[fg={}]{}", hex(style.theme.accent()), escape(&overflow)),
    ));
    out.push_str(
        &" ".repeat(width.saturating_sub(count * tab_width + cells(&overflow) + right_width)),
    );
    out.push_str(&format!("#[fg={}]{}", hex(style.theme.subtle()), changes));
    out.push_str(&range(
        "attention",
        &format!(
            "#[fg={},bold]{}",
            hex(if totals.attention > 0 {
                style.theme.gold
            } else {
                style.theme.subtle()
            }),
            attention
        ),
    ));
    out.push_str("#[default]");
    out
}

pub fn render(
    session: &str,
    current: &str,
    width: usize,
    row: Row,
    projection: bool,
) -> io::Result<String> {
    if !stable(session, '$') || !stable(current, '@') || !(20..=4096).contains(&width) {
        return Err(io::Error::other(
            "Status rendering requires stable IDs and width 20..4096",
        ));
    }
    let workspaces = if row == Row::Tabs || projection {
        discovery::read_all_tmux()
    } else {
        discovery::discover_all_tmux()
    }
    .map_err(io::Error::other)?;
    let style = Style::load()?;
    let local = tmux(&[
        "list-windows",
        "-t",
        session,
        "-F",
        "#{window_index}␟#{window_id}",
    ])?;
    let mut rows = Vec::new();
    for line in local.lines() {
        if let Some((index, id)) = line.split_once('␟') {
            let w = workspaces
                .iter()
                .find(|w| w.identity.window_id == id)
                .ok_or_else(|| {
                    io::Error::other("Local inventory changed during render; refresh")
                })?;
            rows.push(Tab {
                index: index.parse().map_err(io::Error::other)?,
                workspace: w,
            });
        }
    }
    rows.sort_by_key(|r| r.index);
    if !rows
        .iter()
        .any(|r| r.workspace.identity.window_id == current)
    {
        return Err(io::Error::other(
            "Selected window membership disappeared; refresh",
        ));
    }
    if row == Row::Balanced {
        return Ok(balanced(&workspaces, current, session, width, &style));
    }
    if row == Row::Dense {
        return Ok(dense(&rows, &workspaces, current, session, width, &style));
    }
    if row == Row::Focus {
        return Ok(focus(&workspaces, current, width, &style));
    }
    let mut output = Vec::new();
    if row != Row::Context {
        output.push(tabs(&rows, current, session, width, &style));
    }
    if row != Row::Tabs {
        output.push(context(&workspaces, current, width, &style));
    }
    Ok(output.join("\n"))
}
pub fn action(target: &str) -> io::Result<()> {
    let client = navigation::client()?;
    if let Some(rest) = target.strip_prefix("window:") {
        let (session, window) = rest
            .split_once(':')
            .ok_or_else(|| io::Error::other("Invalid status target"))?;
        return navigation::open_for(&client, Some(window), Some(session));
    }
    let mut args = vec!["cockpit".to_owned()];
    if target == "windows" || target.starts_with("windows:") {
        let session = if let Some(session) = target.strip_prefix("windows:") {
            session.to_owned()
        } else {
            tmux(&["list-clients", "-F", "#{client_name}␟#{session_id}"])?
                .lines()
                .filter_map(|row| row.split_once('␟'))
                .find(|(name, _)| *name == client)
                .map(|(_, session)| session.to_owned())
                .ok_or_else(|| io::Error::other("Requesting client detached"))?
        };
        if !stable(&session, '$') {
            return Err(io::Error::other("Invalid local session"));
        }
        tmux(&["list-windows", "-t", &session, "-F", "#{window_id}"])?;
        args.extend(["--windows".into(), "--local-session".into(), session]);
    } else if matches!(target, "failed" | "input" | "review" | "attention") {
        args.extend(["--state".into(), target.into()]);
    } else if target != "agents" {
        return Err(io::Error::other("Unknown status action"));
    }
    let style = Style::load()?;
    args.extend([
        "--theme".into(),
        style
            .options
            .get("@drudwyn-theme")
            .filter(|v| matches!(v.as_str(), "moon" | "dawn" | "rose-pine"))
            .cloned()
            .unwrap_or_else(|| "rose-pine".into()),
    ]);
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    let command = std::iter::once(std::env::current_exe()?.to_string_lossy().into_owned())
        .chain(args)
        .map(|s| quote(&s))
        .collect::<Vec<_>>()
        .join(" ");
    tmux(&[
        "display-popup",
        "-c",
        &client,
        "-EE",
        "-w",
        "95%",
        "-h",
        "85%",
        "-e",
        &format!("DRUDWYN_CLIENT={client}"),
        &command,
    ])?;
    Ok(())
}
