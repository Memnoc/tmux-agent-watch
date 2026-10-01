//! Interactive editor for Drudwyn's tmux options.

use std::{io, path::PathBuf, process::Command, time::Duration};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    theme::{Theme, Variant},
    ui::{self, FooterTone},
};

const SETTINGS_NAVIGATION: &[(&str, &str)] = &[("j/k", "Move"), ("h/l/Tab", "Category")];
const SETTINGS_ACTIONS: &[(&str, &str)] = &[("Enter", "Choose"), ("e", "Edit"), ("r", "Reset")];
const CLOSE_ACTION: &[(&str, &str)] = &[("Esc", "Close")];
const EDIT_ACTIONS: &[(&str, &str)] = &[("Enter", "Apply"), ("Backspace", "Delete")];
const CANCEL_ACTION: &[(&str, &str)] = &[("Esc", "Cancel")];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Category {
    General,
    Appearance,
    Behaviour,
    Shortcuts,
    Lifecycle,
}

impl Category {
    const ALL: [Self; 5] = [
        Self::General,
        Self::Appearance,
        Self::Behaviour,
        Self::Shortcuts,
        Self::Lifecycle,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Behaviour => "Behaviour",
            Self::Shortcuts => "Shortcuts",
            Self::Lifecycle => "Lifecycle",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum SettingKind {
    Choice(&'static [&'static str]),
    Preset(&'static [&'static str]),
    Number { min: u16, max: u16 },
    Text,
    Key,
    Colour,
}

#[derive(Clone, Copy, Debug)]
struct SettingSpec {
    category: Category,
    option: &'static str,
    label: &'static str,
    default: &'static str,
    kind: SettingKind,
}

impl SettingSpec {
    fn description(self) -> &'static str {
        match self.option {
            "@drudwyn-agent" => {
                "Agent preselected when opening a new workspace form; existing chats keep their agent."
            }
            "@drudwyn-base-branch" => {
                "Starting branch for new worktrees and merge check when finishing work. Must exist in the repository."
            }
            "@drudwyn-branch-prefix" => {
                "Prefix added to branches generated from a task in the Cockpit; affects new workspaces only."
            }
            "@drudwyn-redact-labels" => {
                "Hide workspace, repository and branch labels in Drudwyn views. Reopen other popups to update them."
            }
            "@drudwyn-status-layout" => {
                "Focus uses one row for the selected workspace, Git line counts and global attention. Dense fits compact window tabs in one row. Tabs restores the two-row overview."
            }
            "@drudwyn-theme" => {
                "Colour palette for the status bar and popups. Previews here immediately; reopen other popups."
            }
            "@drudwyn-icon-mode" => {
                "Auto detects installed fonts; nerd forces font symbols; safe uses ASCII and overrides custom icons."
            }
            "@drudwyn-agent-icon" => {
                "Auto uses each agent's icon in the bar. Hound, bot or a custom glyph replaces all agent icons."
            }
            "@drudwyn-codex-icon" => {
                "Codex symbol in the status bar. Requires All agent icons = auto and Icon mode other than safe."
            }
            "@drudwyn-claude-icon" => {
                "Claude symbol in the status bar. Requires All agent icons = auto and Icon mode other than safe."
            }
            "@drudwyn-opencode-icon" => {
                "OpenCode symbol in the status bar. Requires All agent icons = auto and Icon mode other than safe."
            }
            "@drudwyn-separator-color" => {
                "Separator colour in the legacy status layout; Rust uses both rows for information. Enter cycles colours; e sets a hex value."
            }
            "@drudwyn-color-window-names" => {
                "Colour agent window numbers by lifecycle state, in the HUD and native tmux window list."
            }
            "@drudwyn-v2" => {
                "On uses Rust supervision. Off uses legacy shell scanning, which reads terminal scrollback."
            }
            "@drudwyn-hud" => {
                "Show the Drudwyn status bar in the selected layout. Off restores the status layout saved when the HUD was enabled."
            }
            "@drudwyn-visible-tabs" => {
                "Maximum local tabs: 1 selected-only, 3, 4 default, 6, or auto. Width may show fewer; +N excludes the cap and opens every local window."
            }
            "@drudwyn-status-key" => {
                "Prefix then this key opens status actions: f failed, i input, r review, a all attention, w every local window; Escape cancels."
            }
            "@drudwyn-status" => {
                "Add lifecycle symbols before windows in the native tmux window list; visible with Lifecycle HUD off."
            }
            "@drudwyn-sidebar" => {
                "Enable a side pane for agent windows. Opens on an active agent window; off removes Drudwyn sidebars."
            }
            "@drudwyn-interval" => {
                "Seconds between agent scans. Restarts the watcher with the new delay; lifecycle hooks remain immediate."
            }
            "@drudwyn-git-interval" => {
                "Seconds between cached Git checks in legacy mode only. Rust views read Git live; zero checks every scan."
            }
            "@drudwyn-sidebar-width" => {
                "Collapsed sidebar width in terminal columns. Resizes existing collapsed sidebars when enabled."
            }
            "@drudwyn-sidebar-expanded-width" => {
                "Expanded sidebar width in terminal columns. Resizes existing expanded sidebars when enabled."
            }
            "@drudwyn-next-key" => {
                "Key after the tmux prefix to jump to the next agent needing attention. Enter edits the key name."
            }
            "@drudwyn-sidebar-key" => {
                "Key after the tmux prefix to expand or collapse the sidebar. Requires Legacy sidebar = on."
            }
            "@drudwyn-restart-key" => {
                "Key after the tmux prefix to recreate the sidebar. Requires Legacy sidebar = on."
            }
            "@drudwyn-worktree-key" => {
                "Key after the tmux prefix to open the new workspace form. The replacement binding is installed immediately."
            }
            "@drudwyn-finish-key" => {
                "Key after the tmux prefix to finish the current linked worktree, after its normal confirmation checks."
            }
            "@drudwyn-cockpit-key" => {
                "Key after the tmux prefix to open the Workspace Cockpit for starting, reviewing and finishing work."
            }
            "@drudwyn-navigator-key" => {
                "Key after the tmux prefix to open the grouped workspace navigator."
            }
            "@drudwyn-native-navigator-key" => {
                "Key after the tmux prefix to open tmux's built-in window tree."
            }
            "@drudwyn-session-key" => {
                "Key after the tmux prefix to open Drudwyn's session navigator."
            }
            "@drudwyn-native-session-key" => {
                "Key after the tmux prefix to open tmux's built-in session tree."
            }
            "@drudwyn-help-key" => {
                "Key after the tmux prefix to open shortcut help. Use tmux names such as H, C-h or F1."
            }
            "@drudwyn-options-key" => {
                "Key after the tmux prefix to reopen this menu. The old binding is removed after the new one is installed."
            }
            "@drudwyn-working-symbol" => {
                "Working marker in the native window list; requires Window status markers on and Lifecycle HUD off."
            }
            "@drudwyn-needs-input-symbol" => {
                "Waiting marker in the native window list; requires Window status markers on and Lifecycle HUD off."
            }
            "@drudwyn-done-symbol" => {
                "Review-ready marker in the native window list; requires Window status markers on and Lifecycle HUD off."
            }
            "@drudwyn-failed-symbol" => {
                "Failed marker in the native window list; requires Window status markers on and Lifecycle HUD off."
            }
            "@drudwyn-working-color" => {
                "Working colour in the status bar and native markers. Default follows the theme; e accepts a hex value."
            }
            "@drudwyn-needs-input-color" => {
                "Waiting badge and native marker colour. Default follows the theme; e accepts a hex value."
            }
            "@drudwyn-done-color" => {
                "Review-ready badge and native marker colour. Default follows the theme; e accepts a hex value."
            }
            "@drudwyn-failed-color" => {
                "Failed badge and native marker colour. Default follows the theme; e accepts a hex value."
            }
            _ => unreachable!("every setting has a description"),
        }
    }
}

const ON_OFF: &[&str] = &["on", "off"];
const SETTINGS: &[SettingSpec] = &[
    SettingSpec {
        category: Category::General,
        option: "@drudwyn-agent",
        label: "Default agent",
        default: "codex",
        kind: SettingKind::Choice(&["codex", "claude", "opencode"]),
    },
    SettingSpec {
        category: Category::General,
        option: "@drudwyn-base-branch",
        label: "Base branch",
        default: "main",
        kind: SettingKind::Text,
    },
    SettingSpec {
        category: Category::General,
        option: "@drudwyn-branch-prefix",
        label: "Branch prefix",
        default: "work/",
        kind: SettingKind::Text,
    },
    SettingSpec {
        category: Category::General,
        option: "@drudwyn-redact-labels",
        label: "Redact labels",
        default: "off",
        kind: SettingKind::Choice(ON_OFF),
    },
    SettingSpec {
        category: Category::Appearance,
        option: "@drudwyn-status-layout",
        label: "Status layout",
        default: "focus",
        kind: SettingKind::Choice(&["focus", "tabs", "dense"]),
    },
    SettingSpec {
        category: Category::Appearance,
        option: "@drudwyn-theme",
        label: "Theme",
        default: "rose-pine",
        kind: SettingKind::Choice(&["rose-pine", "moon", "dawn"]),
    },
    SettingSpec {
        category: Category::Appearance,
        option: "@drudwyn-icon-mode",
        label: "Icon mode",
        default: "auto",
        kind: SettingKind::Choice(&["auto", "nerd", "safe"]),
    },
    SettingSpec {
        category: Category::Appearance,
        option: "@drudwyn-agent-icon",
        label: "All agent icons",
        default: "auto",
        kind: SettingKind::Preset(&["auto", "hound", "bot"]),
    },
    SettingSpec {
        category: Category::Appearance,
        option: "@drudwyn-codex-icon",
        label: "Codex icon",
        default: "✣",
        kind: SettingKind::Text,
    },
    SettingSpec {
        category: Category::Appearance,
        option: "@drudwyn-claude-icon",
        label: "Claude icon",
        default: "✦",
        kind: SettingKind::Text,
    },
    SettingSpec {
        category: Category::Appearance,
        option: "@drudwyn-opencode-icon",
        label: "OpenCode icon",
        default: "⌬",
        kind: SettingKind::Text,
    },
    SettingSpec {
        category: Category::Appearance,
        option: "@drudwyn-separator-color",
        label: "Separator colour",
        default: "default",
        kind: SettingKind::Colour,
    },
    SettingSpec {
        category: Category::Appearance,
        option: "@drudwyn-color-window-names",
        label: "Colour window numbers",
        default: "on",
        kind: SettingKind::Choice(ON_OFF),
    },
    SettingSpec {
        category: Category::Appearance,
        option: "@drudwyn-visible-tabs",
        label: "Visible status tabs",
        default: "4",
        kind: SettingKind::Choice(&["1", "3", "4", "6", "auto"]),
    },
    SettingSpec {
        category: Category::Behaviour,
        option: "@drudwyn-v2",
        label: "Rust implementation",
        default: "on",
        kind: SettingKind::Choice(ON_OFF),
    },
    SettingSpec {
        category: Category::Behaviour,
        option: "@drudwyn-hud",
        label: "Lifecycle HUD",
        default: "on",
        kind: SettingKind::Choice(ON_OFF),
    },
    SettingSpec {
        category: Category::Behaviour,
        option: "@drudwyn-status",
        label: "Window status markers",
        default: "off",
        kind: SettingKind::Choice(ON_OFF),
    },
    SettingSpec {
        category: Category::Behaviour,
        option: "@drudwyn-sidebar",
        label: "Legacy sidebar",
        default: "off",
        kind: SettingKind::Choice(ON_OFF),
    },
    SettingSpec {
        category: Category::Behaviour,
        option: "@drudwyn-interval",
        label: "Scan interval",
        default: "2",
        kind: SettingKind::Number { min: 1, max: 3600 },
    },
    SettingSpec {
        category: Category::Behaviour,
        option: "@drudwyn-git-interval",
        label: "Git interval",
        default: "10",
        kind: SettingKind::Number { min: 0, max: 3600 },
    },
    SettingSpec {
        category: Category::Behaviour,
        option: "@drudwyn-sidebar-width",
        label: "Sidebar width",
        default: "3",
        kind: SettingKind::Number { min: 1, max: 200 },
    },
    SettingSpec {
        category: Category::Behaviour,
        option: "@drudwyn-sidebar-expanded-width",
        label: "Expanded sidebar width",
        default: "38",
        kind: SettingKind::Number { min: 2, max: 300 },
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-next-key",
        label: "Next attention",
        default: "a",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-sidebar-key",
        label: "Toggle sidebar",
        default: "Space",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-restart-key",
        label: "Restart sidebar",
        default: "A",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-worktree-key",
        label: "New workspace",
        default: "W",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-finish-key",
        label: "Finish workspace",
        default: "X",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-cockpit-key",
        label: "Workspace cockpit",
        default: "P",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-navigator-key",
        label: "Workspace navigator",
        default: "w",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-native-navigator-key",
        label: "Native window tree",
        default: "C-w",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-session-key",
        label: "Session navigator",
        default: "s",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-native-session-key",
        label: "Native session tree",
        default: "S",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-help-key",
        label: "Help",
        default: "H",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-options-key",
        label: "Options",
        default: "O",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Shortcuts,
        option: "@drudwyn-status-key",
        label: "Status actions",
        default: "g",
        kind: SettingKind::Key,
    },
    SettingSpec {
        category: Category::Lifecycle,
        option: "@drudwyn-working-symbol",
        label: "Working symbol",
        default: "●",
        kind: SettingKind::Text,
    },
    SettingSpec {
        category: Category::Lifecycle,
        option: "@drudwyn-working-color",
        label: "Working colour",
        default: "default",
        kind: SettingKind::Colour,
    },
    SettingSpec {
        category: Category::Lifecycle,
        option: "@drudwyn-needs-input-symbol",
        label: "Waiting symbol",
        default: "●",
        kind: SettingKind::Text,
    },
    SettingSpec {
        category: Category::Lifecycle,
        option: "@drudwyn-needs-input-color",
        label: "Waiting colour",
        default: "default",
        kind: SettingKind::Colour,
    },
    SettingSpec {
        category: Category::Lifecycle,
        option: "@drudwyn-done-symbol",
        label: "Review symbol",
        default: "●",
        kind: SettingKind::Text,
    },
    SettingSpec {
        category: Category::Lifecycle,
        option: "@drudwyn-done-color",
        label: "Review colour",
        default: "default",
        kind: SettingKind::Colour,
    },
    SettingSpec {
        category: Category::Lifecycle,
        option: "@drudwyn-failed-symbol",
        label: "Failed symbol",
        default: "●",
        kind: SettingKind::Text,
    },
    SettingSpec {
        category: Category::Lifecycle,
        option: "@drudwyn-failed-color",
        label: "Failed colour",
        default: "default",
        kind: SettingKind::Colour,
    },
];

#[derive(Clone, Debug)]
struct Setting {
    spec: SettingSpec,
    value: String,
}

struct App {
    settings: Vec<Setting>,
    category: usize,
    selected: usize,
    editing: Option<String>,
    notice: Option<String>,
    theme: Theme,
}

impl App {
    fn load(variant: Variant) -> io::Result<Self> {
        let settings = SETTINGS
            .iter()
            .copied()
            .map(|spec| {
                let value = tmux_output(&["show-option", "-gqv", spec.option])?;
                Ok(Setting {
                    spec,
                    value: if value.is_empty() {
                        spec.default.into()
                    } else {
                        value
                    },
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        Ok(Self {
            settings,
            category: 0,
            selected: 0,
            editing: None,
            notice: None,
            theme: Theme::rose_pine(variant),
        })
    }

    fn category(&self) -> Category {
        Category::ALL[self.category]
    }

    fn visible(&self) -> Vec<usize> {
        self.settings
            .iter()
            .enumerate()
            .filter_map(|(index, setting)| {
                (setting.spec.category == self.category()).then_some(index)
            })
            .collect()
    }

    fn selected_index(&self) -> Option<usize> {
        self.visible().get(self.selected).copied()
    }

    fn colour_preview(&self, setting: &Setting, value: &str) -> Option<Color> {
        if value == "default" {
            let variant = self
                .settings
                .iter()
                .find(|setting| setting.spec.option == "@drudwyn-theme")?
                .value
                .as_str();
            return Some(match setting.spec.option {
                "@drudwyn-working-color" if variant == "dawn" => Color::Rgb(86, 148, 159),
                "@drudwyn-working-color" => Color::Rgb(156, 207, 216),
                "@drudwyn-needs-input-color" => self.theme.gold,
                "@drudwyn-done-color" => self.theme.pine,
                "@drudwyn-failed-color" => self.theme.love,
                "@drudwyn-separator-color" => match variant {
                    "dawn" => Color::Rgb(223, 218, 217),
                    "rose-pine" => Color::Rgb(64, 61, 82),
                    _ => Color::Rgb(57, 53, 82),
                },
                _ => return None,
            });
        }
        let hex = value.strip_prefix('#').filter(|hex| hex.len() == 6)?;
        let rgb = u32::from_str_radix(hex, 16).ok()?;
        Some(Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8))
    }

    fn change_category(&mut self, delta: isize) {
        self.category = self
            .category
            .saturating_add_signed(delta)
            .min(Category::ALL.len() - 1);
        self.selected = 0;
        self.notice = None;
    }

    fn cycle_category(&mut self) {
        self.category = (self.category + 1) % Category::ALL.len();
        self.selected = 0;
        self.notice = None;
    }

    fn move_selection(&mut self, delta: isize) {
        let last = self.visible().len().saturating_sub(1);
        self.selected = self.selected.saturating_add_signed(delta).min(last);
        self.notice = None;
    }

    fn apply(&mut self, index: usize, value: String) {
        let spec = self.settings[index].spec;
        if let Err(error) = validate(spec.kind, &value) {
            self.notice = Some(error);
            return;
        }
        if (spec.option.ends_with("-symbol") || spec.option.ends_with("-icon"))
            && !(spec.option == "@drudwyn-agent-icon"
                && matches!(value.as_str(), "auto" | "hound" | "bot"))
            && (Span::raw(&value).width() != 1 || value.trim().is_empty())
        {
            self.notice = Some("Choose a single terminal-cell symbol".into());
            return;
        }
        let previous = self.settings[index].value.clone();
        let result = (|| -> io::Result<()> {
            if matches!(spec.kind, SettingKind::Key) {
                validate_key(&value, &previous)?;
            }
            // Resolve the runtime helper before changing anything, including for
            // direct binary invocations outside the plugin's wrapper.
            let helper = runtime_helper()?;
            let previous_option = tmux_output(&["show-option", "-gqv", spec.option])?;
            tmux_status(&["set-option", "-gq", spec.option, &value])?;
            let repaint = || -> io::Result<()> {
                if spec.category == Category::Lifecycle || spec.option == "@drudwyn-theme" {
                    crate::lifecycle::refresh_styles().map_err(io::Error::other)?;
                }
                Ok(())
            };
            let applied = repaint()
                .and_then(|()| command_output(Command::new("bash").arg(&helper).arg(spec.option)));
            if let Err(error) = applied {
                if previous_option.is_empty() {
                    tmux_status(&["set-option", "-gu", spec.option])?;
                } else {
                    tmux_status(&["set-option", "-gq", spec.option, &previous_option])?;
                }
                let _ = command_output(Command::new("bash").arg(&helper).arg(spec.option));
                let _ = repaint();
                return Err(error);
            }
            if matches!(spec.kind, SettingKind::Key) && previous != value {
                // A pre-existing user binding may have prevented the new status
                // shortcut from installing. Rebinding must not remove that key.
                let owned = spec.option != "@drudwyn-status-key"
                    || Command::new("tmux")
                        .args(["list-keys", "-T", "prefix", &previous])
                        .output()
                        .is_ok_and(|o| {
                            String::from_utf8_lossy(&o.stdout)
                                .contains("switch-client -T drudwyn-status")
                        });
                if owned {
                    tmux_status(&["unbind-key", &previous])?;
                }
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.settings[index].value = value.clone();
                if spec.option == "@drudwyn-theme" {
                    self.theme = Theme::rose_pine(match value.as_str() {
                        "dawn" => Variant::Dawn,
                        "rose-pine" => Variant::RosePine,
                        _ => Variant::Moon,
                    });
                }
                self.notice = Some(format!("Applied {} = {}", spec.label, value));
            }
            Err(error) => self.notice = Some(format!("Could not apply setting: {error}")),
        }
    }

    fn cycle_value(&mut self, delta: isize) {
        let Some(index) = self.selected_index() else {
            return;
        };
        let choices = match self.settings[index].spec.kind {
            SettingKind::Choice(choices) | SettingKind::Preset(choices) => choices,
            SettingKind::Colour => &[
                "default", "#9ccfd8", "#f6c177", "#3e8fb0", "#eb6f92", "#c4a7e7",
            ],
            _ => {
                self.editing = Some(self.settings[index].value.clone());
                return;
            }
        };
        let current = choices
            .iter()
            .position(|choice| *choice == self.settings[index].value)
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(choices.len() as isize) as usize;
        self.apply(index, choices[next].into());
    }

    fn begin_edit(&mut self) {
        if let Some(index) = self.selected_index() {
            self.editing = Some(self.settings[index].value.clone());
            self.notice = None;
        }
    }

    fn commit_edit(&mut self) {
        let Some(index) = self.selected_index() else {
            return;
        };
        if let Some(value) = self.editing.clone() {
            self.apply(index, value);
            if self
                .notice
                .as_deref()
                .is_some_and(|notice| notice.starts_with("Applied"))
            {
                self.editing = None;
            }
        }
    }

    fn reset(&mut self) {
        if let Some(index) = self.selected_index() {
            self.apply(index, self.settings[index].spec.default.into());
        }
    }
}

fn validate(kind: SettingKind, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("A setting value cannot be empty".into());
    }
    if value.chars().any(char::is_control) {
        return Err("Setting values must fit on one line".into());
    }
    match kind {
        SettingKind::Choice(choices) if !choices.contains(&value) => {
            return Err(format!("Choose one of: {}", choices.join(", ")));
        }
        SettingKind::Number { min, max } => {
            let number = value
                .parse::<u16>()
                .map_err(|_| format!("Enter a number from {min} to {max}"))?;
            if !(min..=max).contains(&number) {
                return Err(format!("Enter a number from {min} to {max}"));
            }
        }
        SettingKind::Colour
            if value != "default"
                && !(value.len() == 7
                    && value.starts_with('#')
                    && value[1..]
                        .chars()
                        .all(|character| character.is_ascii_hexdigit())) =>
        {
            return Err("Use #RRGGBB or default".into());
        }
        _ => {}
    }
    Ok(())
}

fn validate_key(value: &str, previous: &str) -> io::Result<()> {
    let table = format!("drudwyn-validate-{}", std::process::id());
    let result = tmux_status(&["bind-key", "-T", &table, value, "display-message", ""]);
    let _ = tmux_status(&["unbind-key", "-a", "-T", &table]);
    result?;
    if value != previous {
        let binding = Command::new("tmux")
            .args(["list-keys", "-T", "prefix", value])
            .output()?;
        if binding.status.success() && !binding.stdout.is_empty() {
            return Err(io::Error::other(format!(
                "{value} is already bound; choose an unused key"
            )));
        }
    }
    Ok(())
}

fn runtime_helper() -> io::Result<PathBuf> {
    let directory = std::env::var("DRUDWYN_PLUGIN_DIR")
        .ok()
        .filter(|path| !path.is_empty())
        .unwrap_or(tmux_output(&[
            "show-option",
            "-gqv",
            "@drudwyn_plugin_dir",
        ])?);
    let helper = PathBuf::from(directory).join("scripts/settings-apply.sh");
    if !helper.is_file() {
        return Err(io::Error::other(
            "Reload the Drudwyn plugin to locate its settings helper",
        ));
    }
    Ok(helper)
}

fn command_output(command: &mut Command) -> io::Result<String> {
    let output = command.output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim_end().into())
    } else {
        Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

pub fn run(variant: Variant) -> io::Result<()> {
    let mut app = App::load(variant)?;
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
        if app.editing.is_some() {
            match key.code {
                KeyCode::Esc => app.editing = None,
                KeyCode::Enter => app.commit_edit(),
                KeyCode::Backspace => {
                    app.editing.as_mut().expect("editing value").pop();
                }
                KeyCode::Char(character) => {
                    app.editing.as_mut().expect("editing value").push(character)
                }
                _ => {}
            }
            continue;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
            KeyCode::Tab => app.cycle_category(),
            KeyCode::Left | KeyCode::Char('h') => app.change_category(-1),
            KeyCode::Right | KeyCode::Char('l') => app.change_category(1),
            KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
            KeyCode::Enter | KeyCode::Char(' ') => app.cycle_value(1),
            KeyCode::Char('e') => app.begin_edit(),
            KeyCode::Char('r') => app.reset(),
            _ => {}
        }
    }
}

fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.base).fg(app.theme.text)),
        frame.area(),
    );
    let controls_height = if app.editing.is_some() || app.notice.is_some() {
        3
    } else {
        2
    };
    let description = app
        .selected_index()
        .map(|index| app.settings[index].spec.description())
        .unwrap_or("");
    let width = frame.area().width.saturating_sub(2).max(1) as usize;
    let mut help_lines = Vec::new();
    let mut line = String::from(" ");
    for word in description.split_whitespace() {
        if line.chars().count() + word.chars().count() + 1 > width && line.len() > 1 {
            help_lines.push(Line::raw(line));
            line = String::from(" ");
        }
        line.push_str(word);
        line.push(' ');
    }
    help_lines.push(Line::raw(line));
    let help_height = (help_lines.len() as u16).max(2);
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(controls_height),
            Constraint::Length(help_height),
        ])
        .split(frame.area());

    render_header(frame, app, areas[0]);

    let categories = Category::ALL.iter().map(|category| {
        let selected = *category == app.category();
        Span::styled(
            format!(" {} ", category.label()),
            if selected {
                Style::default()
                    .fg(app.theme.base)
                    .bg(app.theme.rose)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.muted)
            },
        )
    });
    frame.render_widget(
        Paragraph::new(Line::from(categories.collect::<Vec<_>>())),
        areas[1],
    );

    let visible = app.visible();
    let items = visible.iter().map(|index| {
        let setting = &app.settings[*index];
        let choice = matches!(
            setting.spec.kind,
            SettingKind::Choice(_) | SettingKind::Preset(_) | SettingKind::Colour
        );
        let mut spans = vec![Span::styled(
            format!(" {:<24}", setting.spec.label),
            Style::default().fg(app.theme.text),
        )];
        if matches!(setting.spec.kind, SettingKind::Colour) {
            let value = if app.selected_index() == Some(*index) {
                app.editing.as_deref().unwrap_or(&setting.value)
            } else {
                &setting.value
            };
            let colour = app
                .colour_preview(setting, value)
                .or_else(|| app.colour_preview(setting, &setting.value))
                .unwrap_or(app.theme.base);
            spans.push(Span::raw(" "));
            // Background swatches retain their colour under the selection
            // highlight, which overrides the row's foreground.
            spans.push(Span::styled("   ", Style::default().bg(colour)));
            spans.push(Span::styled(
                format!(" {:<14}", setting.value),
                Style::default().fg(app.theme.text),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {:<18}", setting.value),
                Style::default()
                    .fg(app.theme.pine)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        spans.push(Span::styled(
            if choice { "Enter choose" } else { "Enter edit" },
            Style::default().fg(app.theme.muted),
        ));
        ListItem::new(Line::from(spans))
    });
    let mut state = ListState::default().with_selected(Some(app.selected));
    let option = app
        .selected_index()
        .map(|index| app.settings[index].spec.option)
        .unwrap_or("");
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(format!(" {} · {option} ", app.category().label()))
                    .borders(Borders::ALL),
            )
            .highlight_symbol("▶")
            .highlight_style(
                Style::default()
                    .fg(app.theme.rose)
                    .add_modifier(Modifier::BOLD),
            ),
        areas[2],
        &mut state,
    );

    if let Some(value) = &app.editing {
        let message = if let Some(error) = &app.notice {
            format!("{error} · {value}_")
        } else {
            format!("{option} › {value}_")
        };
        ui::render_footer(
            frame,
            areas[3],
            app.theme,
            &[EDIT_ACTIONS, CANCEL_ACTION],
            ("EDIT", &message, FooterTone::Info),
        );
    } else if let Some(notice) = &app.notice {
        let tone = if notice.starts_with("Applied") {
            FooterTone::Info
        } else {
            FooterTone::Error
        };
        ui::render_footer(
            frame,
            areas[3],
            app.theme,
            &[SETTINGS_NAVIGATION, SETTINGS_ACTIONS, CLOSE_ACTION],
            ("STATUS", notice, tone),
        );
    } else {
        ui::render_action_bar(
            frame,
            areas[3],
            app.theme,
            &[SETTINGS_NAVIGATION, SETTINGS_ACTIONS, CLOSE_ACTION],
        );
    }
    frame.render_widget(
        Paragraph::new(help_lines).style(Style::default().fg(app.theme.muted)),
        areas[4],
    );
}

fn render_header(frame: &mut ratatui::Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    frame.render_widget(Block::default().borders(Borders::BOTTOM), area);
    crate::brand::render(
        frame,
        ratatui::layout::Rect::new(area.x + 1, area.y, 16, 8),
        // Match the Cockpit header and preserve contrast in the Dawn theme.
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
            Line::from("OPTIONS"),
            Line::from(Span::styled(
                format!(
                    "{} settings · changes apply to this tmux server",
                    SETTINGS.len()
                ),
                Style::default().fg(app.theme.muted),
            )),
        ]),
        ratatui::layout::Rect::new(area.x + 19, area.y + 2, area.width.saturating_sub(19), 4),
    );
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

fn tmux_status(args: &[&str]) -> io::Result<()> {
    command_output(Command::new("tmux").args(args)).map(|_| ())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn option_catalogue_is_unique_and_covers_every_category() {
        let options = SETTINGS
            .iter()
            .map(|setting| setting.option)
            .collect::<HashSet<_>>();
        assert_eq!(options.len(), SETTINGS.len());
        assert_eq!(SETTINGS.len(), 43);
        for setting in SETTINGS {
            assert!(!setting.description().is_empty());
        }
        for category in Category::ALL {
            assert!(SETTINGS.iter().any(|setting| setting.category == category));
        }
    }

    #[test]
    fn validates_numbers_colours_and_single_line_values() {
        assert!(validate(SettingKind::Number { min: 1, max: 10 }, "2").is_ok());
        assert!(validate(SettingKind::Number { min: 1, max: 10 }, "0").is_err());
        assert!(validate(SettingKind::Colour, "#c4a7e7").is_ok());
        assert!(validate(SettingKind::Colour, "default").is_ok());
        assert!(validate(SettingKind::Colour, "rose").is_err());
        assert!(validate(SettingKind::Text, "two\nlines").is_err());
    }
}
