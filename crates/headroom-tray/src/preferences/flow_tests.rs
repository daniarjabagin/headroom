use super::*;

fn running() -> Flow {
    let mut flow = Flow::default();
    flow.start();
    flow
}

#[test]
fn parses_progress_events_and_ignores_extra_fields() {
    assert_eq!(
        parse_add_event(r#"{"event":"started","provider":"codex","home":"/x"}"#),
        Some(AddEvent::Started)
    );
    assert_eq!(
        parse_add_event(r#"{"event":"done","account_id":"codex:1","label":null}"#),
        Some(AddEvent::Done)
    );
    assert_eq!(
        parse_add_event(r#"{"event":"url","url":"https://auth.example/x"}"#),
        Some(AddEvent::Url {
            url: "https://auth.example/x".into()
        })
    );
    assert_eq!(parse_add_event(r#"{"event":"mystery"}"#), None);
    assert_eq!(parse_add_event("plain text"), None);
}

#[test]
fn a_sign_in_runs_to_done() {
    let mut flow = running();
    flow.event(AddEvent::Output {
        line: "Starting local login server".into(),
    });
    flow.event(AddEvent::Url {
        url: "https://auth.example/a".into(),
    });
    flow.event(AddEvent::Url {
        url: "https://auth.example/b".into(),
    });
    assert_eq!(flow.url.as_deref(), Some("https://auth.example/a"));
    assert_eq!(flow.log.len(), 3);
    flow.event(AddEvent::Done);
    assert_eq!(flow.phase, Phase::Done);
    flow.exited(Lang::En, Some("ignored".into()));
    assert_eq!(flow.phase, Phase::Done);
}

#[test]
fn errors_and_early_exits_fail_the_flow() {
    let mut flow = running();
    flow.event(AddEvent::Error {
        message: "cancelled".into(),
    });
    assert_eq!(flow.phase, Phase::Failed("cancelled".into()));
    let mut flow = running();
    flow.exited(Lang::En, Some(exit_message(Lang::En, 3)));
    assert_eq!(
        flow.phase,
        Phase::Failed("headroom exited with status 3".into())
    );
    let mut flow = running();
    flow.exited(Lang::En, Some(String::new()));
    assert_eq!(
        flow.phase,
        Phase::Failed("The sign-in stopped before it finished".into())
    );
    let mut flow = running();
    flow.exited(Lang::En, None);
    assert_eq!(flow.phase, Phase::Done);
}

#[test]
fn events_after_the_end_are_ignored() {
    let mut flow = Flow::default();
    flow.event(AddEvent::Done);
    assert_eq!(flow.phase, Phase::Form);
    let mut flow = running();
    flow.fail("boom".into());
    flow.event(AddEvent::Done);
    assert_eq!(flow.phase, Phase::Failed("boom".into()));
    flow.reset();
    assert_eq!(flow, Flow::default());
}

#[test]
fn finds_device_codes() {
    assert_eq!(
        device_code("Enter this one-time code: ABCD-1234"),
        Some("ABCD-1234".into())
    );
    assert_eq!(
        device_code("! First copy your one-time code: 1A2B-3C4D."),
        Some("1A2B-3C4D".into())
    );
    assert_eq!(device_code("Visit ABCD-1234 now"), None);
    assert_eq!(device_code("your code is abcd-efgh"), None);
    assert_eq!(device_code("code: AB-CD"), None);
    let mut flow = running();
    flow.event(AddEvent::Url {
        url: "https://auth.example/device?user_code=WXYZ-1234".into(),
    });
    assert_eq!(flow.code.as_deref(), Some("WXYZ-1234"));
    let mut flow = running();
    flow.event(AddEvent::Output {
        line: "Enter code FFFF-0000 at https://example.com/device".into(),
    });
    assert_eq!(flow.code.as_deref(), Some("FFFF-0000"));
}

#[test]
fn the_log_keeps_the_latest_lines() {
    let mut flow = running();
    for index in 0..(LOG_LIMIT + 5) {
        flow.event(AddEvent::Output {
            line: index.to_string(),
        });
    }
    assert_eq!(flow.log.len(), LOG_LIMIT);
    assert_eq!(flow.log[0], "5");
}
