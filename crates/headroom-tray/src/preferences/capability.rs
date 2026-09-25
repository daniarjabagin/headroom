use super::change::Change;
use crate::payload::{PanelLabel, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capabilities {
    pub release_0_6: bool,
}

impl Capabilities {
    #[must_use]
    pub fn of(state: Option<&State>) -> Self {
        Self {
            release_0_6: state.is_some_and(State::speaks_0_6),
        }
    }

    #[must_use]
    pub fn permits(self, change: &Change) -> bool {
        self.release_0_6 || !change.requires_0_6()
    }

    #[must_use]
    pub fn has_0_6_methods(self) -> bool {
        self.release_0_6
    }
}

impl Change {
    #[must_use]
    pub fn requires_0_6(&self) -> bool {
        match self {
            Change::Theme(_)
            | Change::Language(_)
            | Change::ValueMode(_)
            | Change::ResetFormat(_)
            | Change::Section(..)
            | Change::CombineAccounts(_)
            | Change::ReducedMotion(_)
            | Change::RefreshInterval(_)
            | Change::Headline(_)
            | Change::CheckUpdates(_)
            | Change::Notify(..)
            | Change::HiddenWindows { .. } => false,
            Change::PanelLabel(label) => *label == PanelLabel::None,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::{Density, parse_state};

    const FULL: &str = include_str!("../../../headroom-daemon/src/state/snapshots/state_full.json");

    fn without_panel_items() -> State {
        let mut raw: serde_json::Value = serde_json::from_str(FULL).unwrap();
        raw.as_object_mut().unwrap().remove("panel_items");
        parse_state(&raw.to_string()).unwrap()
    }

    #[test]
    fn a_daemon_with_panel_items_accepts_new_keys() {
        let state = parse_state(FULL).unwrap();
        let capable = Capabilities::of(Some(&state));
        assert!(capable.release_0_6);
        assert!(capable.permits(&Change::Density(Density::Compact)));
        assert!(capable.has_0_6_methods());
    }

    #[test]
    fn older_daemons_only_get_old_keys() {
        let old = Capabilities::of(Some(&without_panel_items()));
        assert!(!old.release_0_6);
        assert!(!old.permits(&Change::Density(Density::Compact)));
        assert!(!old.permits(&Change::PanelLabel(PanelLabel::None)));
        assert!(old.permits(&Change::PanelLabel(PanelLabel::Window)));
        assert!(old.permits(&Change::ReducedMotion(true)));
        assert!(!old.has_0_6_methods());
        assert_eq!(Capabilities::of(None), Capabilities::default());
    }
}
