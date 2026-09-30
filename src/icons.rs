//! Share the shell status bar's font policy with standalone Rust interfaces.
use std::process::Command;

pub(crate) fn agent_icon() -> String {
    policy().1
}

pub(crate) fn policy() -> (String, String) {
    Command::new("bash")
        .args(["-c", include_str!("../scripts/icons.sh")])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            let text = String::from_utf8(output.stdout).ok()?;
            let mut lines = text.lines();
            Some((lines.next()?.to_owned(), lines.next()?.to_owned()))
        })
        .unwrap_or_else(|| ("safe".into(), "A".into()))
}
