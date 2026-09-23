use super::*;

fn reading(slot: Slot, used: f64, period_secs: Option<i64>) -> WindowReading {
    WindowReading {
        slot,
        used: Percent::new(used),
        period: period_secs.map(SignedDuration::from_secs),
        resets_at: None,
    }
}

fn ids(windows: &[QuotaWindow]) -> Vec<(WindowId, String)> {
    windows
        .iter()
        .map(|window| (window.id.clone(), window.label.clone()))
        .collect()
}

#[test]
fn classifies_by_length_not_slot() {
    let windows = classify(
        Family::Main,
        vec![
            reading(Slot::Primary, 10.0, Some(604_800)),
            reading(Slot::Secondary, 20.0, Some(18_000)),
        ],
    );
    assert_eq!(
        ids(&windows),
        [
            (WindowId::Weekly, "Weekly".to_owned()),
            (WindowId::Session, "Session".to_owned()),
        ]
    );
}

#[test]
fn unknown_length_falls_back_to_slot_and_assumes_its_period() {
    let windows = classify(
        Family::Main,
        vec![
            reading(Slot::Primary, 10.0, None),
            reading(Slot::Secondary, 20.0, None),
        ],
    );
    assert_eq!(windows[0].id, WindowId::Session);
    assert_eq!(windows[0].period, Some(WindowId::SESSION_PERIOD));
    assert_eq!(windows[1].id, WindowId::Weekly);
    assert_eq!(windows[1].period, Some(WindowId::WEEKLY_PERIOD));
}

#[test]
fn slot_fallback_never_steals_an_exact_window() {
    let windows = classify(
        Family::Main,
        vec![
            reading(Slot::Primary, 10.0, Some(604_800)),
            reading(Slot::Secondary, 20.0, None),
        ],
    );
    assert_eq!(
        ids(&windows),
        [
            (WindowId::Weekly, "Weekly".to_owned()),
            (WindowId::Other("secondary".into()), "Secondary".to_owned()),
        ]
    );
    assert_eq!(windows[1].period, None);
}

#[test]
fn unusual_lengths_are_kept_as_other() {
    let windows = classify(
        Family::Main,
        vec![reading(Slot::Primary, 5.0, Some(86_400))],
    );
    assert_eq!(
        ids(&windows),
        [(WindowId::Other("1d".into()), "1d".to_owned())]
    );
    assert_eq!(windows[0].period, Some(SignedDuration::from_hours(24)));
}

#[test]
fn model_windows_are_distinct_per_length() {
    let windows = classify(
        Family::Model("GPT-5.3-Codex-Spark"),
        vec![
            reading(Slot::Primary, 1.0, Some(18_000)),
            reading(Slot::Secondary, 2.0, Some(604_800)),
        ],
    );
    assert_eq!(
        ids(&windows),
        [
            (
                WindowId::Model("GPT-5.3-Codex-Spark".into()),
                "Spark".to_owned()
            ),
            (
                WindowId::Model("GPT-5.3-Codex-Spark:weekly".into()),
                "Spark Weekly".to_owned()
            ),
        ]
    );
}
