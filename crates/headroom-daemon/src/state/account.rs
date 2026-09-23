use headroom_core::pace::{Pace, Severity, pace, tone};
use headroom_core::quota::{Balance, BalanceAmount, LimitsSnapshot, Notice, QuotaWindow};
use jiff::Timestamp;

use super::AssembleContext;
use super::payload::{
    AccountView, BalanceAmountView, BalanceView, NoticeView, PaceView, WindowView,
};
use super::status::{error_view, source, status};
use crate::model::{Model, window_key};
use crate::storage::accounts::AccountRecord;

#[must_use]
pub fn account_view(
    record: &AccountRecord,
    model: &Model,
    ctx: &AssembleContext<'_>,
) -> AccountView {
    let entry = model.snapshots.get(record.id());
    let runtime = model.runtime.get(record.id());
    let snapshot = entry.map(|e| &e.snapshot);
    AccountView {
        id: record.id().0.clone(),
        provider: record.reference.provider,
        label: record.label.clone(),
        email: identity_field(
            snapshot,
            |s| s.identity.email.clone(),
            record.email.as_deref(),
        ),
        plan: identity_field(
            snapshot,
            |s| s.identity.plan.clone(),
            record.plan.as_deref(),
        ),
        hidden: record.hidden,
        status: status(runtime, entry, ctx.now),
        error: runtime.and_then(|r| r.failure.as_ref()).map(error_view),
        updated_at: entry.map(crate::model::SnapshotEntry::data_time),
        source: entry.map(source),
        windows: snapshot.map_or_else(Vec::new, |s| windows(s, ctx.now)),
        balances: snapshot.map_or_else(Vec::new, |s| s.balances.iter().map(balance_view).collect()),
        notices: snapshot.map_or_else(Vec::new, |s| s.notices.iter().map(notice_view).collect()),
        usage_home: ctx.homes.show(&record.reference.home),
    }
}

fn identity_field(
    snapshot: Option<&LimitsSnapshot>,
    field: impl Fn(&LimitsSnapshot) -> Option<String>,
    stored: Option<&str>,
) -> Option<String> {
    snapshot
        .and_then(field)
        .or_else(|| stored.map(str::to_owned))
}

fn windows(snapshot: &LimitsSnapshot, now: Timestamp) -> Vec<WindowView> {
    snapshot
        .windows
        .iter()
        .map(|w| window_view(w, now))
        .collect()
}

#[must_use]
pub fn window_view(window: &QuotaWindow, now: Timestamp) -> WindowView {
    let pace = pace(window, now);
    WindowView {
        id: window_key(&window.id),
        label: window.label.clone(),
        used_percent: window.used.value(),
        remaining_percent: window.used.remaining().value(),
        resets_at: window.resets_at,
        period_seconds: window.period.map(|p| p.as_secs()),
        tone: tone(window, &pace),
        pace: pace_view(&pace),
    }
}

fn pace_view(pace: &Pace) -> PaceView {
    PaceView {
        severity: pace.severity,
        even_pace_percent: pace.even_pace.map(f64::from),
        projected_percent: pace.projected.map(f64::from),
        spare_percent: spare_percent(pace),
        runs_out_at: pace.runs_out_at,
    }
}

fn spare_percent(pace: &Pace) -> Option<f64> {
    let on_track = matches!(pace.severity, Severity::Healthy | Severity::Close);
    pace.projected
        .filter(|_| on_track)
        .map(|projected| projected.remaining().value())
}

fn balance_view(balance: &Balance) -> BalanceView {
    let amount = match &balance.amount {
        BalanceAmount::Usd(micros) => BalanceAmountView::Usd {
            usd_micros: micros.0,
        },
        BalanceAmount::Count { value, unit } => BalanceAmountView::Count {
            value: *value,
            unit: unit.clone(),
        },
    };
    BalanceView {
        id: balance.id.clone(),
        label: balance.label.clone(),
        amount,
    }
}

fn notice_view(notice: &Notice) -> NoticeView {
    NoticeView {
        tone: notice.tone,
        text: notice.text.clone(),
    }
}
