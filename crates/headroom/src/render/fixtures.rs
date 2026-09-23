use headroom_daemon::state::payload::StatePayload;

const STATE_FULL: &str = include_str!("fixtures/state_full.json");

pub fn full_state() -> StatePayload {
    serde_json::from_str(STATE_FULL).unwrap()
}

#[test]
fn the_fixture_is_the_daemon_snapshot() {
    let daemon = include_str!("../../../headroom-daemon/src/state/snapshots/state_full.json");
    assert_eq!(STATE_FULL, daemon);
}
