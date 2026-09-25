use crate::payload::{Display, PanelLimit};

pub const MAX_PANEL_LIMITS: usize = 3;

fn toggled<T: Clone + PartialEq>(items: &[T], item: &T, on: bool, cap: usize) -> Vec<T> {
    if on && items.contains(item) {
        return items.to_vec();
    }
    let mut list: Vec<T> = items
        .iter()
        .filter(|known| *known != item)
        .cloned()
        .collect();
    if on && list.len() < cap {
        list.push(item.clone());
    }
    list
}

impl Display {
    #[must_use]
    pub fn is_window_hidden(&self, account_id: &str, window_id: &str) -> bool {
        self.hidden_windows
            .get(account_id)
            .is_some_and(|windows| windows.iter().any(|id| id == window_id))
    }

    #[must_use]
    pub fn hidden_windows_after(
        &self,
        account_id: &str,
        window_id: &str,
        hidden: bool,
    ) -> Vec<String> {
        let mut windows: Vec<String> = self
            .hidden_windows
            .get(account_id)
            .map(|windows| {
                windows
                    .iter()
                    .filter(|id| *id != window_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if hidden {
            windows.push(window_id.to_owned());
        }
        windows
    }

    #[must_use]
    pub fn starred_after(&self, account_id: &str, starred: bool) -> Vec<String> {
        toggled(
            &self.starred_accounts,
            &account_id.to_owned(),
            starred,
            usize::MAX,
        )
    }

    #[must_use]
    pub fn has_panel_limit(&self, limit: &PanelLimit) -> bool {
        self.panel_limits.contains(limit)
    }

    #[must_use]
    pub fn panel_limits_after(&self, limit: &PanelLimit, shown: bool) -> Vec<PanelLimit> {
        toggled(&self.panel_limits, limit, shown, MAX_PANEL_LIMITS)
    }

    #[must_use]
    pub fn can_add_panel_limit(&self) -> bool {
        self.panel_limits.len() < MAX_PANEL_LIMITS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit(account: &str, window: &str) -> PanelLimit {
        PanelLimit {
            account_id: account.into(),
            window: window.into(),
        }
    }

    #[test]
    fn starring_adds_once_and_unstarring_removes() {
        let display = Display {
            starred_accounts: vec!["a".into(), "b".into()],
            ..Display::default()
        };
        assert!(display.is_starred("a"));
        assert_eq!(display.starred_after("c", true), ["a", "b", "c"]);
        assert_eq!(display.starred_after("a", true), ["a", "b"]);
        assert_eq!(display.starred_after("a", false), ["b"]);
    }

    #[test]
    fn panel_limits_stop_at_three() {
        let display = Display {
            panel_limits: vec![limit("a", "session"), limit("b", "weekly")],
            ..Display::default()
        };
        assert!(display.can_add_panel_limit());
        let three = display.panel_limits_after(&limit("c", "session"), true);
        assert_eq!(three.len(), 3);
        let full = Display {
            panel_limits: three,
            ..Display::default()
        };
        assert!(!full.can_add_panel_limit());
        assert_eq!(
            full.panel_limits_after(&limit("d", "session"), true).len(),
            3
        );
        assert_eq!(
            full.panel_limits_after(&limit("a", "session"), false),
            [limit("b", "weekly"), limit("c", "session")]
        );
        assert!(full.has_panel_limit(&limit("b", "weekly")));
    }
}
