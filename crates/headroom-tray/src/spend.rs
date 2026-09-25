use crate::i18n::{Lang, fill};
use crate::numbers::{compact_tokens, exact_tokens_text, exact_usd, usd};
use crate::payload::{Daily, ModelUsage, OtherModels, PeriodSpend, Spend, Totals, Usage};

const TREND_DAYS: usize = 30;
pub const TREND_HEIGHT: u64 = 18;
const TREND_STUB: u64 = 2;
const TREND_MIN_PERCENT: u64 = 18;
const PARTIAL_MARK: &str = "*";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Period {
    #[default]
    Today,
    Yesterday,
    Last30Days,
}

impl Period {
    pub const ALL: [Period; 3] = [Period::Today, Period::Yesterday, Period::Last30Days];

    #[must_use]
    pub fn segment_title(self, lang: Lang) -> &'static str {
        match self {
            Period::Today => lang.tr("Today"),
            Period::Yesterday => lang.tr("Yesterday"),
            Period::Last30Days => lang.tr("30 Days"),
        }
    }

    #[must_use]
    pub fn row_title(self, lang: Lang) -> &'static str {
        match self {
            Period::Last30Days => lang.tr("Last 30 Days"),
            other => other.segment_title(lang),
        }
    }

    #[must_use]
    pub fn of(self, spend: &Spend) -> &PeriodSpend {
        match self {
            Period::Today => &spend.today,
            Period::Yesterday => &spend.yesterday,
            Period::Last30Days => &spend.last_30_days,
        }
    }

    #[must_use]
    pub fn totals(self, usage: &Usage) -> &Totals {
        match self {
            Period::Today => &usage.today,
            Period::Yesterday => &usage.yesterday,
            Period::Last30Days => &usage.last_30_days,
        }
    }
}

#[must_use]
pub fn shows_token_line(period: &PeriodSpend) -> bool {
    period.by_provider.len() == 1
}

#[must_use]
pub fn info_text(lang: Lang, period: &PeriodSpend) -> String {
    let base = lang.tr("Estimated from local logs and public pricing.");
    if period.partial {
        format!(
            "{base} {}",
            lang.tr("Some models have no public price yet.")
        )
    } else {
        base.to_owned()
    }
}

fn cost_text(lang: Lang, cost_micros: i64, partial: bool) -> String {
    if partial && cost_micros == 0 {
        lang.tr("unpriced").to_owned()
    } else {
        usd(cost_micros)
    }
}

fn model_line(lang: Lang, name: &str, tokens: u64, cost: i64, partial: bool) -> String {
    let mark = if partial {
        format!(" {PARTIAL_MARK}")
    } else {
        String::new()
    };
    format!(
        "{name}{mark}\t{}\t{}",
        compact_tokens(lang, tokens),
        cost_text(lang, cost, partial)
    )
}

fn other_line(lang: Lang, other: &OtherModels) -> String {
    let name = fill(
        lang.tr("Other ({count})"),
        &[("count", &other.count.to_string())],
    );
    model_line(
        lang,
        &name,
        other.total_tokens,
        other.cost_usd_micros,
        other.partial,
    )
}

fn partial_note(lang: Lang, models: &[ModelUsage], other: Option<&OtherModels>) -> Option<String> {
    let partial = models.iter().any(|model| model.partial) || other.is_some_and(|o| o.partial);
    partial.then(|| {
        format!(
            "{PARTIAL_MARK} {}",
            lang.tr("Partly unpriced, cost leaves it out")
        )
    })
}

#[must_use]
pub fn breakdown_text(
    lang: Lang,
    title: &str,
    models: &[ModelUsage],
    other: Option<&OtherModels>,
    totals: (i64, u64),
) -> Option<String> {
    if models.is_empty() && other.is_none() {
        return None;
    }
    let rows = models.iter().map(|model| {
        model_line(
            lang,
            &model.model,
            model.total_tokens,
            model.cost_usd_micros,
            model.partial,
        )
    });
    let total = format!(
        "{} · {}",
        exact_usd(totals.0),
        exact_tokens_text(lang, totals.1)
    );
    let lines: Vec<String> = std::iter::once(title.to_owned())
        .chain(rows)
        .chain(other.map(|other| other_line(lang, other)))
        .chain(std::iter::once(total))
        .chain(partial_note(lang, models, other))
        .collect();
    Some(lines.join("\n"))
}

#[must_use]
pub fn trend_days(daily: &[Daily]) -> Vec<Option<&Daily>> {
    let shown = &daily[daily.len().saturating_sub(TREND_DAYS)..];
    let padding = TREND_DAYS - shown.len();
    std::iter::repeat_n(None, padding)
        .chain(shown.iter().map(Some))
        .collect()
}

#[must_use]
pub fn bar_height(value: u64, peak: u64) -> u64 {
    if value == 0 || peak == 0 {
        return TREND_STUB;
    }
    let minimum = (TREND_HEIGHT * TREND_MIN_PERCENT + 50) / 100;
    let scaled = (u128::from(TREND_HEIGHT) * u128::from(value) * 2 + u128::from(peak))
        / (u128::from(peak) * 2);
    minimum.max(u64::try_from(scaled).unwrap_or(TREND_HEIGHT))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(name: &str, tokens: u64, cost: i64, partial: bool) -> ModelUsage {
        ModelUsage {
            model: name.into(),
            total_tokens: tokens,
            cost_usd_micros: cost,
            partial,
            cost_per_mtok_usd_micros: None,
        }
    }

    #[test]
    fn bar_heights_scale_to_the_peak() {
        assert_eq!(bar_height(0, 100), 2);
        assert_eq!(bar_height(100, 100), 18);
        assert_eq!(bar_height(1, 100), 3);
        assert_eq!(bar_height(50, 100), 9);
    }

    #[test]
    fn trend_pads_to_thirty_days() {
        let day = Daily {
            date: jiff::civil::Date::new(2026, 9, 23).unwrap(),
            total_tokens: 5,
            cost_usd_micros: 15,
            partial: false,
        };
        let days = trend_days(std::slice::from_ref(&day));
        assert_eq!(days.len(), 30);
        assert!(days[..29].iter().all(Option::is_none));
        assert_eq!(days[29], Some(&day));
        let many = vec![day; 40];
        assert_eq!(trend_days(&many).len(), 30);
    }

    #[test]
    fn breakdown_lists_models_and_other() {
        let models = [
            model("gpt-5.5", 1_200_000, 2_400_000, false),
            model("x", 10, 0, true),
        ];
        let other = OtherModels {
            count: 3,
            total_tokens: 500,
            cost_usd_micros: 1000,
            partial: false,
        };
        let text = breakdown_text(
            Lang::En,
            "Today · Codex",
            &models,
            Some(&other),
            (2_401_000, 1_200_510),
        )
        .unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "Today · Codex");
        assert_eq!(lines[1], "gpt-5.5\t1.2M\t$2.40");
        assert_eq!(lines[2], "x *\t10\tunpriced");
        assert_eq!(lines[3], "Other (3)\t500\t$0.00");
        assert_eq!(lines[4], "$2.40 · 1,200,510 tokens");
        assert_eq!(lines[5], "* Partly unpriced, cost leaves it out");
        assert!(breakdown_text(Lang::En, "t", &[], None, (0, 0)).is_none());
    }

    #[test]
    fn period_titles() {
        assert_eq!(Period::Last30Days.segment_title(Lang::En), "30 Days");
        assert_eq!(Period::Last30Days.row_title(Lang::Ru), "За 30 дней");
        let period = PeriodSpend {
            partial: true,
            ..PeriodSpend::default()
        };
        assert!(info_text(Lang::En, &period).ends_with("no public price yet."));
    }
}
