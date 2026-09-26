use headroom_core::pace::{Pace, Severity, pace, tone};
use headroom_core::quota::{Balance, BalanceAmount, LimitsSnapshot, Notice, QuotaWindow};
use jiff::Timestamp;

use super::AssembleContext;
use super::account_collapse::account_collapsed;
use super::account_recovery::recovery;
use super::payload::{
    AccountView, BalanceAmountView, BalanceView, NoticeView, PaceView, WindowView,
};
use super::refresh::refresh_view;
use super::status::{error_view, source, status};
use crate::model::{Model, RefreshFailure, window_key};
use crate::storage::accounts::AccountRecord;

#[must_use]
pub fn account_view(
    record: &AccountRecord,
    model: &Model,
    ctx: &AssembleContext<'_>,
) -> AccountView {
    let view = base_view(record, model, ctx);
    AccountView {
        collapsed: account_collapsed(&view, &model.settings.display),
        ..view
    }
}

fn base_view(record: &AccountRecord, model: &Model, ctx: &AssembleContext<'_>) -> AccountView {
    let runtime = model.runtime.get(record.id());
    let failure = runtime.and_then(|r| r.failure.as_ref());
    let lapsed = failure.is_some_and(RefreshFailure::is_no_subscription);
    let entry = model.snapshots.get(record.id()).filter(|_| !lapsed);
    let snapshot = entry.map(|e| &e.snapshot);
    AccountView {
        id: record.id().0.clone(),
        provider: record.reference.provider.clone(),
        provider_name: ctx
            .catalog
            .display_name(&record.reference.provider)
            .to_owned(),
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
        )
        .filter(|_| !lapsed),
        hidden: record.hidden,
        owner: record.reference.owner,
        status: status(runtime, entry, ctx.now),
        error: failure.map(error_view),
        recovery: recovery(failure, &record.reference, ctx.catalog),
        updated_at: entry.map(crate::model::SnapshotEntry::data_time),
        source: entry.map(source),
        windows: snapshot.map_or_else(Vec::new, |s| windows(s, record, model, ctx.now)),
        balances: snapshot.map_or_else(Vec::new, |s| s.balances.iter().map(balance_view).collect()),
        notices: snapshot.map_or_else(Vec::new, |s| s.notices.iter().map(notice_view).collect()),
        usage_home: ctx.homes.show(&model.usage_home_of(&record.reference)),
        refresh: Some(refresh_view(record, runtime, model, ctx.now)),
        collapsed: false,
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

fn windows(
    snapshot: &LimitsSnapshot,
    record: &AccountRecord,
    model: &Model,
    now: Timestamp,
) -> Vec<WindowView> {
    let display = &model.settings.display;
    snapshot
        .windows
        .iter()
        .map(|w| {
            let hidden = display.is_hidden(&record.id().0, &window_key(&w.id));
            window_view(w, now, hidden)
        })
        .collect()
}

fn window_view(window: &QuotaWindow, now: Timestamp, hidden: bool) -> WindowView {
    let pace = pace(window, now);
    WindowView {
        id: window_key(&window.id),
        label: window.label.clone(),
        used_percent: window.used.value(),
        remaining_percent: window.used.remaining().value(),
        resets_at: window.resets_at,
        period_seconds: window.period.map(|p| p.as_secs()),
        tone: tone(window, &pace, now),
        pace: pace_view(&pace),
        hidden,
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
        BalanceAmount::Money(money) => BalanceAmountView::Money {
            currency: money.currency.to_string(),
            micros: money.micros,
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

#[cfg(test)]
mod tests {
    use headroom_core::units::{CurrencyCode, MicroUsd, Money};

    use super::*;

    fn view_json(amount: BalanceAmount) -> serde_json::Value {
        let balance = Balance {
            id: "total".into(),
            label: "Balance".into(),
            amount,
        };
        serde_json::to_value(balance_view(&balance)).unwrap()
    }

    #[test]
    fn money_balances_carry_their_currency_and_micros() {
        let amount = BalanceAmount::Money(Money {
            currency: CurrencyCode::parse("CNY").unwrap(),
            micros: 12_500_000,
        });
        assert_eq!(
            view_json(amount),
            serde_json::json!({
                "id": "total",
                "label": "Balance",
                "kind": "money",
                "currency": "CNY",
                "micros": 12_500_000
            })
        );
    }

    #[test]
    fn usd_balances_keep_their_existing_shape() {
        assert_eq!(
            view_json(BalanceAmount::Usd(MicroUsd(-1))),
            serde_json::json!({ "id": "total", "label": "Balance", "kind": "usd", "usd_micros": -1 })
        );
    }
}
