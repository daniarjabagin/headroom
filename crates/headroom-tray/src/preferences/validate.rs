use super::change::Change;
use super::display::MAX_PANEL_LIMITS;
use super::notify::{PROVIDER_THRESHOLD_RANGE, THRESHOLD_RANGE};
use crate::payload::PanelLimit;

pub const MAX_SHORTCUT_CHARS: usize = 64;

fn blank(text: &str) -> bool {
    text.trim().is_empty()
}

fn valid_limits(limits: &[PanelLimit]) -> bool {
    let mut distinct: Vec<&PanelLimit> = Vec::new();
    for limit in limits {
        if blank(&limit.account_id) || blank(&limit.window) {
            return false;
        }
        if !distinct.contains(&limit) {
            distinct.push(limit);
        }
    }
    distinct.len() <= MAX_PANEL_LIMITS
}

#[must_use]
pub fn is_accelerator(text: &str) -> bool {
    let mut rest = text;
    while let Some(tail) = rest.strip_prefix('<') {
        let Some((modifier, after)) = tail.split_once('>') else {
            return false;
        };
        if modifier.is_empty() || !modifier.chars().all(|c| c.is_ascii_alphabetic()) {
            return false;
        }
        rest = after;
    }
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[must_use]
pub fn is_valid_shortcut(text: &str) -> bool {
    text.chars().count() <= MAX_SHORTCUT_CHARS && (text.is_empty() || is_accelerator(text))
}

impl Change {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        match self {
            Change::ThresholdPercent(percent) => THRESHOLD_RANGE.contains(percent),
            Change::ProviderThreshold {
                provider,
                threshold,
            } => {
                !blank(provider)
                    && threshold.is_none_or(|percent| PROVIDER_THRESHOLD_RANGE.contains(&percent))
            }
            Change::QuietHours(quiet) => quiet.is_valid(),
            Change::Shortcut(text) => is_valid_shortcut(text),
            Change::PanelLimits(limits) => valid_limits(limits),
            Change::StarredAccounts(ids) => !ids.iter().any(|id| blank(id)),
            Change::HiddenWindows {
                account_id,
                windows,
            } => !blank(account_id) && !windows.iter().any(|id| blank(id)),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::model::{ClockTime, QuietHours};

    fn limit(account: &str, window: &str) -> PanelLimit {
        PanelLimit {
            account_id: account.into(),
            window: window.into(),
        }
    }

    #[test]
    fn shortcuts_follow_gtk_accelerator_syntax() {
        for good in ["", "<Super>u", "<Control><Alt>h", "F12", "<Shift>Page_Up"] {
            assert!(is_valid_shortcut(good), "{good}");
        }
        for bad in ["<Super>", "<>u", "<Super", "<Su per>u", "Ctrl+U", "<1>u"] {
            assert!(!is_valid_shortcut(bad), "{bad}");
        }
        assert!(!is_valid_shortcut(&format!("<Super>{}", "a".repeat(60))));
    }

    #[test]
    fn thresholds_mirror_the_daemon_ranges() {
        assert!(Change::ThresholdPercent(1).is_valid());
        assert!(Change::ThresholdPercent(50).is_valid());
        assert!(!Change::ThresholdPercent(0).is_valid());
        assert!(!Change::ThresholdPercent(51).is_valid());
        let provider = |name: &str, threshold| Change::ProviderThreshold {
            provider: name.into(),
            threshold,
        };
        assert!(provider("copilot", Some(0)).is_valid());
        assert!(provider("copilot", None).is_valid());
        assert!(!provider("copilot", Some(51)).is_valid());
        assert!(!provider(" ", Some(10)).is_valid());
    }

    #[test]
    fn lists_and_quiet_hours_are_checked() {
        let three = vec![
            limit("a", "session"),
            limit("b", "weekly"),
            limit("c", "session"),
        ];
        assert!(Change::PanelLimits(three.clone()).is_valid());
        let mut with_duplicate = three.clone();
        with_duplicate.push(limit("a", "session"));
        assert!(Change::PanelLimits(with_duplicate).is_valid());
        let mut four = three;
        four.push(limit("d", "weekly"));
        assert!(!Change::PanelLimits(four).is_valid());
        assert!(!Change::PanelLimits(vec![limit("a", "")]).is_valid());
        assert!(!Change::StarredAccounts(vec![String::new()]).is_valid());
        let empty = QuietHours {
            enabled: true,
            from: ClockTime::DEFAULT_TO,
            ..QuietHours::default()
        };
        assert!(!Change::QuietHours(empty).is_valid());
        assert!(Change::QuietHours(QuietHours::default()).is_valid());
    }
}
