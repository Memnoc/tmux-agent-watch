use std::{
    collections::HashMap,
    io,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crossterm::{
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind,
        KeyModifiers,
    },
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
    ui,
    workspace::{self, Start},
};

const COCKPIT_NAVIGATION: &[(&str, &str)] = &[("j/k", "Move"), ("Enter", "Open")];
const COCKPIT_ACTIONS: &[(&str, &str)] = &[
    ("o", "Recover"),
    ("n", "New"),
    ("b", "Batch setup"),
    ("c", "Coordinator"),
    ("i", "Integrate"),
    ("P", "Promote"),
    ("C", "Conflict"),
    ("V", "Verify"),
    ("f", "Finish"),
    ("/", "Search"),
    ("p/s", "Project/state"),
    ("g", "Group"),
    ("w", "Windows"),
    ("d", "Details"),
    ("x", "Clear filters"),
    ("r", "Refresh"),
];
const CLOSE_ACTION: &[(&str, &str)] = &[("q/Esc", "Close")];
const FILTER_ACTIONS: &[(&str, &str)] = &[
    ("text", "Filter"),
    ("Backspace", "Delete"),
    ("Enter/Esc", "Done"),
];

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
    launch: LaunchForm,
    start_agent: usize,
    from_current: bool,
    start_point: Option<workspace::StartPoint>,
    start_error: Option<String>,
    active_batch: Option<batch::Batch>,
    batch_form: Option<BatchForm>,
    recovery: Option<RecoveryForm>,
    batches: HashMap<String, String>,
    integration: Option<IntegrationForm>,
    conflict: Option<ConflictForm>,
    verification: Option<VerificationForm>,
    snapshot: Option<crate::inventory::Snapshot>,
    query: crate::inventory::Query,
    current_window: String,
    refreshed: Instant,
    stale: bool,
    refreshing: Option<std::sync::mpsc::Receiver<io::Result<crate::inventory::Snapshot>>>,
    selection_changed: bool,
    details_open: bool,
    help_open: bool,
    detail_scroll: u16,
    agent_icon: String,
    config: Config,
    theme: Theme,
}

#[derive(Default)]
struct LaunchForm {
    name: String,
    branch: String,
    custom_branch: bool,
    field: usize,
    cursors: [usize; 3],
    file: bool,
}

struct RecoveryForm {
    repo: PathBuf,
    checkouts: Vec<crate::recovery::Checkout>,
    selected: usize,
    restart: bool,
    task: String,
    file: bool,
    batch: String,
    field: usize,
    cursors: [usize; 2],
    error: Option<String>,
}

struct VerificationForm {
    fields: [String; 3],
    cursors: [usize; 3],
    field: usize,
    status: String,
}

struct ConflictForm {
    path: PathBuf,
    status: String,
    error: Option<String>,
    scroll: u16,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CheckoutAction {
    Integrate,
    Promote,
    Finish,
}
enum ActionPreview {
    Merge(crate::integration::Preview),
    Removal(crate::cleanup::Preview),
}
impl ActionPreview {
    fn display(&self, redact: bool) -> String {
        match self {
            Self::Merge(p) => p.display(redact),
            Self::Removal(p) => p.display(redact),
        }
    }
}
struct IntegrationForm {
    action: CheckoutAction,
    anchor: PathBuf,
    verification: String,
    window: String,
    pane: String,
    source: PathBuf,
    fields: [String; 2],
    cursors: [usize; 2],
    field: usize,
    preview: Option<ActionPreview>,
    result: Option<String>,
    error: Option<String>,
    scroll: u16,
}

struct BatchForm {
    fields: [String; 4],
    selected: usize,
    reuse: bool,
    preview: Option<batch::Preview>,
    error: Option<String>,
}

impl App {
    pub fn new(workspaces: Vec<Workspace>, variant: Variant, config: Config) -> Self {
        let visible = (0..workspaces.len()).collect();
        let batches = HashMap::new();
        Self {
            batches,
            active_batch: None,
            batch_form: None,
            recovery: None,
            workspaces,
            visible,
            selected: 0,
            filter: String::new(),
            filtering: false,
            error: None,
            task: None,
            launch: LaunchForm::default(),
            from_current: false,
            start_point: None,
            start_error: None,
            start_agent: agents()
                .iter()
                .position(|agent| *agent == config.default_agent)
                .unwrap_or(0),
            integration: None,
            conflict: None,
            verification: None,
            snapshot: None,
            query: crate::inventory::Query::default(),
            current_window: String::new(),
            refreshed: Instant::now(),
            stale: false,
            refreshing: None,
            selection_changed: false,
            details_open: false,
            help_open: false,
            detail_scroll: 0,
            agent_icon: "A".into(),
            config,
            theme: Theme::rose_pine(variant),
        }
    }

    fn begin_verification(&mut self) {
        let destination = self
            .selected_workspace()
            .and_then(|w| batch::for_window(&w.identity.window_id).ok().flatten())
            .and_then(|b| crate::integration::batch_destination(&b).ok())
            .map(|c| c.path);
        // Without a validated batch, require an explicit checkout; a selected
        // worker's branch is never silently treated as the assembled target.
        let path = destination
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        self.verification = Some(VerificationForm {
            cursors: [path.len(), 0, 0],
            fields: [path, String::new(), String::new()],
            field: 0,
            status:
                "Select checkout, check identity and command. F5 runs visibly; no automatic checks."
                    .into(),
        });
    }
    fn verification_edit(&mut self, key: KeyCode, paste: Option<&str>) {
        if let Some(form) = &mut self.verification {
            edit_text(
                &mut form.fields[form.field],
                &mut form.cursors[form.field],
                key,
                paste,
                form.field == 2,
            );
        }
    }

    fn begin_conflict(&mut self, path: Option<PathBuf>) {
        let path = path.map(Ok).unwrap_or_else(|| {
            let w = self.selected_workspace().ok_or_else(|| {
                workspace::Error::Invalid("Select a destination or associated worker".into())
            })?;
            if let Some(batch) = batch::for_window(&w.identity.window_id)? {
                Ok(batch.checkout)
            } else {
                crate::recovery::selected_checkout(&w.identity.window_id, &w.identity.pane_id)
            }
        });
        match path {
            Ok(path) => {
                let result = crate::integration::conflict(
                    &path,
                    crate::integration::ConflictAction::Inspect,
                    self.config.redact_labels,
                );
                self.conflict = Some(ConflictForm {
                    path,
                    status: result.as_ref().ok().cloned().unwrap_or_default(),
                    error: result.err().map(|e| e.to_string()),
                    scroll: 0,
                });
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
    fn conflict_key(&mut self, key: KeyCode) -> bool {
        use crate::integration::ConflictAction;
        if key == KeyCode::Esc {
            self.conflict = None;
            self.integration = None;
            self.begin_refresh();
            return false;
        }
        let Some(form) = &mut self.conflict else {
            return false;
        };
        let action = match key {
            KeyCode::Char('c') => ConflictAction::Continue,
            KeyCode::Char('a') => ConflictAction::Abort,
            KeyCode::Char('t') => ConflictAction::Retry,
            KeyCode::Char('o') => ConflictAction::Open,
            KeyCode::Char('r') => ConflictAction::Inspect,
            KeyCode::Char('p') => {
                let result = crate::navigation::context("#{session_id}")
                    .map_err(workspace::Error::from)
                    .and_then(|project| {
                        crate::integration::select_conflict_project(&form.path, &project)
                    });
                if let Err(e) = result {
                    form.error = Some(e.to_string());
                    return false;
                }
                ConflictAction::Inspect
            }
            KeyCode::F(4) => {
                self.next_start_agent();
                return false;
            }
            KeyCode::Char('v') => {
                let path = form.path.clone();
                let agent = self.start_agent();
                match crate::integration::recover_conflict_agent(&path, agent) {
                    Ok(result) => {
                        let form = self.conflict.as_mut().unwrap();
                        form.status = format!("{result}\n{}", form.status);
                        form.error = None;
                    }
                    Err(e) => {
                        self.conflict.as_mut().unwrap().error = Some(e.to_string());
                    }
                }
                return false;
            }
            KeyCode::PageDown | KeyCode::Down => {
                form.scroll = form.scroll.saturating_add(5);
                return false;
            }
            KeyCode::PageUp | KeyCode::Up => {
                form.scroll = form.scroll.saturating_sub(5);
                return false;
            }
            KeyCode::Home => {
                form.scroll = 0;
                return false;
            }
            _ => return false,
        };
        match crate::integration::conflict(&form.path, action, self.config.redact_labels) {
            Ok(status) => {
                form.status = status;
                form.error = None;
                action == ConflictAction::Open
            }
            Err(e) => {
                form.error = Some(e.to_string());
                form.scroll = 0;
                false
            }
        }
    }

    fn begin_integration(&mut self, action: CheckoutAction) {
        let Some(w) = self.selected_workspace() else {
            self.error = Some("Select an available worker before integration".into());
            return;
        };
        let window = w.identity.window_id.clone();
        let pane = w.identity.pane_id.clone();
        let source = match crate::recovery::selected_checkout(&window, &pane) {
            Ok(path) => path,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        let selected = batch::for_window(&window);
        let mut id = selected
            .as_ref()
            .ok()
            .and_then(|b| b.as_ref())
            .map(|b| b.id.clone())
            .unwrap_or_default();
        let anchor = source.clone();
        let mut source = source;
        let mut branch = String::new();
        if action == CheckoutAction::Promote {
            // A worker's own live batch supplies the assembled source. No
            // unrelated batch is searched when metadata is absent or stale.
            if let Ok(Some(batch)) = &selected {
                match crate::integration::batch_destination(batch) {
                    Ok(target) => source = target.path,
                    Err(e) => {
                        self.error = Some(e.to_string());
                        return;
                    }
                }
            } else if selected.is_err() {
                self.error = Some(
                    "Selected batch unavailable; explicitly reselect the assembled checkout".into(),
                );
                return;
            }
            id.clear();
            branch = self.config.base_branch.clone();
        }
        self.integration = Some(IntegrationForm {
            action,
            anchor,
            verification: String::new(),
            window,
            pane,
            source,
            fields: [branch.clone(), id.clone()],
            cursors: [branch.len(), id.len()],
            field: 0,
            preview: None,
            result: None,
            error: selected.err().map(|e| e.to_string()),
            scroll: 0,
        });
        if !id.is_empty() || !branch.is_empty() {
            self.integration_preview();
        }
    }
    fn integration_preview(&mut self) {
        let Some(form) = &mut self.integration else {
            return;
        };
        form.error = None;
        form.result = None;
        form.preview = None;
        form.scroll = 0;
        let destination = match (form.fields[0].is_empty(), form.fields[1].is_empty()) {
            (false, true) => crate::integration::Destination::Branch(form.fields[0].clone()),
            (true, false) => crate::integration::Destination::Batch(form.fields[1].clone()),
            _ => {
                form.error = Some("Choose exactly one explicit destination branch or live batch ID; no base is inferred".into());
                return;
            }
        };
        let request = crate::integration::Request {
            source: form.source.clone(),
            destination,
        };
        let preview = if form.action == CheckoutAction::Finish {
            crate::cleanup::preview(request).map(ActionPreview::Removal)
        } else {
            crate::integration::preview(request).map(ActionPreview::Merge)
        };
        match preview {
            Ok(preview) => {
                if let ActionPreview::Merge(p) = &preview {
                    form.verification =
                        crate::verification::inspect(&p.target.path, self.config.redact_labels)
                            .unwrap_or_else(|_| {
                                "Not verified · destination evidence unavailable".into()
                            });
                }
                form.preview = Some(preview);
            }
            Err(error) => form.error = Some(error.to_string()),
        }
    }

    fn integration_edit(&mut self, key: KeyCode, paste: Option<&str>) {
        if let Some(form) = &mut self.integration {
            if form.preview.is_none() {
                edit_text(
                    &mut form.fields[form.field],
                    &mut form.cursors[form.field],
                    key,
                    paste,
                    false,
                );
            }
        }
    }
    fn integration_key(&mut self, key: KeyCode) {
        if key == KeyCode::Esc {
            self.integration = None;
            self.begin_refresh();
            return;
        }
        let Some(form) = &mut self.integration else {
            return;
        };
        if form.preview.is_some() {
            match key {
                KeyCode::Char('y') if form.result.is_none() && form.error.is_none() => {
                    let current = crate::recovery::selected_checkout(&form.window, &form.pane);
                    if !current.is_ok_and(|path| path == form.anchor) {
                        let action = match form.action {
                            CheckoutAction::Integrate => "Integrate",
                            CheckoutAction::Promote => "Promote",
                            CheckoutAction::Finish => "Finish",
                        };
                        form.error = Some(format!(
                            "Pending {action} target changed or disappeared; cancel and inspect again"
                        ));
                        return;
                    }
                    let result = match form.preview.as_ref().unwrap() {
                        ActionPreview::Merge(preview) => {
                            crate::integration::apply(preview).map(str::to_owned)
                        }
                        ActionPreview::Removal(preview) => {
                            crate::cleanup::apply(preview).map(|_| {
                                "Removed worktree; branch retained. No push or deployment.".into()
                            })
                        }
                    };
                    match result {
                        Ok(result) => {
                            form.result = Some(result);
                            if let Some(ActionPreview::Merge(p)) = &form.preview {
                                form.verification = crate::verification::inspect(
                                    &p.target.path,
                                    self.config.redact_labels,
                                )
                                .unwrap_or_else(|_| {
                                    "Not verified · destination evidence unavailable".into()
                                });
                            }
                        }
                        Err(error) => {
                            form.error = Some(error.to_string());
                            if let Some(ActionPreview::Merge(p)) = &form.preview {
                                if crate::integration::active_conflict_for(p) {
                                    let path = p.target.path.clone();
                                    self.begin_conflict(Some(path));
                                    return;
                                }
                            }
                        }
                    }
                    form.scroll = 0;
                }
                KeyCode::Char('r') => self.integration_preview(),
                KeyCode::Char('e') => {
                    form.preview = None;
                    form.result = None;
                    form.error = None;
                    form.scroll = 0;
                }
                KeyCode::PageDown | KeyCode::Down | KeyCode::Char('j') => {
                    form.scroll = form.scroll.saturating_add(5)
                }
                KeyCode::PageUp | KeyCode::Up | KeyCode::Char('k') => {
                    form.scroll = form.scroll.saturating_sub(5)
                }
                KeyCode::Home => form.scroll = 0,
                _ => {}
            }
        } else {
            match key {
                KeyCode::Enter => self.integration_preview(),
                KeyCode::Tab | KeyCode::BackTab => form.field = 1 - form.field,
                _ => self.integration_edit(key, None),
            }
        }
    }

    fn begin_recovery(&mut self) {
        let repo = match self.selected_workspace() {
            Some(w) => {
                crate::recovery::selected_checkout(&w.identity.window_id, &w.identity.pane_id)
            }
            None => std::env::current_dir().map_err(workspace::Error::from),
        };
        let repo = match repo {
            Ok(repo) => repo,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        self.begin_recovery_at(repo);
    }

    fn begin_recovery_at(&mut self, repo: PathBuf) {
        match crate::recovery::list(&repo) {
            Ok(checkouts) => {
                self.recovery = Some(RecoveryForm {
                    selected: checkouts.iter().position(|c| c.path == repo).unwrap_or(0),
                    repo,
                    checkouts,
                    restart: false,
                    task: String::new(),
                    file: false,
                    batch: String::new(),
                    field: 0,
                    cursors: [0; 2],
                    error: None,
                })
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn recovery_edit(&mut self, key: KeyCode, paste: Option<&str>) {
        if let Some(form) = &mut self.recovery {
            if form.restart {
                let text = if form.field == 0 {
                    &mut form.task
                } else {
                    &mut form.batch
                };
                edit_text(
                    text,
                    &mut form.cursors[form.field],
                    key,
                    paste,
                    form.field == 0 && !form.file,
                );
            }
        }
    }

    fn recovery_key(&mut self, key: KeyCode) -> bool {
        let agent = self.start_agent();
        let Some(form) = &mut self.recovery else {
            return false;
        };
        if key == KeyCode::Esc {
            if form.restart {
                form.restart = false;
                form.task.clear();
                form.error = None;
            } else {
                self.recovery = None;
            }
            return false;
        }
        let mut action = None;
        if form.restart {
            match key {
                KeyCode::Tab | KeyCode::BackTab => form.field = 1 - form.field,
                KeyCode::F(4) => {
                    self.next_start_agent();
                    return false;
                }
                KeyCode::F(5) => {
                    form.file = !form.file;
                    form.task.clear();
                    form.cursors[0] = 0;
                }
                KeyCode::F(6) => action = Some((Some(agent), false)),
                _ => {
                    self.recovery_edit(key, None);
                    return false;
                }
            }
        } else {
            match key {
                KeyCode::Down | KeyCode::Char('j') => {
                    form.selected = (form.selected + 1).min(form.checkouts.len().saturating_sub(1))
                }
                KeyCode::Up | KeyCode::Char('k') => form.selected = form.selected.saturating_sub(1),
                KeyCode::Char('r') => match crate::recovery::list(&form.repo) {
                    Ok(rows) => {
                        let path = form.checkouts.get(form.selected).map(|c| c.path.clone());
                        form.selected = rows
                            .iter()
                            .position(|c| Some(&c.path) == path.as_ref())
                            .unwrap_or(0);
                        form.checkouts = rows;
                    }
                    Err(e) => form.error = Some(e.to_string()),
                },
                KeyCode::Char('s') => action = Some((None, false)),
                KeyCode::Char('c') => action = Some((None, true)),
                KeyCode::Char('t') => {
                    form.restart = true;
                    form.task = form
                        .checkouts
                        .get(form.selected)
                        .and_then(|c| c.task_file.clone())
                        .unwrap_or_default();
                    form.file = !form.task.is_empty();
                    form.cursors = [form.task.len(), 0];
                    form.field = 0;
                    form.batch.clear();
                    form.error = None;
                }
                KeyCode::Enter => {
                    if let Some(row) = form.checkouts.get(form.selected) {
                        match row.live.as_slice() {
                            [window] => match crate::navigation::open(Some(window), None) { Ok(()) => return true, Err(e) => form.error = Some(e.to_string()) },
                            _ => form.error = Some("Select Open shell or Restart for a survivor; multiple live windows require the navigator".into()),
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some((agent, coordinator)) = action {
            if let Some(checkout) = form.checkouts.get(form.selected) {
                let task = if agent.is_none() {
                    crate::recovery::Task::None
                } else if form.file {
                    crate::recovery::Task::File(form.task.clone())
                } else {
                    crate::recovery::Task::Text(form.task.clone())
                };
                let request = crate::recovery::Request {
                    repo: form.repo.clone(),
                    path: checkout.path.clone(),
                    agent,
                    task,
                    batch: (!form.batch.is_empty()).then(|| form.batch.clone()),
                    coordinator,
                };
                match crate::recovery::recover(request) {
                    Ok(_) => return true,
                    Err(error) => form.error = Some(error.to_string()),
                }
            }
        }
        false
    }

    fn begin_start(&mut self) {
        self.task = Some(String::new());
        self.launch = LaunchForm::default();
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

    fn launch_edit(&mut self, key: KeyCode, paste: Option<&str>) {
        let field = self.launch.field;
        let text = match field {
            0 => &mut self.launch.name,
            1 => &mut self.launch.branch,
            _ => self.task.as_mut().unwrap(),
        };
        let multiline = field == 2 && !self.launch.file;
        edit_text(text, &mut self.launch.cursors[field], key, paste, multiline);
        if field == 1 {
            self.launch.custom_branch = true;
        }
        if field == 0 && !self.launch.custom_branch {
            self.launch.branch =
                format!("{}{}", self.config.branch_prefix, slug(&self.launch.name));
            self.launch.cursors[1] = self.launch.branch.len();
        }
    }

    fn launch_worker(&mut self) -> io::Result<()> {
        self.error = None;
        if self.launch.name.trim().is_empty()
            || self.task.as_ref().is_none_or(|t| t.trim().is_empty())
        {
            self.error = Some("Short name and task/reference are required".into());
            return Ok(());
        }
        let Some(point) = self.start_point.clone() else {
            return Ok(());
        };
        match workspace::start(Start {
            name: Some(self.launch.name.clone()),
            task_file: self
                .launch
                .file
                .then(|| self.task.clone().unwrap_or_default()),
            batch: self.active_batch.as_ref().map(|b| b.id.clone()),
            repo: std::env::current_dir()?,
            branch: self.launch.branch.clone(),
            start_point: point.commit,
            root: None,
            command: vec![self.start_agent().command().into()],
        }) {
            Ok(created) => {
                let task = self.task.take().unwrap_or_default();
                let sent = workspace::send_started(
                    &created,
                    &task,
                    self.launch.file,
                    Some(self.start_agent()),
                );
                self.error = Some(match sent {
                    Ok(()) => format!(
                        "Worker created; task sent; acceptance and implementation unknown. Open window {} to inspect.",
                        created.window_id
                    ),
                    Err(error) => format!(
                        "{error}; retained window {} pane {} and checkout {}. Open and inspect before deliberate delivery retry.",
                        created.window_id,
                        created.pane_id,
                        created.path.display()
                    ),
                });
                self.refresh();
            }
            Err(error) => {
                let message = error.to_string();
                if message.starts_with("launch failed:") {
                    self.task = None;
                }
                self.error = Some(message);
            }
        }
        Ok(())
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
                        self.refresh();
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

    fn project_display_label(&self, key: &str, width: usize) -> String {
        if self.config.redact_labels {
            return ui::ellipsize("Project", width);
        }
        let label = self
            .snapshot
            .as_ref()
            .and_then(|s| s.projects.get(key))
            .map(String::as_str)
            .unwrap_or(key);
        if !key.starts_with("repo:") {
            return ui::ellipsize(label, width);
        }
        let path = std::path::Path::new(label);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or(label);
        let duplicates: Vec<_> = self
            .snapshot
            .as_ref()
            .into_iter()
            .flat_map(|s| s.projects.iter())
            .filter(|(id, other)| {
                id.starts_with("repo:")
                    && std::path::Path::new(other).file_name() == path.file_name()
            })
            .collect();
        if duplicates.len() > 1 {
            let parent = path.parent().and_then(|p| p.file_name());
            let candidate = format!("{}/{}", parent.and_then(|p| p.to_str()).unwrap_or(""), name);
            let unique_parent = duplicates
                .iter()
                .filter(|(_, other)| {
                    std::path::Path::new(other)
                        .parent()
                        .and_then(|p| p.file_name())
                        == parent
                })
                .count()
                == 1;
            if unique_parent && Line::from(candidate.as_str()).width() <= width {
                return candidate;
            }
            // Reserve the disambiguator before shortening the name. Full paths
            // remain in details; stable action targets never use this ordinal.
            let ordinal = duplicates
                .iter()
                .position(|(id, _)| id.as_str() == key)
                .unwrap_or(0)
                + 1;
            let suffix = format!(" [{ordinal}]");
            return ui::ellipsize(
                &format!(
                    "{}{}",
                    ui::ellipsize(name, width.saturating_sub(suffix.len())),
                    suffix
                ),
                width,
            );
        }
        ui::ellipsize(name, width)
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
        self.selection_changed = false;
        self.detail_scroll = 0;
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.visible.len() - 1);
    }

    fn refresh_current(&mut self) {
        self.current_window.clear();
        if let Ok(client) = crate::navigation::client() {
            if let Ok(rows) =
                crate::navigation::tmux(&["list-clients", "-F", "#{client_name}␟#{window_id}"])
            {
                self.current_window = rows
                    .lines()
                    .filter_map(|r| r.split_once('␟'))
                    .find(|(name, _)| *name == client)
                    .map(|(_, id)| id.to_owned())
                    .unwrap_or_default();
            }
        }
    }

    fn refresh(&mut self) {
        self.apply_refresh(crate::inventory::Snapshot::capture());
    }

    fn begin_refresh(&mut self) {
        if self.refreshing.is_some() {
            return;
        }
        let (send, receive) = std::sync::mpsc::channel();
        self.refreshing = Some(receive);
        std::thread::spawn(move || {
            let _ = send.send(crate::inventory::Snapshot::capture());
        });
    }

    fn complete_refresh(&mut self) {
        let result = self
            .refreshing
            .as_ref()
            .and_then(|receive| match receive.try_recv() {
                Ok(result) => Some(result),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(_) => Some(Err(io::Error::other("Refresh interrupted; retry"))),
            });
        if let Some(result) = result {
            self.refreshing = None;
            self.apply_refresh(result);
        }
    }

    fn apply_refresh(&mut self, result: io::Result<crate::inventory::Snapshot>) {
        let selected = self
            .selected_workspace()
            .map(|w| w.identity.window_id.clone());
        match result {
            Ok(snapshot) => {
                self.workspaces = snapshot.workspaces.clone();
                self.batches = snapshot
                    .details
                    .iter()
                    .map(|(id, d)| {
                        (
                            id.clone(),
                            d.batch
                                .as_ref()
                                .map(|b| b.display(self.config.redact_labels))
                                .unwrap_or_else(|| {
                                    "Batch: unknown; source/destination unknown".into()
                                }),
                        )
                    })
                    .collect();
                self.snapshot = Some(snapshot);
                self.refresh_current();
                self.refreshed = Instant::now();
                self.stale = false;
                self.refresh_filter();
                if let Some(id) = selected {
                    if let Some(index) = self
                        .visible
                        .iter()
                        .position(|i| self.workspaces[*i].identity.window_id == id)
                    {
                        self.selected = index;
                    } else {
                        self.selection_changed = true;
                        self.error = Some(format!(
                            "Selected window {id} unavailable; move or inspect a row before action"
                        ));
                    }
                }
            }
            Err(error) => {
                self.stale = true;
                self.error = Some(format!(
                    "Refresh failed; retained snapshot is STALE: {error}"
                ));
            }
        }
    }

    fn refresh_filter(&mut self) {
        let selected = self
            .selected_workspace()
            .map(|w| w.identity.window_id.clone());
        self.query.search = self.filter.clone();
        if let Some(snapshot) = &self.snapshot {
            match snapshot.visible(&self.query) {
                Ok(visible) => self.visible = visible,
                Err(error) => {
                    self.visible.clear();
                    self.error = Some(error.to_string());
                }
            }
            self.selected = selected
                .and_then(|id| {
                    self.visible
                        .iter()
                        .position(|i| self.workspaces[*i].identity.window_id == id)
                })
                .unwrap_or(0);
            self.detail_scroll = 0;
            return;
        }
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

pub fn run(
    variant: Variant,
    start: bool,
    query: crate::inventory::Query,
) -> Result<(), CockpitError> {
    let config = Config::load_tmux().map_err(|error| io::Error::other(error.to_string()))?;
    let mut app = App::new(Vec::new(), variant, config);
    app.filter = query.search.clone();
    app.query = query;
    app.refresh();
    if start {
        app.begin_start();
    }
    app.agent_icon = crate::icons::agent_icon();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = event_loop(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableBracketedPaste,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    result
}

fn edit_text(
    text: &mut String,
    cursor: &mut usize,
    key: KeyCode,
    paste: Option<&str>,
    multiline: bool,
) {
    *cursor = (*cursor).min(text.len());
    let previous = text[..*cursor]
        .char_indices()
        .last()
        .map(|(i, _)| i)
        .unwrap_or(0);
    let next = text[*cursor..]
        .chars()
        .next()
        .map(|c| *cursor + c.len_utf8())
        .unwrap_or(*cursor);
    if let Some(paste) = paste {
        let paste: String = paste
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .chars()
            .filter(|c| !c.is_control() || (multiline && matches!(*c, '\n' | '\t')))
            .collect();
        text.insert_str(*cursor, &paste);
        *cursor += paste.len();
        return;
    }
    match key {
        KeyCode::Left => *cursor = previous,
        KeyCode::Right => *cursor = next,
        KeyCode::Home => *cursor = text[..*cursor].rfind('\n').map(|i| i + 1).unwrap_or(0),
        KeyCode::End => *cursor += text[*cursor..].find('\n').unwrap_or(text.len() - *cursor),
        KeyCode::PageUp => *cursor = 0,
        KeyCode::PageDown => *cursor = text.len(),
        KeyCode::Up | KeyCode::Down => {
            let start = text[..*cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let column = text[start..*cursor].chars().count();
            let destination = if key == KeyCode::Up {
                start
                    .checked_sub(1)
                    .map(|end| (text[..end].rfind('\n').map(|i| i + 1).unwrap_or(0), end))
            } else {
                text[*cursor..].find('\n').map(|n| {
                    let start = *cursor + n + 1;
                    (
                        start,
                        start + text[start..].find('\n').unwrap_or(text.len() - start),
                    )
                })
            };
            if let Some((start, end)) = destination {
                *cursor = start
                    + text[start..end]
                        .char_indices()
                        .nth(column)
                        .map(|(i, _)| i)
                        .unwrap_or(end - start);
            }
        }
        KeyCode::Backspace => {
            text.drain(previous..*cursor);
            *cursor = previous;
        }
        KeyCode::Delete => {
            text.drain(*cursor..next);
        }
        KeyCode::Enter if multiline => {
            text.insert(*cursor, '\n');
            *cursor += 1;
        }
        KeyCode::Char(c) if !c.is_control() => {
            text.insert(*cursor, c);
            *cursor += c.len_utf8();
        }
        _ => {}
    }
}

fn input_view(text: &str, cursor: usize, width: usize) -> (String, usize) {
    let cursor = cursor.min(text.len());
    let mut start = 0;
    while Span::raw(&text[start..cursor]).width() >= width.max(1) {
        let Some(c) = text[start..cursor].chars().next() else {
            break;
        };
        start += c.len_utf8();
    }
    let column = Span::raw(&text[start..cursor]).width();
    let mut visible = String::new();
    for c in text[start..].chars() {
        if Span::raw(visible.as_str()).width() + Span::raw(c.to_string()).width() > width {
            break;
        }
        visible.push(c);
    }
    (visible, column)
}

fn editor_rows(text: &str, cursor: usize, width: usize) -> (Vec<String>, usize, usize) {
    let mut rows = vec![String::new()];
    let (mut row, mut col, mut cursor_row, mut cursor_col) = (0, 0, 0, 0);
    for (i, c) in text.char_indices() {
        if i == cursor {
            cursor_row = row;
            cursor_col = col;
        }
        if c == '\n' {
            rows.push(String::new());
            row += 1;
            col = 0;
        } else {
            let glyph = if c == '\t' {
                "    ".into()
            } else {
                c.to_string()
            };
            let columns = Span::raw(glyph.as_str()).width();
            if col + columns > width {
                rows.push(String::new());
                row += 1;
                col = 0;
            }
            rows[row].push_str(&glyph);
            col += columns;
            if col >= width {
                rows.push(String::new());
                row += 1;
                col = 0;
            }
        }
    }
    if cursor >= text.len() {
        cursor_row = row;
        cursor_col = col;
    }
    (rows, cursor_row, cursor_col)
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), CockpitError> {
    loop {
        app.complete_refresh();
        terminal.draw(|frame| render(frame, app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let input = event::read()?;
        if let Event::Paste(text) = &input {
            if app.verification.is_some() {
                app.verification_edit(KeyCode::Null, Some(text));
                continue;
            }
            if app.integration.is_some() {
                app.integration_edit(KeyCode::Null, Some(text));
                continue;
            }
            if app.recovery.is_some() {
                app.recovery_edit(KeyCode::Null, Some(text));
                continue;
            }
            if app.task.is_some() && app.batch_form.is_none() {
                app.launch_edit(KeyCode::Null, Some(text));
            }
            continue;
        }
        let Event::Key(key) = input else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if app.verification.is_some() {
            match key.code {
                KeyCode::Esc => {
                    app.verification = None;
                    app.begin_refresh();
                }
                KeyCode::Tab => {
                    let form = app.verification.as_mut().unwrap();
                    form.field = (form.field + 1) % 3;
                }
                KeyCode::BackTab => {
                    let form = app.verification.as_mut().unwrap();
                    form.field = (form.field + 2) % 3;
                }
                KeyCode::F(5) => {
                    let form = app.verification.as_mut().unwrap();
                    let path = PathBuf::from(&form.fields[0]);
                    let check = form.fields[1].clone();
                    let command = std::mem::take(&mut form.fields[2]);
                    form.cursors[2] = 0;
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        DisableBracketedPaste,
                        LeaveAlternateScreen
                    )?;
                    let result =
                        crate::verification::run(&path, &check, &command, app.config.redact_labels);
                    let status = match result {
                        Ok((text, _)) => text,
                        Err(e) => {
                            if app.config.redact_labels {
                                "Verification unavailable; labels hidden".into()
                            } else {
                                e.to_string()
                            }
                        }
                    };
                    println!("{status}\nPress Enter to return to verification.");
                    enable_raw_mode()?;
                    loop {
                        if matches!(event::read()?, Event::Key(k) if k.code == KeyCode::Enter) {
                            break;
                        }
                    }
                    execute!(
                        terminal.backend_mut(),
                        EnterAlternateScreen,
                        EnableBracketedPaste
                    )?;
                    terminal.clear()?;
                    app.verification.as_mut().unwrap().status = status;
                }
                KeyCode::F(6) => {
                    let form = app.verification.as_mut().unwrap();
                    form.status = crate::verification::inspect(
                        std::path::Path::new(&form.fields[0]),
                        app.config.redact_labels,
                    )
                    .unwrap_or_else(|_| {
                        "Not verified · checkout or live evidence unavailable".into()
                    });
                }
                _ => app.verification_edit(key.code, None),
            }
            continue;
        }
        if app.conflict.is_some() {
            if app.conflict_key(key.code) {
                return Ok(());
            }
            continue;
        }
        if app.integration.is_some() {
            app.integration_key(key.code);
            continue;
        }
        if app.recovery.is_some() {
            if app.recovery_key(key.code) {
                return Ok(());
            }
            continue;
        }
        if app.batch_form.is_some() {
            app.batch_key(key.code);
            continue;
        }
        if app.task.is_some() {
            match key.code {
                KeyCode::Esc => {
                    app.task = None;
                }
                KeyCode::Tab => app.launch.field = (app.launch.field + 1) % 3,
                KeyCode::BackTab => app.launch.field = (app.launch.field + 2) % 3,
                KeyCode::F(3) => app.begin_batch(),
                KeyCode::F(4) => app.next_start_agent(),
                KeyCode::F(5) => {
                    app.launch.file = !app.launch.file;
                    app.task = Some(String::new());
                    app.launch.cursors[2] = 0;
                }
                KeyCode::F(2) if app.active_batch.is_none() => {
                    app.from_current = !app.from_current;
                    app.resolve_start_point();
                }
                KeyCode::F(6) => app.launch_worker()?,
                KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.launch_edit(KeyCode::Home, None)
                }
                KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.launch_edit(KeyCode::End, None)
                }
                _ => app.launch_edit(key.code, None),
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
        if app.help_open {
            // Dismiss help and dispatch an advertised shortcut normally.
            app.help_open = false;
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
                continue;
            }
        }
        if app.details_open {
            match key.code {
                KeyCode::Esc | KeyCode::Char('d') => {
                    app.details_open = false;
                    app.detail_scroll = 0;
                }
                KeyCode::PageDown | KeyCode::Down | KeyCode::Char('j') => {
                    app.detail_scroll = app.detail_scroll.saturating_add(5)
                }
                KeyCode::PageUp | KeyCode::Up | KeyCode::Char('k') => {
                    app.detail_scroll = app.detail_scroll.saturating_sub(5)
                }
                KeyCode::Home => app.detail_scroll = 0,
                _ => {}
            }
            continue;
        }
        if (app.stale || app.refreshing.is_some() || app.selection_changed)
            && matches!(
                key.code,
                KeyCode::Enter | KeyCode::Char('c' | 'f' | 'n' | 'b' | 'o' | 'i' | 'P' | 'C' | 'V')
            )
        {
            app.error = Some(
                if app.refreshing.is_some() {
                    "REFRESHING; wait before action"
                } else if app.selection_changed {
                    "Selected window unavailable; move or inspect a row before action"
                } else {
                    "Snapshot is STALE; refresh successfully before action"
                }
                .into(),
            );
            continue;
        }
        match key.code {
            KeyCode::Char('?') => app.help_open = true,
            KeyCode::Char('i') => app.begin_integration(CheckoutAction::Integrate),
            KeyCode::Char('P') => app.begin_integration(CheckoutAction::Promote),
            KeyCode::Char('C') => app.begin_conflict(None),
            KeyCode::Char('V') => app.begin_verification(),
            KeyCode::Char('d') => {
                app.selection_changed = false;
                app.details_open = true;
                app.detail_scroll = 0;
            }
            KeyCode::Char('g') => {
                app.query.group = if app.query.group == crate::inventory::Group::Project {
                    crate::inventory::Group::Attention
                } else {
                    crate::inventory::Group::Project
                };
                app.refresh_filter();
            }
            KeyCode::Char('s') => {
                app.query.state = app.query.state.next();
                app.refresh_filter();
            }
            KeyCode::Char('p') => {
                // Choosing a project explicitly leaves the exact local-session route.
                app.query.local_session = None;
                let mut ids: Vec<_> = app
                    .snapshot
                    .as_ref()
                    .map(|s| s.projects.keys().cloned().collect())
                    .unwrap_or_default();
                ids.push("unassociated".into());
                app.query.project = match &app.query.project {
                    None => ids.first().cloned(),
                    Some(id) => ids
                        .iter()
                        .position(|p| p == id)
                        .and_then(|i| ids.get(i + 1).cloned()),
                };
                app.refresh_filter();
            }
            KeyCode::Char('w') => {
                app.query.windows = !app.query.windows;
                if app.query.windows {
                    app.query.project = app
                        .selected_workspace()
                        .map(|w| crate::inventory::project_key(&app.workspaces, w));
                }
                app.refresh_filter();
            }
            KeyCode::Char('x') => {
                app.query = crate::inventory::Query::default();
                app.filter.clear();
                app.refresh_filter();
            }
            KeyCode::PageDown => app.move_selection(10),
            KeyCode::PageUp => app.move_selection(-10),
            KeyCode::Home => app.move_selection(-(app.selected as isize)),
            KeyCode::End => app.move_selection(app.visible.len() as isize),
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
            KeyCode::Char('/') => app.filtering = true,
            KeyCode::Char('o') => app.begin_recovery(),
            KeyCode::Char('b') => app.begin_batch(),
            KeyCode::Char('n') => {
                app.begin_start();
            }
            KeyCode::Char('f') => app.begin_integration(CheckoutAction::Finish),
            KeyCode::Char('c') => {
                if let Some(workspace) = app.selected_workspace() {
                    let result = if let Some(project) = &workspace.project {
                        crate::navigation::tmux(&["list-windows","-t",project,"-F","#{window_id}"]).and_then(|windows| {
                            if !windows.lines().any(|id|id==workspace.identity.window_id) {return Err(io::Error::other("Selected project window has vanished; refresh before coordinator navigation"));}
                            crate::coordinator::open_project(project)
                        })
                    } else {
                        crate::coordinator::open_window(&workspace.identity.window_id)
                    };
                    match result {
                        Ok(()) => return Ok(()),
                        Err(error) => app.error = Some(error.to_string()),
                    }
                }
            }
            KeyCode::Char('r') => {
                app.error = None;
                app.begin_refresh();
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
    frame.render_widget(
        Block::default().style(Style::default().fg(app.theme.text).bg(app.theme.base)),
        frame.area(),
    );
    if let Some(form) = &app.verification {
        let sections = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(frame.area());
        frame.render_widget(Paragraph::new("ASSEMBLED VERIFICATION"), sections[0]);
        let mut lines = vec![];
        for (i, label) in [
            "Destination checkout",
            "Check identity",
            "Command (transient)",
        ]
        .iter()
        .enumerate()
        {
            let value = if app.config.redact_labels {
                "[redacted]"
            } else {
                &form.fields[i]
            };
            lines.push(format!(
                "{} {label}: {value}",
                if form.field == i { ">" } else { " " }
            ));
        }
        lines.push(form.status.clone());
        frame.render_widget(
            Paragraph::new(lines.join("\n\n")).wrap(Wrap { trim: false }),
            sections[1],
        );
        frame.render_widget(Paragraph::new("Tab fields · F5 Run visibly\nF6 Inspect live evidence · Esc back\nCommands use bash stdin; no shell history.").wrap(Wrap { trim: false }), sections[2]);
        return;
    }
    if let Some(form) = &app.conflict {
        render_conflict(frame, app, form);
        return;
    }
    if let Some(form) = &app.integration {
        render_integration(frame, app, form);
        return;
    }
    if let Some(form) = &app.recovery {
        render_recovery(frame, app, form);
        return;
    }
    let area = frame.area();
    if app.help_open {
        render_help(frame, app, area);
        return;
    }
    if app.details_open {
        render_detail(frame, app, area);
        return;
    }
    let actions_height = overview_actions(app, area.width).len() as u16;
    let footer_height = if let Some(error) = &app.error {
        let lines = (error.chars().count() + 10).div_ceil((area.width as usize / 2).max(1));
        (lines as u16 + actions_height + 1)
            .min(area.height.saturating_sub(5))
            .max(3)
    } else if app.filtering || !app.filter.is_empty() {
        if area.width < 70 { 4 } else { 3 }
    } else {
        actions_height + 1
    };
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),
            Constraint::Min(5),
            Constraint::Length(footer_height),
        ])
        .split(area);
    render_header(frame, app, layout[0]);
    if area.width < 100 {
        // At popup widths the inventory owns the body. Full details remain one
        // keystroke away, instead of consuming 70% of the workers' viewport.
        render_list(frame, app, layout[1]);
    } else {
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(if area.width >= 140 { 34 } else { 30 }),
            ])
            .split(layout[1]);
        render_list(frame, app, body[0]);
        render_summary(frame, app, body[1]);
    }
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
        let modal = centered(
            area,
            area.width.saturating_sub(4).min(110),
            area.height.saturating_sub(2),
        );
        frame.render_widget(Clear, modal);
        frame.render_widget(
            Block::default()
                .title("START WORKSPACE")
                .borders(Borders::ALL)
                .style(Style::default().bg(app.theme.base).fg(app.theme.text)),
            modal,
        );
        let inner = Rect::new(
            modal.x + 2,
            modal.y + 1,
            modal.width.saturating_sub(4),
            modal.height.saturating_sub(2),
        );
        let private = app.config.redact_labels;
        let label = |value: &str| {
            if private {
                "[redacted]".into()
            } else {
                value.to_owned()
            }
        };
        let (name_view, name_column) = input_view(
            &app.launch.name,
            app.launch.cursors[0],
            inner.width.saturating_sub(15) as usize,
        );
        let (branch_view, branch_column) = input_view(
            &app.launch.branch,
            app.launch.cursors[1],
            inner.width.saturating_sub(11) as usize,
        );
        let source = if app.active_batch.is_some() {
            "Source Pinned batch (F3 new batch)"
        } else if app.from_current {
            "Base   Continue from current branch (F2 change)"
        } else {
            "Base   Independent task (F2 change)"
        };
        let mut lines = vec![
            Line::from(format!(
                "{} Short name: {}",
                if app.launch.field == 0 { ">" } else { " " },
                label(&name_view)
            )),
            Line::from(format!(
                "{} Branch: {}",
                if app.launch.field == 1 { ">" } else { " " },
                label(&branch_view)
            )),
            Line::from(source),
            Line::from(label(
                &app.start_point
                    .as_ref()
                    .map(|p| format!("{} ({})", p.reference, &p.commit[..12]))
                    .unwrap_or_else(|| app.start_error.clone().unwrap_or_default()),
            )),
            Line::from("Local ref; remote freshness unknown"),
            Line::from(match &app.active_batch {
                Some(b) => format!("Batch pinned · Destination {}", label(&b.destination)),
                None => "Batch unknown · F3 choose source and destination".into(),
            }),
            Line::from(format!("Agent {} · F4 change", app.start_agent().label())),
        ];
        lines.push(Line::from("Tab fields · F5 text/file · F6 start"));
        lines.push(Line::from("Enter newline · Esc cancel"));
        let header = lines.len() as u16;
        frame.render_widget(
            Paragraph::new(lines),
            Rect::new(inner.x, inner.y, inner.width, header.min(inner.height)),
        );
        let editor = Rect::new(
            inner.x,
            inner.y + header,
            inner.width,
            inner.height.saturating_sub(header),
        );
        let editor_text = label(task);
        let editor_width = editor.width.saturating_sub(2).max(1) as usize;
        // Explicit visual wrapping makes every task character reachable. Cursor
        // navigation scrolls through the entire input instead of truncating it.
        let (rows, row, col) = editor_rows(
            &editor_text,
            if private { 0 } else { app.launch.cursors[2] },
            editor_width,
        );
        let visible = editor.height.saturating_sub(2).max(1) as usize;
        let scroll = row.saturating_sub(visible - 1);
        frame.render_widget(
            Paragraph::new(rows.join("\n"))
                .scroll((scroll as u16, 0))
                .block(
                    Block::default()
                        .title(if app.launch.file {
                            "Repository task file (agent reads it)"
                        } else {
                            "Task · arrows/Home/End/PgUp/PgDn edit/review"
                        })
                        .borders(Borders::ALL),
                ),
            editor,
        );
        if app.launch.field == 2 && editor.height > 2 && !private {
            frame.set_cursor_position((
                editor.x + 1 + col as u16,
                editor.y + 1 + (row - scroll) as u16,
            ));
        }
        if app.launch.field < 2 && !private && inner.width > 15 {
            let (offset, col) = if app.launch.field == 0 {
                (14, name_column)
            } else {
                (10, branch_column)
            };
            frame.set_cursor_position((
                inner.x + offset + col as u16,
                inner.y + app.launch.field as u16,
            ));
        }
        if let Some(error) = &app.error {
            frame.render_widget(
                Paragraph::new(label(error))
                    .style(Style::default().fg(app.theme.rose))
                    .wrap(Wrap { trim: false }),
                Rect::new(
                    inner.x,
                    inner.y + inner.height.saturating_sub(3),
                    inner.width,
                    3.min(inner.height),
                ),
            );
        }
    }
}

fn render_conflict(frame: &mut ratatui::Frame<'_>, app: &App, form: &ConflictForm) {
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(6),
    ])
    .split(frame.area());
    frame.render_widget(
        Paragraph::new("INTEGRATION CONFLICT").style(
            Style::default()
                .fg(app.theme.rose)
                .add_modifier(Modifier::BOLD),
        ),
        sections[0],
    );
    let mut text = String::new();
    if let Some(error) = &form.error {
        text.push_str(if app.config.redact_labels {
            "Action unavailable; inspect coordinator or Git state. Labels hidden."
        } else {
            error
        });
        text.push_str("\n\n");
    }
    text.push_str(&format!(
        "Recovery agent: {} · F4 changes agent\n",
        app.start_agent().command()
    ));
    text.push_str(&form.status);
    text.push_str("\nSent means transmitted, not accepted. Inspect before deliberate Retry.\n");
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    let max = paragraph
        .line_count(sections[1].width)
        .saturating_sub(sections[1].height as usize) as u16;
    frame.render_widget(paragraph.scroll((form.scroll.min(max), 0)), sections[1]);
    frame.render_widget(Paragraph::new("c Continue · a Abort · o Open\nv Recover agent · t Retry · r Refresh\np Use current project · PgUp/PgDn · Esc back").wrap(Wrap { trim: false }).block(Block::default().borders(Borders::TOP)), sections[2]);
}

fn render_integration(frame: &mut ratatui::Frame<'_>, app: &App, form: &IntegrationForm) {
    let area = frame.area();
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);
    let title = match (form.action, form.preview.is_some()) {
        (CheckoutAction::Integrate, true) => "INTEGRATE PREVIEW",
        (CheckoutAction::Integrate, false) => "INTEGRATE · CHOOSE DESTINATION",
        (CheckoutAction::Promote, true) => "PROMOTE PREVIEW",
        (CheckoutAction::Promote, false) => "PROMOTE · CHOOSE BASE",
        (CheckoutAction::Finish, true) => "FINISH PREVIEW",
        (CheckoutAction::Finish, false) => "FINISH · CHOOSE DESTINATION",
    };
    frame.render_widget(
        Paragraph::new(title).style(
            Style::default()
                .fg(app.theme.rose)
                .add_modifier(Modifier::BOLD),
        ),
        sections[0],
    );
    let redact = app.config.redact_labels;
    let mut text = String::new();
    if let Some(result) = &form.result {
        text.push_str(&format!(
            "{result}\nReviewed state below; r refreshes current ancestry.\n\n"
        ));
    }
    if let Some(error) = &form.error {
        text.push_str(&format!(
            "{}\n\n",
            if redact {
                "Integration unavailable; inspect Git state or choose destination again"
            } else {
                error
            }
        ));
    }
    if let Some(preview) = &form.preview {
        text.push_str(&preview.display(redact));
        if form.action != CheckoutAction::Finish {
            text.push_str(&format!(
                "Destination verification:\n{}\n",
                form.verification
            ));
            text.push_str("Worktrees retained. No push, deployment or automatic cleanup.\n");
        }
    } else {
        text.push_str("Select a destination branch OR a live batch ID.\nMissing metadata never implies a base branch.\n\n");
        for (i, label) in ["Destination branch", "Batch ID"].iter().enumerate() {
            let (value, _) = input_view(
                &form.fields[i],
                form.cursors[i],
                sections[1].width.saturating_sub(23) as usize,
            );
            text.push_str(&format!(
                "{} {label}: {}\n",
                if i == form.field { ">" } else { " " },
                if redact && !value.is_empty() {
                    "[redacted]"
                } else {
                    &value
                }
            ));
        }
        text.push_str("\nThe destination must already have one existing checkout.\nUse Batch setup to create/select one explicitly.\n");
    }
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    let max_scroll = paragraph
        .line_count(sections[1].width)
        .saturating_sub(sections[1].height as usize) as u16;
    frame.render_widget(
        paragraph.scroll((form.scroll.min(max_scroll), 0)),
        sections[1],
    );
    let actions = if form.preview.is_none() {
        "Tab field · Enter preview · Esc cancel"
    } else if form.error.is_some() || form.result.is_some() {
        "r refresh ancestry · e destination · PgUp/PgDn scroll · Esc back"
    } else {
        match form.action {
            CheckoutAction::Integrate => {
                "y integrate reviewed state · e destination · PgUp/PgDn scroll · Esc cancel"
            }
            CheckoutAction::Promote => {
                "y promote reviewed state · e base · PgUp/PgDn scroll · Esc cancel"
            }
            CheckoutAction::Finish => {
                "y remove worktree · branch retained · e destination · Esc cancel"
            }
        }
    };
    frame.render_widget(
        Paragraph::new(actions)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::TOP)),
        sections[2],
    );
}

fn render_recovery(frame: &mut ratatui::Frame<'_>, app: &App, form: &RecoveryForm) {
    let area = frame.area();
    let private = |value: &str| {
        if app.config.redact_labels {
            "[redacted]".to_owned()
        } else {
            value.to_owned()
        }
    };
    if form.restart {
        let groups = Layout::vertical([
            Constraint::Length(5),
            Constraint::Min(3),
            Constraint::Length(if area.width < 80 { 12 } else { 8 }),
        ])
        .split(area);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("RESTART WITH TASK · Fresh conversation"),
                Line::from("Conversation is not restored; no resume offered."),
                Line::from("F4 agent · F5 text/reference · Tab task/batch"),
                Line::from("F6 restart once · Esc cancel"),
                Line::from(format!("Agent: {}", app.start_agent().command())),
            ])
            .wrap(Wrap { trim: false }),
            groups[0],
        );
        let label = if form.file {
            "Task reference"
        } else {
            "Task (multiline)"
        };
        let text = if app.config.redact_labels {
            "[redacted]".to_owned()
        } else {
            form.task.clone()
        };
        let (rows, line, col) = editor_rows(
            &text,
            form.cursors[0].min(text.len()),
            area.width.saturating_sub(2) as usize,
        );
        let height = groups[1].height.saturating_sub(2) as usize;
        let offset = line.saturating_sub(height.saturating_sub(1));
        frame.render_widget(
            Paragraph::new(
                rows.into_iter()
                    .skip(offset)
                    .map(Line::from)
                    .collect::<Vec<_>>(),
            )
            .block(Block::default().borders(Borders::ALL).title(label)),
            groups[1],
        );
        if form.field == 0 && !app.config.redact_labels {
            frame.set_cursor_position((
                groups[1].x + 1 + col as u16,
                groups[1].y + 1 + (line - offset) as u16,
            ));
        }
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!(
                    "{} Batch ID: {}",
                    if form.field == 1 { ">" } else { " " },
                    private(&form.batch)
                )),
                Line::from("Blank deliberately means no batch association."),
                Line::from("Reselect batch after metadata loss; source/destination unknown."),
                Line::from("Historical prompts, checks and exit: Unknown"),
                Line::from(private(
                    form.error
                        .as_deref()
                        .unwrap_or("Review the task and batch choice, then F6."),
                )),
            ])
            .wrap(Wrap { trim: false }),
            groups[2],
        );
        return;
    }
    let groups = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(2),
        Constraint::Length(if area.width < 80 { 12 } else { 8 }),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("RECOVER WORKTREE · selected repository"),
            Line::from("j/k select · Enter open live · r refresh · Esc cancel"),
        ])
        .wrap(Wrap { trim: false }),
        groups[0],
    );
    let rows: Vec<_> = form
        .checkouts
        .iter()
        .map(|row| ListItem::new(format!("{} · {}", row.state(), private(&row.branch))))
        .collect();
    let mut selection =
        ListState::default().with_selected((!rows.is_empty()).then_some(form.selected));
    frame.render_stateful_widget(
        List::new(rows)
            .highlight_symbol("› ")
            .highlight_style(Style::default().fg(app.theme.rose)),
        groups[1],
        &mut selection,
    );
    let detail = form
        .checkouts
        .get(form.selected)
        .map(|r| r.display(app.config.redact_labels))
        .unwrap_or_else(|| "No registered worktrees".into());
    let lines = vec![
        Line::from("s Open shell · t Restart with task"),
        Line::from("c Recover coordinator shell"),
        Line::from("Fresh agent does not restore a conversation."),
        Line::from(detail),
        Line::from(private(
            form.error
                .as_deref()
                .unwrap_or("Files and branches are preserved."),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::TOP)),
        groups[2],
    );
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
    let theme = app.theme;
    let totals = crate::inventory::Totals::from_workspaces(&app.workspaces);
    let top = Rect::new(area.x, area.y, area.width, 5.min(area.height));
    ui::masthead(
        frame,
        top,
        theme,
        "WORKSPACE COCKPIT / ALL PROJECTS",
        &format!(
            "GLOBAL {} workers · {} live · {} project{}",
            totals.workers,
            totals.live,
            totals.projects,
            if totals.projects == 1 { "" } else { "s" }
        ),
    );
    let refresh = if app.refreshing.is_some() {
        "REFRESHING"
    } else if app.stale {
        "STALE: r retry"
    } else {
        "Snapshot: r refresh"
    };
    let mut stats = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!(" {} FAIL ", totals.failed),
            ui::activity_style(Lifecycle::Failed, theme),
        ),
        Span::raw("  "),
        Span::styled(
            format!(" {} INPUT ", totals.input),
            ui::activity_style(Lifecycle::Waiting, theme),
        ),
        Span::raw("  "),
        Span::styled(
            format!(" {} REVIEW ", totals.review),
            ui::activity_style(Lifecycle::Review, theme),
        ),
        Span::styled(
            format!("  {} need you · {} exited", totals.attention, totals.exited),
            Style::default().fg(theme.muted),
        ),
    ]);
    if area.width >= 100 {
        let count = |state| {
            app.workspaces
                .iter()
                .filter(|w| w.is_agent() && w.lifecycle == state)
                .count()
        };
        stats.spans.push(Span::styled(
            format!(
                "  {} WORK  {} RUN  {} UNCONFIRMED",
                count(Lifecycle::Working),
                count(Lifecycle::Running),
                count(Lifecycle::Unknown)
            ),
            Style::default().fg(theme.muted),
        ));
    }
    let requested = app
        .query
        .local_session
        .as_deref()
        .or(app.query.project.as_deref())
        .unwrap_or("all");
    let project = if requested == "all" || requested == "unassociated" {
        requested.to_owned()
    } else {
        app.project_display_label(requested, 24)
    };
    let matching =
        crate::inventory::matching_label(&app.workspaces, &app.visible, app.query.windows);
    let scope = if app.query.local_session.is_some() {
        "session"
    } else {
        "project"
    };
    let filters = format!(
        "  {matching} · {scope} {project} · state {} · / search",
        app.query.state.label()
    );
    let freshness = format!(
        "  {refresh} · {}s{}",
        app.refreshed.elapsed().as_secs(),
        if totals.categories_overlap() {
            " · categories overlap"
        } else {
            ""
        }
    );
    frame.render_widget(
        Paragraph::new(vec![
            stats,
            Line::styled(
                ui::ellipsize(&filters, area.width as usize),
                Style::default().fg(theme.accent()),
            ),
            Line::styled(
                ui::ellipsize(&freshness, area.width as usize),
                Style::default().fg(theme.muted),
            ),
        ])
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.line())),
        ),
        Rect::new(
            area.x,
            area.y + 5,
            area.width,
            area.height.saturating_sub(5),
        ),
    );
}

fn render_list(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let theme = app.theme;
    let muted = Style::default().fg(theme.muted);
    // Optional columns yield before worker identity and activity do.
    let wide = area.width >= 112;
    let branch = area.width >= 86;
    let optional = if wide {
        20 + 9 + 15
    } else if branch {
        22
    } else {
        0
    };
    let agent_width = if area.width >= 60 { 12 } else { 0 };
    let name_width = (area.width as usize).saturating_sub(optional + agent_width + 23);
    let mut columns = vec![ui::cell("  WORKER", name_width, muted)];
    if agent_width > 0 {
        columns.push(ui::cell("AGENT", agent_width, muted));
    }
    columns.push(ui::cell("ACTIVITY", 23, muted));
    if branch {
        columns.push(ui::cell("BRANCH", if wide { 20 } else { 22 }, muted));
    }
    if wide {
        columns.push(ui::cell("GIT", 9, muted));
        columns.push(ui::cell("INTEGRATION", 15, muted));
    }
    frame.render_widget(
        Paragraph::new(Line::from(columns)).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.line())),
        ),
        Rect::new(area.x, area.y, area.width, 2.min(area.height)),
    );
    let body = Rect::new(
        area.x,
        area.y + 2,
        area.width,
        area.height.saturating_sub(2),
    );
    if app.visible.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "  No workspaces match. x clears filters; n starts a worker; r refreshes.",
            )
            .wrap(Wrap { trim: false })
            .style(muted),
            body,
        );
        return;
    }
    let group_key = |w: &Workspace| {
        if app.query.group == crate::inventory::Group::Project {
            crate::inventory::project_key(&app.workspaces, w)
        } else {
            crate::inventory::attention_state(w).label().into()
        }
    };
    let mut counts = HashMap::new();
    for i in &app.visible {
        let w = &app.workspaces[*i];
        let entry = counts.entry(group_key(w)).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += usize::from(crate::inventory::attention_state(w).needs_attention());
    }
    let mut previous = String::new();
    let items = app.visible.iter().map(|index| {
        let w = &app.workspaces[*index];
        let key = group_key(w);
        let mut lines = Vec::new();
        if previous != key {
            previous = key.clone();
            let label = if app.query.group == crate::inventory::Group::Attention {
                key.clone()
            } else {
                app.project_display_label(&key, area.width.saturating_sub(34) as usize)
            };
            lines.push(Line::styled(
                format!(
                    "  {label} · {} matching · {} need you",
                    counts[&key].0, counts[&key].1
                ),
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ));
        }
        let selected = app.visible.get(app.selected) == Some(index);
        let row_style = Style::default().bg(if selected {
            theme.surface()
        } else {
            theme.base
        });
        let role = if w.coordinator.as_deref() == Some(w.identity.window_id.as_str()) {
            "COORD"
        } else if w.checkout.is_linked_worktree {
            "WT"
        } else if w.is_agent() {
            "AGENT"
        } else {
            "SH"
        };
        let name = format!(
            "{} {role}  {}{}",
            if selected { "▎" } else { " " },
            if w.identity.window_id == app.current_window {
                "• "
            } else {
                ""
            },
            app.workspace_label(w)
        );
        let mut row = vec![ui::cell(
            &name,
            name_width,
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        )];
        if agent_width > 0 {
            row.push(ui::cell(
                w.agent.label(),
                agent_width,
                Style::default().fg(theme.text),
            ));
        }
        let state = format!(
            " {}{} ",
            ui::activity(w.lifecycle),
            if w.process == "exited" {
                if crate::inventory::has_failure(w) {
                    " / EXIT FAILED"
                } else {
                    " / EXIT"
                }
            } else {
                ""
            }
        );
        let state = ui::ellipsize(&state, 22);
        let remaining = 23usize.saturating_sub(Line::from(state.as_str()).width());
        row.push(Span::styled(
            state,
            ui::activity_style(crate::inventory::attention_state(w), theme),
        ));
        row.push(Span::raw(" ".repeat(remaining)));
        if branch {
            row.push(ui::cell(
                if app.config.redact_labels {
                    "[redacted]"
                } else {
                    w.checkout.branch.as_deref().unwrap_or("unknown")
                },
                if wide { 20 } else { 22 },
                muted,
            ));
        }
        if wide {
            row.push(ui::cell(
                match w.checkout.git_state {
                    GitState::Clean => "clean",
                    GitState::Dirty => "dirty",
                    GitState::Unknown => "unknown",
                },
                9,
                muted,
            ));
            let integration = app
                .snapshot
                .as_ref()
                .and_then(|s| s.details.get(&w.identity.window_id))
                .and_then(|d| d.integration.as_deref())
                .unwrap_or("unknown");
            row.push(ui::cell(integration, 15, muted));
        }
        lines.push(Line::from(row).style(row_style));
        lines.push(Line::styled(" ".repeat(area.width as usize), row_style));
        ListItem::new(lines)
    });
    let mut state = ListState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(List::new(items), body, &mut state);
}

fn render_summary(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let theme = app.theme;
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(theme.line()));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let content = Rect::new(
        inner.x + 2,
        inner.y + 1,
        inner.width.saturating_sub(4),
        inner.height.saturating_sub(1),
    );
    let Some(w) = app.selected_workspace() else {
        return;
    };
    let private = app.config.redact_labels;
    let detail = app
        .snapshot
        .as_ref()
        .and_then(|s| s.details.get(&w.identity.window_id));
    let mut lines = vec![
        Line::styled("SELECTED WORKER", Style::default().fg(theme.accent())),
        Line::default(),
        Line::styled(
            app.workspace_label(w).to_owned(),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Line::default(),
        Line::from(Span::styled(
            format!(" {} ", ui::activity(w.lifecycle)),
            ui::activity_style(w.lifecycle, theme),
        )),
        Line::default(),
    ];
    let destination = detail
        .and_then(|d| d.batch.as_ref())
        .map(|b| b.destination.as_str())
        .unwrap_or("Not configured");
    let fields = [
        (
            "Project / session",
            if private {
                "[redacted]".into()
            } else {
                format!(
                    "{} / {}",
                    app.project_display_label(
                        &crate::inventory::project_key(&app.workspaces, w),
                        content.width as usize
                    ),
                    w.identity.session
                )
            },
        ),
        ("Workspace", w.role().to_owned()),
        ("Agent", w.agent.label().to_owned()),
        (
            "Branch",
            if private {
                "[redacted]".into()
            } else {
                w.checkout
                    .branch
                    .clone()
                    .unwrap_or_else(|| "unknown".into())
            },
        ),
        (
            "Integrate into",
            if private {
                "[redacted]".into()
            } else {
                destination.into()
            },
        ),
        (
            "Activity evidence",
            format!("{} · {}", w.evidence.label(), w.process_label()),
        ),
        (
            "Integration",
            detail
                .and_then(|d| d.integration.clone())
                .unwrap_or_else(|| "unknown".into()),
        ),
        (
            "Verification",
            if private {
                "[redacted]".into()
            } else {
                detail
                    .and_then(|d| d.verification.clone())
                    .unwrap_or_else(|| "Not verified".into())
            },
        ),
    ];
    for (label, value) in fields {
        lines.push(Line::styled(label, Style::default().fg(theme.muted)));
        lines.push(Line::from(value));
        if content.height >= 38 {
            lines.push(Line::default());
        }
    }
    let footer = vec![
        Line::styled(
            "[Enter] Open workspace",
            Style::default().fg(theme.accent()),
        ),
        Line::styled("[d] Full details", Style::default().fg(theme.accent())),
    ];
    let body = Rect::new(
        content.x,
        content.y,
        content.width,
        content.height.saturating_sub(3),
    );
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
    frame.render_widget(
        Paragraph::new(footer),
        Rect::new(
            content.x,
            content.y + body.height + 1,
            content.width,
            2.min(content.height),
        ),
    );
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
    let color = match crate::inventory::attention_state(workspace) {
        Lifecycle::Waiting => app.theme.gold,
        Lifecycle::Review => app.theme.pine,
        Lifecycle::Failed => app.theme.love,
        Lifecycle::Running | Lifecycle::Working | Lifecycle::Starting => app.theme.rose,
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
        workspace
            .checkout
            .branch
            .as_deref()
            .unwrap_or("unknown/detached")
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
        detail_line(
            "EVIDENCE",
            format!(
                "{} · {}",
                workspace.evidence.label(),
                workspace.process_label()
            ),
            app.theme.muted,
        ),
        detail_line("ROLE", workspace.role().into(), app.theme.muted),
        detail_line(
            "IDENTITY",
            format!(
                "{} {} · project {}",
                workspace.identity.window_id,
                workspace.identity.pane_id,
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
    if workspace.process == "exited" {
        lines.insert(
            2,
            detail_line(
                "RESULT",
                format!(
                    "exit code {} · signal {}",
                    workspace
                        .exit_code
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "unknown".into()),
                    workspace.exit_signal.as_deref().unwrap_or("unknown")
                ),
                app.theme.muted,
            ),
        );
        lines.insert(
            3,
            detail_line(
                "EXIT TIME",
                workspace
                    .exit_time
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "unknown".into()),
                app.theme.muted,
            ),
        );
        lines.insert(
            4,
            detail_line(
                "EXIT",
                "Not task completion; checks unknown".into(),
                app.theme.muted,
            ),
        );
    }
    lines.extend(
        app.batches
            .get(&workspace.identity.window_id)
            .map(String::as_str)
            .unwrap_or("Batch: unknown; source/destination unknown")
            .lines()
            .map(|line| Line::from(format!(" {line}"))),
    );
    lines.extend([
        detail_line(
            "SESSIONS",
            if private {
                "[redacted]".into()
            } else {
                workspace.identity.sessions.join(", ")
            },
            app.theme.muted,
        ),
        detail_line(
            "PROJECT",
            app.project_label(workspace).into(),
            app.theme.muted,
        ),
    ]);
    if let Some(detail) = app
        .snapshot
        .as_ref()
        .and_then(|s| s.details.get(&workspace.identity.window_id))
    {
        for (label, value) in [
            ("COMMIT", detail.commit.as_deref().unwrap_or("unknown")),
            (
                "SOURCE",
                detail.source_commit.as_deref().unwrap_or("unknown"),
            ),
            (
                "TASK REF",
                detail.task_reference.as_deref().unwrap_or("unknown"),
            ),
            (
                "DELIVERY",
                if detail.delivery.is_empty() {
                    "unknown"
                } else {
                    &detail.delivery
                },
            ),
        ] {
            lines.push(detail_line(
                label,
                if private && label != "DELIVERY" {
                    "[redacted]".into()
                } else {
                    value.into()
                },
                app.theme.muted,
            ));
        }
        lines.push(Line::from(format!(
            " CHANGES   {} file names",
            detail.changed_files.len()
        )));
        for name in &detail.changed_files {
            lines.push(Line::from(if private {
                " [redacted]".into()
            } else {
                format!(" {name:?}")
            }));
        }
        if let Some(warning) = &detail.warning {
            lines.push(Line::from(if private {
                " Metadata unavailable".into()
            } else {
                format!(" UNAVAILABLE {warning}")
            }));
        }
    }
    let integration = app
        .snapshot
        .as_ref()
        .and_then(|s| s.details.get(&workspace.identity.window_id))
        .and_then(|detail| detail.integration.as_deref())
        .unwrap_or("unknown · choose a live batch or i explicit destination");
    lines.push(Line::from(format!(" Integration: {integration}")));
    let verification = app.snapshot.as_ref()
        .and_then(|s| s.details.get(&workspace.identity.window_id))
        .and_then(|d| d.verification.as_deref())
        .unwrap_or("Assembled verification: Not verified · destination unknown\nReported worker checks: unknown (Review is not evidence)");
    for line in verification.lines() {
        let value = if private
            && ["Check:", "Checkout:", "Tested revision:"]
                .iter()
                .any(|prefix| line.starts_with(prefix))
        {
            format!("{} [redacted]", line.split(':').next().unwrap_or("Label"))
        } else {
            line.into()
        };
        lines.push(Line::from(value));
    }
    lines.push(Line::from(" V verify · explicitly select assembled checks"));
    lines.push(Line::from(
        " i integrate · preview explicit destination and ancestry",
    ));
    lines.push(Line::from(""));
    lines.push(Line::styled(
        " NEXT",
        Style::default()
            .fg(app.theme.rose)
            .add_modifier(Modifier::BOLD),
    ));
    let next = match workspace.lifecycle {
        Lifecycle::Waiting => " Enter  open · input required",
        Lifecycle::Review => " Enter  open · inspect; checks unknown",
        Lifecycle::Failed => " Enter  open · inspect failure",
        Lifecycle::Running | Lifecycle::Working | Lifecycle::Starting => {
            " Enter  open · agent active"
        }
        Lifecycle::Unknown => " Enter  open workspace",
    };
    lines.push(Line::styled(next, Style::default().fg(app.theme.text)));
    let finish = if !workspace.checkout.is_linked_worktree {
        " f      unavailable · primary checkout"
    } else if workspace.checkout.git_state != GitState::Clean {
        " f      unavailable · worktree dirty"
    } else {
        " f      finish · selected destination and writer guards"
    };
    lines.push(Line::styled(finish, Style::default().fg(app.theme.muted)));
    // Use the renderer's actual wrapping, including word boundaries and borders,
    // so every last detail remains reachable at narrow widths.
    let block = Block::default()
        .title(if app.details_open {
            " DETAILS · PgUp/PgDn scroll · Esc back "
        } else {
            " SELECTED · d full details "
        })
        .borders(Borders::LEFT);
    let inner = block.inner(area);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let max_scroll = paragraph
        .line_count(inner.width)
        .saturating_sub(inner.height as usize)
        .min(u16::MAX as usize) as u16;
    frame.render_widget(
        paragraph
            .scroll((app.detail_scroll.min(max_scroll), 0))
            .block(block),
        area,
    );
}

fn detail_line(label: &'static str, value: String, muted: ratatui::style::Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {label:<9}"), Style::default().fg(muted)),
        Span::raw(value),
    ])
}

fn age(since: Option<u64>) -> String {
    let Some(since) = since else {
        return "unknown".into();
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

/// Wrap between whole actions, never between a key and its label.
fn overview_actions(app: &App, width: u16) -> Vec<Line<'static>> {
    let actions: Vec<_> = COCKPIT_NAVIGATION
        .iter()
        .chain(
            COCKPIT_ACTIONS
                .iter()
                .filter(|(key, _)| matches!(*key, "d" | "/")),
        )
        .chain(std::iter::once(&("?", "Actions")))
        .chain(CLOSE_ACTION)
        .copied()
        .collect();
    ui::action_lines(&[&actions], app.theme, width)
}

fn render_help(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let mut lines = vec![Line::styled(
        " COCKPIT ACTIONS · Esc / ? back",
        Style::default()
            .fg(app.theme.rose)
            .add_modifier(Modifier::BOLD),
    )];
    for (key, label) in COCKPIT_NAVIGATION
        .iter()
        .chain(COCKPIT_ACTIONS)
        .chain(CLOSE_ACTION)
    {
        lines.push(Line::from(format!(" {key:<8} {label}")));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_footer(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    if app.filtering {
        let message = if app.config.redact_labels && !app.filter.is_empty() {
            "[redacted]"
        } else {
            &app.filter
        };
        frame.render_widget(
            Paragraph::new(vec![
                ui::action_line(&[FILTER_ACTIONS], app.theme),
                Line::from(format!(" SEARCH > {message}_")),
            ])
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::TOP)),
            area,
        );
        return;
    }

    if app.error.is_none() && app.filter.is_empty() {
        frame.render_widget(
            Paragraph::new(overview_actions(app, area.width))
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::TOP)),
            area,
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
            Paragraph::new({
                let mut lines = overview_actions(app, area.width);
                lines.push(Line::from(format!(" ERROR  {message}")));
                lines
            })
            .style(Style::default().fg(app.theme.love))
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::TOP)),
            area,
        );
        return;
    }
    let query = if app.config.redact_labels {
        "[redacted]"
    } else {
        app.filter.as_str()
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(" / search · x clear · d details · Enter open"),
            Line::from(format!(" SEARCH {query}")),
        ])
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::TOP)),
        area,
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
            process: "running".into(),
            exit_code: None,
            exit_signal: None,
            exit_time: None,
        }
    }

    #[test]
    fn branded_header_keeps_freshness_and_overlap_at_breakpoints() {
        let mut w = workspace("main", Lifecycle::Review);
        w.process = "exited".into();
        w.exit_code = Some(23);
        let mut app = App::new(vec![w], Variant::Moon, Config::default());
        app.stale = true;
        app.query.project = Some(format!("repo:/private/{}", "long-project-".repeat(20)));
        for width in [48, 64, 70, 84, 100, 120, 160] {
            let mut terminal = Terminal::new(TestBackend::new(width, 27)).unwrap();
            terminal.draw(|frame| render(frame, &app)).unwrap();
            let content = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(
                content.contains("STALE: r retry"),
                "freshness lost at {width}"
            );
            assert!(
                content.contains("categories overlap"),
                "overlap lost at {width}"
            );
        }
    }

    #[test]
    fn activity_evidence_and_exit_receipt_are_readable() {
        for width in [80, 120, 160] {
            for (state, source, process, code) in [
                (Lifecycle::Running, EvidenceSource::Process, "running", None),
                (Lifecycle::Working, EvidenceSource::Hook, "running", None),
                (Lifecycle::Review, EvidenceSource::Hook, "exited", Some(23)),
                (
                    Lifecycle::Unknown,
                    EvidenceSource::Process,
                    "exited",
                    Some(0),
                ),
            ] {
                let mut worker = workspace("work/test", state);
                worker.evidence = source;
                worker.process = process.into();
                worker.exit_code = code;
                worker.exit_time = code.map(|_| 1234567890);
                let mut app = App::new(vec![worker], Variant::Moon, Config::default());
                // The approved overview has a concise sidebar; complete receipts
                // remain available through the full-details surface at every width.
                app.details_open = true;
                let mut terminal = Terminal::new(TestBackend::new(width, 40)).unwrap();
                terminal.draw(|frame| render(frame, &app)).unwrap();
                let content = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|c| c.symbol())
                    .collect::<String>();
                assert!(content.contains(state.label()));
                assert!(content.contains(source.label()));
                if let Some(code) = code {
                    assert!(
                        content.contains(&format!("code {code}")),
                        "missing exit code at {width}: {content}"
                    );
                    assert!(content.contains("1234567890"));
                    assert!(content.contains("Not task completion"));
                }
            }
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
        app.details_open = true;
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
        coordinator.process.clear();
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
                app.details_open = true;
                terminal.draw(|frame| render(frame, &app)).unwrap();
                let content = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>();
                app.details_open = false;
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
    fn long_multiline_task_keeps_the_cursor_and_last_line_visible() {
        for width in [48, 64, 80, 120, 160] {
            let mut app = App::new(vec![], Variant::Moon, Config::default());
            let task = (0..20)
                .map(|n| format!("line {n:02}: private café 界\tinstructions for width {width}"))
                .collect::<Vec<_>>()
                .join("\n");
            app.launch.cursors[2] = task.len();
            app.launch.field = 2;
            app.task = Some(task);
            let mut terminal = Terminal::new(TestBackend::new(width, 24)).unwrap();
            terminal.draw(|frame| render(frame, &app)).unwrap();
            let content = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(content.contains("line 19:"), "width {width}: {content}");
        }
    }

    #[test]
    fn start_form_wraps_task_for_complete_review() {
        let mut app = App::new(vec![], Variant::Moon, Config::default());
        app.task = Some(
            "Audit workspace lifecycle and worktree edge cases; add regression tests and fix any failures"
                .into(),
        );
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();

        let modal = centered(Rect::new(0, 0, 100, 24), 96, 22);
        let buffer = terminal.backend().buffer();
        for y in [modal.y + 3, modal.y + 4] {
            assert_eq!(buffer.cell((modal.right() - 2, y)).unwrap().symbol(), " ");
        }
        let content = (modal.y + 11..modal.bottom() - 2)
            .map(|y| {
                (modal.x + 3..modal.right() - 3)
                    .map(|x| buffer.cell((x, y)).unwrap().symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect::<String>();
        assert_eq!(content, app.task.as_deref().unwrap());
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
