use serde::Deserialize;

use crate::i18n::{Lang, fill};

const LOG_LIMIT: usize = 200;
const CODE_PARAMS: [&str; 2] = ["user_code=", "code="];
const MIN_CODE_PART: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AddEvent {
    Started,
    Url { url: String },
    Output { line: String },
    Done,
    Error { message: String },
}

#[must_use]
pub fn parse_add_event(line: &str) -> Option<AddEvent> {
    serde_json::from_str(line.trim()).ok()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Form,
    Running,
    Done,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Flow {
    pub phase: Phase,
    pub url: Option<String>,
    pub code: Option<String>,
    pub log: Vec<String>,
}

impl Flow {
    pub fn start(&mut self) {
        *self = Flow {
            phase: Phase::Running,
            ..Flow::default()
        };
    }

    pub fn reset(&mut self) {
        *self = Flow::default();
    }

    pub fn event(&mut self, event: AddEvent) {
        if self.phase != Phase::Running {
            return;
        }
        match event {
            AddEvent::Started => {}
            AddEvent::Url { url } => self.found_url(url),
            AddEvent::Output { line } => {
                if self.code.is_none() {
                    self.code = device_code(&line);
                }
                self.push_log(line);
            }
            AddEvent::Done => self.phase = Phase::Done,
            AddEvent::Error { message } => self.phase = Phase::Failed(message),
        }
    }

    pub fn exited(&mut self, lang: Lang, failure: Option<String>) {
        if self.phase != Phase::Running {
            return;
        }
        self.phase = match failure {
            None => Phase::Done,
            Some(message) if message.trim().is_empty() => {
                Phase::Failed(lang.tr("The sign-in stopped before it finished").into())
            }
            Some(message) => Phase::Failed(message),
        };
    }

    pub fn fail(&mut self, message: String) {
        self.phase = Phase::Failed(message);
    }

    fn found_url(&mut self, url: String) {
        if self.code.is_none() {
            self.code = code_in_url(&url);
        }
        self.push_log(url.clone());
        if self.url.is_none() {
            self.url = Some(url);
        }
    }

    fn push_log(&mut self, line: String) {
        if self.log.len() == LOG_LIMIT {
            self.log.remove(0);
        }
        self.log.push(line);
    }
}

#[must_use]
pub fn exit_message(lang: Lang, status: i32) -> String {
    fill(
        lang.tr("headroom exited with status {status}"),
        &[("status", &status.to_string())],
    )
}

fn looks_like_code(word: &str) -> bool {
    let parts: Vec<&str> = word.split('-').collect();
    parts.len() >= 2
        && parts.iter().all(|part| {
            part.len() >= MIN_CODE_PART
                && part
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        })
}

fn code_in_url(url: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        CODE_PARAMS.iter().find_map(|name| {
            let value = pair.strip_prefix(name)?;
            looks_like_code(value).then(|| value.to_owned())
        })
    })
}

#[must_use]
pub fn device_code(line: &str) -> Option<String> {
    if !line.to_lowercase().contains("code") {
        return None;
    }
    line.split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-'))
        .find(|word| looks_like_code(word))
        .map(str::to_owned)
}

#[cfg(test)]
#[path = "flow_tests.rs"]
mod tests;
