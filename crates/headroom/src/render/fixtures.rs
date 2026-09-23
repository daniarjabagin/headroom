use headroom_daemon::state::payload::StatePayload;

pub fn full_state() -> StatePayload {
    serde_json::from_str(include_str!("fixtures/state_full.json")).unwrap()
}
