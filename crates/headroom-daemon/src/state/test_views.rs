use headroom_core::account::{CredentialOwner, ProviderId};
use headroom_core::pace::{Severity, Tone};

use super::payload::{AccountStatus, AccountView, PaceView, WindowView};
use crate::testing::ts;

pub fn account(provider: &ProviderId, name: &str, windows: Vec<WindowView>) -> AccountView {
    AccountView {
        id: format!("{provider}:{name}"),
        provider: provider.clone(),
        provider_name: provider_name(provider),
        label: Some(name.into()),
        email: None,
        plan: Some("Pro".into()),
        hidden: false,
        owner: CredentialOwner::Cli,
        status: AccountStatus::Fresh,
        error: None,
        recovery: None,
        updated_at: None,
        source: None,
        windows,
        balances: Vec::new(),
        notices: Vec::new(),
        usage_home: format!("~/.{provider}"),
        refresh: None,
        collapsed: false,
    }
}

fn provider_name(provider: &ProviderId) -> String {
    let mut name = provider.to_string();
    if let Some(first) = name.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    name
}

pub fn window(id: &str, used: f64, resets_at: &str) -> WindowView {
    WindowView {
        id: id.into(),
        label: label(id),
        used_percent: used,
        remaining_percent: (100.0 - used).max(0.0),
        resets_at: Some(ts(resets_at)),
        period_seconds: None,
        tone: Tone::Good,
        pace: untracked(),
        hidden: false,
    }
}

pub fn tracked(mut window: WindowView, severity: Severity, projected: f64) -> WindowView {
    window.pace = PaceView {
        severity,
        even_pace_percent: Some(50.0),
        projected_percent: Some(projected),
        spare_percent: None,
        runs_out_at: None,
    };
    window
}

pub fn untracked() -> PaceView {
    PaceView {
        severity: Severity::Untracked,
        even_pace_percent: None,
        projected_percent: None,
        spare_percent: None,
        runs_out_at: None,
    }
}

fn label(id: &str) -> String {
    let mut label = id.to_owned();
    if let Some(first) = label.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    label
}
