//! Explicit delegation to the user's installed tmux-resurrect save script.

use std::process::{Command, Stdio};

pub(crate) fn save() -> String {
    let Ok(output) = Command::new("tmux")
        .args(["show-option", "-gqv", "@resurrect-save-script-path"])
        .output()
    else {
        return "Save unavailable: could not query tmux".into();
    };
    let path = String::from_utf8_lossy(&output.stdout);
    let path = path.trim_end();
    if !output.status.success() || path.is_empty() {
        return "Save unavailable: load tmux-resurrect first".into();
    }

    // Use an executable path, not a shell command. Resurrect owns its snapshot
    // and capture settings; no snapshot or pane content passes through us.
    match Command::new(path)
        .arg("quiet")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) if status.success() => "Saved via tmux-resurrect".into(),
        Ok(_) => "Save failed: tmux-resurrect returned an error".into(),
        Err(_) => "Save failed: could not run tmux-resurrect's save script".into(),
    }
}
