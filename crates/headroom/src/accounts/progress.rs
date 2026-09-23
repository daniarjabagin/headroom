use std::io::Write;

use anyhow::Result;
use headroom_core::account::ProviderKind;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ProgressEvent {
    Started {
        provider: ProviderKind,
        home: String,
    },
    Url {
        url: String,
    },
    Output {
        line: String,
    },
    Done {
        account_id: String,
        label: Option<String>,
    },
    Error {
        message: String,
    },
}

pub struct JsonLines<W: Write> {
    out: W,
}

impl<W: Write> JsonLines<W> {
    pub fn new(out: W) -> JsonLines<W> {
        JsonLines { out }
    }

    pub fn emit(&mut self, event: &ProgressEvent) -> Result<()> {
        serde_json::to_writer(&mut self.out, event)?;
        self.out.write_all(b"\n")?;
        self.out.flush()?;
        Ok(())
    }

    pub fn finish<T>(
        &mut self,
        result: Result<T>,
        done: impl FnOnce(&T) -> ProgressEvent,
    ) -> Result<T> {
        match result {
            Ok(value) => {
                self.emit(&done(&value))?;
                Ok(value)
            }
            Err(error) => {
                self.emit(&ProgressEvent::Error {
                    message: format!("{error:#}"),
                })?;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(events: &[ProgressEvent]) -> String {
        let mut out = JsonLines::new(Vec::new());
        for event in events {
            out.emit(event).unwrap();
        }
        String::from_utf8(out.out).unwrap()
    }

    #[test]
    fn events_are_one_tagged_object_per_line() {
        let text = lines(&[
            ProgressEvent::Started {
                provider: ProviderKind::Codex,
                home: "/data/codex/1".into(),
            },
            ProgressEvent::Url {
                url: "https://auth.example/".into(),
            },
            ProgressEvent::Output {
                line: "Paste the code".into(),
            },
            ProgressEvent::Done {
                account_id: "codex:abc".into(),
                label: None,
            },
            ProgressEvent::Error {
                message: "boom".into(),
            },
        ]);
        let expected = [
            r#"{"event":"started","provider":"codex","home":"/data/codex/1"}"#,
            r#"{"event":"url","url":"https://auth.example/"}"#,
            r#"{"event":"output","line":"Paste the code"}"#,
            r#"{"event":"done","account_id":"codex:abc","label":null}"#,
            r#"{"event":"error","message":"boom"}"#,
        ];
        assert_eq!(text, expected.join("\n") + "\n");
    }

    #[test]
    fn finish_reports_done_or_error() {
        let mut out = JsonLines::new(Vec::new());
        let ok = out.finish(Ok("codex:a".to_owned()), |id| ProgressEvent::Done {
            account_id: id.clone(),
            label: Some("Work".into()),
        });
        assert_eq!(ok.unwrap(), "codex:a");
        let failed: Result<String> =
            out.finish(Err(anyhow::anyhow!("no luck")), |_| ProgressEvent::Error {
                message: String::new(),
            });
        assert!(failed.is_err());
        let text = String::from_utf8(out.out).unwrap();
        assert_eq!(
            text,
            "{\"event\":\"done\",\"account_id\":\"codex:a\",\"label\":\"Work\"}\n\
             {\"event\":\"error\",\"message\":\"no luck\"}\n"
        );
    }
}
