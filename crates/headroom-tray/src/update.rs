use serde::Deserialize;

use crate::i18n::{Lang, fill};
use crate::payload::{InstallKind, Update};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateAction {
    Install,
    Command,
    Notes,
}

impl UpdateAction {
    #[must_use]
    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            UpdateAction::Install => lang.tr("Update"),
            UpdateAction::Command => lang.tr("How to update"),
            UpdateAction::Notes => lang.tr("Release notes"),
        }
    }
}

fn release_url(update: &Update) -> Option<&str> {
    let url = update.url.trim();
    (url.starts_with("https://") && !url.contains(char::is_whitespace)).then_some(url)
}

#[must_use]
pub fn update_action(update: &Update) -> Option<UpdateAction> {
    match update.install {
        InstallKind::SelfManaged => Some(UpdateAction::Install),
        InstallKind::Package if !update.command.trim().is_empty() => Some(UpdateAction::Command),
        _ => release_url(update).map(|_| UpdateAction::Notes),
    }
}

#[must_use]
pub fn whats_new_url(update: &Update) -> Option<&str> {
    release_url(update).filter(|_| update_action(update) != Some(UpdateAction::Notes))
}

#[must_use]
pub fn notes_url(update: &Update) -> Option<&str> {
    release_url(update)
}

#[must_use]
pub fn update_title(lang: Lang, update: &Update) -> String {
    fill(
        lang.tr("Headroom {version} is available"),
        &[("version", &update.version)],
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ProgressEvent {
    Step {
        text: String,
    },
    Done {
        version: Option<String>,
        relogin: bool,
    },
    Error {
        message: String,
    },
}

#[must_use]
pub fn parse_progress(line: &str) -> Option<ProgressEvent> {
    serde_json::from_str(line.trim()).ok()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateRun {
    #[default]
    Idle,
    Running(Option<String>),
    Done {
        version: Option<String>,
        relogin: bool,
    },
    Failed(String),
}

impl UpdateRun {
    #[must_use]
    pub fn after_event(self, event: ProgressEvent) -> Self {
        if !matches!(self, UpdateRun::Running(_)) {
            return self;
        }
        match event {
            ProgressEvent::Step { text } => UpdateRun::Running(Some(text)),
            ProgressEvent::Done { version, relogin } => UpdateRun::Done { version, relogin },
            ProgressEvent::Error { message } => UpdateRun::Failed(message),
        }
    }

    #[must_use]
    pub fn after_exit(self, lang: Lang, error: Option<String>) -> Self {
        match self {
            UpdateRun::Running(_) => UpdateRun::Failed(
                error
                    .unwrap_or_else(|| lang.tr("The update stopped before it finished").to_owned()),
            ),
            other => other,
        }
    }

    #[must_use]
    pub fn line(&self, lang: Lang) -> Option<String> {
        match self {
            UpdateRun::Idle => None,
            UpdateRun::Running(step) => Some(
                step.clone()
                    .unwrap_or_else(|| lang.tr("Starting the update…").to_owned()),
            ),
            UpdateRun::Done { relogin: true, .. } => Some(
                lang.tr("Updated — log out and back in to finish")
                    .to_owned(),
            ),
            UpdateRun::Done {
                version: Some(version),
                ..
            } => Some(fill(
                lang.tr("Updated to {version}"),
                &[("version", version)],
            )),
            UpdateRun::Done { .. } => Some(lang.tr("Updated").to_owned()),
            UpdateRun::Failed(message) => Some(message.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(install: InstallKind, command: &str, url: &str) -> Update {
        Update {
            version: "0.5.0".into(),
            url: url.into(),
            install,
            command: command.into(),
        }
    }

    const URL: &str = "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0";

    #[test]
    fn actions_follow_the_install_kind() {
        let own = update(InstallKind::SelfManaged, "headroom update", URL);
        assert_eq!(update_action(&own), Some(UpdateAction::Install));
        assert_eq!(whats_new_url(&own), Some(URL));
        let package = update(InstallKind::Package, "Download the new .deb", URL);
        assert_eq!(update_action(&package), Some(UpdateAction::Command));
        let unknown = update(InstallKind::Unknown, URL, URL);
        assert_eq!(update_action(&unknown), Some(UpdateAction::Notes));
        assert_eq!(whats_new_url(&unknown), None);
        let broken = update(InstallKind::Unknown, "", "http://x");
        assert_eq!(update_action(&broken), None);
        assert_eq!(update_title(Lang::En, &own), "Headroom 0.5.0 is available");
    }

    #[test]
    fn parses_progress_lines() {
        assert_eq!(
            parse_progress(r#"{"event":"step","text":"Unpacking…"}"#),
            Some(ProgressEvent::Step {
                text: "Unpacking…".into()
            })
        );
        assert_eq!(
            parse_progress(r#"{"event":"done","version":"0.5.0","relogin":true}"#),
            Some(ProgressEvent::Done {
                version: Some("0.5.0".into()),
                relogin: true
            })
        );
        assert_eq!(parse_progress("==> not json"), None);
    }

    #[test]
    fn run_state_machine() {
        let run = UpdateRun::Running(None);
        assert_eq!(run.line(Lang::En).as_deref(), Some("Starting the update…"));
        let run = run.after_event(ProgressEvent::Step {
            text: "Verifying".into(),
        });
        assert_eq!(run.line(Lang::En).as_deref(), Some("Verifying"));
        let done = run.clone().after_event(ProgressEvent::Done {
            version: Some("0.5.0".into()),
            relogin: false,
        });
        assert_eq!(done.line(Lang::En).as_deref(), Some("Updated to 0.5.0"));
        assert_eq!(done.clone().after_exit(Lang::En, None), done);
        let failed = run.after_exit(Lang::En, None);
        assert_eq!(
            failed.line(Lang::En).as_deref(),
            Some("The update stopped before it finished")
        );
        assert_eq!(UpdateRun::Idle.line(Lang::En), None);
    }
}
