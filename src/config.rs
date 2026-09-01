use std::{collections::HashMap, io, process::Command};

use thiserror::Error;

use crate::domain::AgentKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub base_branch: String,
    pub default_agent: AgentKind,
    pub branch_prefix: String,
    pub redact_labels: bool,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid {option}: {value}")]
    Invalid { option: String, value: String },
    #[error("could not read tmux configuration: {0}")]
    Tmux(#[from] io::Error),
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_branch: "main".into(),
            default_agent: AgentKind::Codex,
            branch_prefix: "work/".into(),
            redact_labels: false,
        }
    }
}

impl Config {
    pub fn load_tmux() -> Result<Self, ConfigError> {
        let mut options = Vec::new();
        for name in [
            "@agent-watch-base-branch",
            "@agent-watch-agent",
            "@agent-watch-branch-prefix",
            "@agent-watch-redact-labels",
        ] {
            let output = Command::new("tmux")
                .args(["show-option", "-gqv", name])
                .output()?;
            if output.status.success() {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                if !value.is_empty() {
                    options.push((name.to_owned(), value));
                }
            }
        }
        Self::from_options(options)
    }

    pub fn from_options<K, V, I>(options: I) -> Result<Self, ConfigError>
    where
        K: AsRef<str>,
        V: AsRef<str>,
        I: IntoIterator<Item = (K, V)>,
    {
        let values = options
            .into_iter()
            .map(|(key, value)| (key.as_ref().to_owned(), value.as_ref().to_owned()))
            .collect::<HashMap<_, _>>();
        let mut config = Self::default();
        if let Some(value) = values.get("@agent-watch-base-branch") {
            config.base_branch = nonempty("@agent-watch-base-branch", value)?;
        }
        if let Some(value) = values.get("@agent-watch-branch-prefix") {
            config.branch_prefix = nonempty("@agent-watch-branch-prefix", value)?;
        }
        if let Some(value) = values.get("@agent-watch-agent") {
            config.default_agent =
                AgentKind::from_command(value).ok_or_else(|| ConfigError::Invalid {
                    option: "@agent-watch-agent".into(),
                    value: value.clone(),
                })?;
        }
        if let Some(value) = values.get("@agent-watch-redact-labels") {
            config.redact_labels = match value.as_str() {
                "on" | "true" | "1" => true,
                "off" | "false" | "0" => false,
                _ => {
                    return Err(ConfigError::Invalid {
                        option: "@agent-watch-redact-labels".into(),
                        value: value.clone(),
                    });
                }
            };
        }
        Ok(config)
    }
}

fn nonempty(option: &str, value: &str) -> Result<String, ConfigError> {
    (!value.trim().is_empty())
        .then(|| value.trim().to_owned())
        .ok_or_else(|| ConfigError::Invalid {
            option: option.into(),
            value: value.into(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typed_tmux_configuration() {
        let config = Config::from_options([
            ("@agent-watch-base-branch", "trunk"),
            ("@agent-watch-agent", "claude"),
            ("@agent-watch-branch-prefix", "quick-win/"),
            ("@agent-watch-redact-labels", "on"),
        ])
        .expect("valid options");
        assert_eq!(config.base_branch, "trunk");
        assert_eq!(config.default_agent, AgentKind::Claude);
        assert_eq!(config.branch_prefix, "quick-win/");
        assert!(config.redact_labels);
    }

    #[test]
    fn rejects_invalid_typed_tmux_configuration() {
        let error = Config::from_options([("@agent-watch-agent", "vim")])
            .expect_err("unknown agent must be rejected");
        assert!(error.to_string().contains("@agent-watch-agent"));
    }
}
