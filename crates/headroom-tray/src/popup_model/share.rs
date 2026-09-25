use jiff::Timestamp;
use jiff::civil::Date;

use crate::account::{shown_plan, shown_windows};
use crate::combined::{CombinedGroup, CombinedWindow, SegmentFill, combined_row};
use crate::dates::Locale;
use crate::format::{reading_percent, reset_phrase, round_percent, window_label};
use crate::i18n::{Lang, fill};
use crate::payload::{Account, Display, Headline, ResetFormat, ValueMode, Window};
use crate::quota::quota_view;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ShareRow {
    pub label: String,
    pub headline: String,
    pub trailing: String,
    pub segments: Vec<SegmentFill>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShareCard {
    pub provider: String,
    pub provider_name: String,
    pub meta: Option<String>,
    pub stamp: String,
    pub date: String,
    pub hero: String,
    pub hero_unit: String,
    pub sub: String,
    pub bars: Vec<f64>,
    pub rows: Vec<ShareRow>,
}

#[derive(Debug, Clone, Copy)]
pub struct ShareInput<'a> {
    pub locale: &'a Locale,
    pub display: &'a Display,
    pub headline: Option<&'a Headline>,
    pub now: Timestamp,
}

fn date_text(lang: Lang, date: Date) -> String {
    let month = usize::try_from(date.month() - 1).unwrap_or(0);
    match lang {
        Lang::En => format!("{} {} {}", date.day(), MONTHS[month], date.year()),
        Lang::Ru => format!("{:02}.{:02}.{}", date.day(), date.month(), date.year()),
    }
}

fn stamp(lang: Lang, provider_name: &str, date: &str) -> String {
    fill(
        lang.tr("{provider} limits · {date}"),
        &[("provider", provider_name), ("date", date)],
    )
    .to_uppercase()
}

fn hero_unit(lang: Lang, mode: ValueMode) -> String {
    lang.tr(match mode {
        ValueMode::Left => "left",
        ValueMode::Used => "used",
    })
    .to_owned()
}

fn reset_part(input: ShareInput, resets_at: Option<Timestamp>) -> Option<String> {
    let reset = resets_at?;
    Some(reset_phrase(
        input.locale,
        reset,
        input.now,
        ResetFormat::Countdown,
        false,
    ))
}

fn joined(parts: impl IntoIterator<Item = Option<String>>) -> String {
    parts.into_iter().flatten().collect::<Vec<_>>().join(" · ")
}

fn base_card(input: ShareInput, provider: &str, provider_name: &str) -> ShareCard {
    let lang = input.locale.lang;
    let date = date_text(lang, input.now.to_zoned(input.locale.tz.clone()).date());
    ShareCard {
        provider: provider.to_owned(),
        provider_name: provider_name.to_owned(),
        meta: None,
        stamp: stamp(lang, provider_name, &date),
        date,
        hero: String::new(),
        hero_unit: hero_unit(lang, input.display.value_mode),
        sub: String::new(),
        bars: Vec::new(),
        rows: Vec::new(),
    }
}

fn account_hero<'a>(
    input: ShareInput,
    account: &Account,
    windows: &[&'a Window],
) -> Option<&'a Window> {
    let pinned = input
        .headline
        .filter(|headline| headline.account_id.as_deref() == Some(account.id.as_str()))
        .and_then(|headline| windows.iter().find(|window| window.id == headline.window));
    pinned.copied().or_else(|| {
        windows
            .iter()
            .min_by(|a, b| a.remaining_percent.total_cmp(&b.remaining_percent))
            .copied()
    })
}

#[must_use]
pub fn account_card(input: ShareInput, account: &Account) -> Option<ShareCard> {
    let windows = shown_windows(account);
    let hero = account_hero(input, account, &windows)?;
    let lang = input.locale.lang;
    let percent = reading_percent(hero, input.display.value_mode);
    let rows = windows
        .iter()
        .map(|window| {
            let view = quota_view(input.locale, window, input.display, input.now);
            ShareRow {
                label: view.label,
                headline: view.headline,
                trailing: view.trailing,
                segments: vec![SegmentFill {
                    fraction: view.fill,
                    tone: view.tone,
                    tick: view.tick,
                }],
            }
        })
        .collect();
    let window = window_label(lang, &hero.id, &hero.label).to_lowercase();
    Some(ShareCard {
        meta: shown_plan(account).map(str::to_owned),
        hero: format!("{}%", round_percent(percent)),
        sub: joined([Some(window), reset_part(input, hero.resets_at)]),
        bars: vec![(percent / 100.0).clamp(0.0, 1.0)],
        rows,
        ..base_card(input, &account.provider, &account.provider_name)
    })
}

fn group_hero<'a>(input: ShareInput, group: &'a CombinedGroup) -> Option<&'a CombinedWindow> {
    let pinned = input
        .headline
        .filter(|headline| headline.combined && headline.provider == group.provider)
        .and_then(|headline| group.windows.iter().find(|w| w.id == headline.window));
    let share =
        |window: &CombinedWindow| window.remaining_percent / window.capacity_percent.max(1.0);
    pinned.or_else(|| {
        group
            .windows
            .iter()
            .min_by(|a, b| share(a).total_cmp(&share(b)))
    })
}

fn group_meta(lang: Lang, group: &CombinedGroup) -> String {
    let count = u64::try_from(group.accounts.len()).unwrap_or(u64::MAX);
    let accounts = fill(
        lang.tr_plural(["{count} account", "{count} accounts"], count),
        &[("count", &count.to_string())],
    );
    let plans: Vec<&str> = group
        .accounts
        .iter()
        .filter_map(|account| account.plan.as_deref())
        .collect();
    joined([
        Some(accounts),
        (!plans.is_empty()).then(|| plans.join(" + ")),
    ])
}

fn group_row(input: ShareInput, window: &CombinedWindow, members: &[Account]) -> ShareRow {
    let row = combined_row(input.locale, window, members, input.display, input.now);
    ShareRow {
        label: row.label,
        headline: row.headline,
        trailing: row.trailing,
        segments: row.segments,
    }
}

fn capacity_text(lang: Lang, hero: &CombinedWindow) -> String {
    let window = window_label(lang, &hero.id, &hero.label).to_lowercase();
    let capacity = round_percent(hero.capacity_percent).to_string();
    fill(
        lang.tr("of {capacity}% {window}"),
        &[("capacity", &capacity), ("window", &window)],
    )
}

#[must_use]
pub fn group_card(
    input: ShareInput,
    group: &CombinedGroup,
    members: &[Account],
) -> Option<ShareCard> {
    let hero = group_hero(input, group)?;
    let lang = input.locale.lang;
    let percent = match input.display.value_mode {
        ValueMode::Left => hero.remaining_percent,
        ValueMode::Used => hero.used_percent,
    };
    let bars = group_row(input, hero, members)
        .segments
        .iter()
        .map(|segment| segment.fraction)
        .collect();
    Some(ShareCard {
        meta: Some(group_meta(lang, group)),
        hero: format!("{}%", round_percent(percent)),
        sub: joined([
            Some(capacity_text(lang, hero)),
            reset_part(input, hero.resets_at),
        ]),
        bars,
        rows: group
            .windows
            .iter()
            .map(|window| group_row(input, window, members))
            .collect(),
        ..base_card(input, &group.provider, &group.provider_name)
    })
}

#[must_use]
pub fn summary_text(lang: Lang, card: &ShareCard) -> String {
    let title = fill(
        lang.tr("{provider} limits · {date}"),
        &[("provider", &card.provider_name), ("date", &card.date)],
    );
    let heading = joined([Some(card.provider_name.clone()), card.meta.clone()]);
    let rows = card
        .rows
        .iter()
        .map(|row| format!("{}: {} · {}", row.label, row.headline, row.trailing));
    std::iter::once(title)
        .chain(std::iter::once(heading))
        .chain(rows)
        .chain(std::iter::once(format!(
            "Headroom · {}",
            lang.tr("Know what's left.")
        )))
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn file_name(provider: &str, locale: &Locale, now: Timestamp) -> String {
    let local = now.to_zoned(locale.tz.clone());
    let safe: String = provider
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!(
        "headroom-{safe}-{:04}-{:02}-{:02}-{:02}{:02}.png",
        local.year(),
        local.month(),
        local.day(),
        local.hour(),
        local.minute()
    )
}

#[cfg(test)]
#[path = "share_tests.rs"]
mod tests;
