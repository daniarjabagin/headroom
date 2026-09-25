use crate::i18n::Lang;
use crate::numbers::{compact_tokens, ring_usd, usd};
use crate::payload::{
    Display, PeriodSpend, ProviderSpend, Spend, SpendBreakdown, SpendPeriod, SpendUnit,
};

const PERIODS: [SpendPeriod; 4] = [
    SpendPeriod::Today,
    SpendPeriod::Yesterday,
    SpendPeriod::Last7Days,
    SpendPeriod::Last30Days,
];
const UNITS: [SpendUnit; 3] = [SpendUnit::Cost, SpendUnit::Tokens, SpendUnit::CostPerMtok];
const MISSING: &str = "—";
const MICROS_PER_MILLION_TOKENS: u128 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpendOverride {
    pub period: Option<SpendPeriod>,
    pub unit: Option<SpendUnit>,
    pub breakdown: Option<SpendBreakdown>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendChoice {
    pub period: SpendPeriod,
    pub unit: SpendUnit,
    pub breakdown: SpendBreakdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    Cost,
    Tokens,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RingCenter {
    pub amount: String,
    pub unit_line: Option<&'static str>,
}

impl SpendChoice {
    #[must_use]
    pub fn resolve(display: &Display, chosen: SpendOverride, spend: &Spend, recent: bool) -> Self {
        let period = chosen.period.unwrap_or(display.spend_period);
        let unit = chosen.unit.unwrap_or(display.spend_unit);
        let breakdown = chosen.breakdown.unwrap_or(display.spend_breakdown);
        Self {
            period: if spend.has_period(period) {
                period
            } else {
                SpendPeriod::Last30Days
            },
            unit: if units(recent).contains(&unit) {
                unit
            } else {
                SpendUnit::Cost
            },
            breakdown: if spend.has_projects() {
                breakdown
            } else {
                SpendBreakdown::Models
            },
        }
    }

    #[must_use]
    pub fn period_of(self, spend: &Spend) -> &PeriodSpend {
        spend.period(self.period).unwrap_or(&spend.last_30_days)
    }

    #[must_use]
    pub fn basis(self) -> Basis {
        match self.unit {
            SpendUnit::Cost => Basis::Cost,
            SpendUnit::Tokens | SpendUnit::CostPerMtok => Basis::Tokens,
        }
    }
}

#[must_use]
pub fn periods(spend: &Spend) -> Vec<SpendPeriod> {
    PERIODS
        .into_iter()
        .filter(|period| spend.has_period(*period))
        .collect()
}

#[must_use]
pub fn units(recent: bool) -> Vec<SpendUnit> {
    UNITS
        .into_iter()
        .filter(|unit| recent || *unit != SpendUnit::CostPerMtok)
        .collect()
}

#[must_use]
pub fn period_title(lang: Lang, period: SpendPeriod) -> &'static str {
    lang.tr(match period {
        SpendPeriod::Today => "Today",
        SpendPeriod::Yesterday => "Yesterday",
        SpendPeriod::Last7Days => "7 Days",
        SpendPeriod::Last30Days => "30 Days",
    })
}

#[must_use]
pub fn period_heading(lang: Lang, period: SpendPeriod) -> &'static str {
    lang.tr(match period {
        SpendPeriod::Today => "Today",
        SpendPeriod::Yesterday => "Yesterday",
        SpendPeriod::Last7Days => "Last 7 days",
        SpendPeriod::Last30Days => "Last 30 days",
    })
}

#[must_use]
pub fn unit_title(lang: Lang, unit: SpendUnit) -> &'static str {
    lang.tr(match unit {
        SpendUnit::Cost => "Total Spend",
        SpendUnit::Tokens => "Total Tokens",
        SpendUnit::CostPerMtok => "Cost per MTok",
    })
}

#[must_use]
pub fn unit_subtitle(lang: Lang, unit: SpendUnit) -> &'static str {
    lang.tr(match unit {
        SpendUnit::Cost => "Dollars, from local logs and public prices",
        SpendUnit::Tokens => "Input, output and cache tokens",
        SpendUnit::CostPerMtok => "Spend divided by million tokens",
    })
}

#[must_use]
pub fn basis_of(basis: Basis, period: &PeriodSpend) -> Basis {
    if basis == Basis::Cost && period.cost_usd_micros <= 0 {
        Basis::Tokens
    } else {
        basis
    }
}

#[must_use]
pub fn measure(basis: Basis, cost_micros: i64, tokens: u64) -> u128 {
    match basis {
        Basis::Cost => u128::try_from(cost_micros.max(0)).unwrap_or(0),
        Basis::Tokens => u128::from(tokens),
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "slice proportions only, never shown as a number"
)]
#[must_use]
pub fn proportion(part: u128, whole: u128) -> f64 {
    if whole == 0 {
        0.0
    } else {
        (part as f64 / whole as f64).clamp(0.0, 1.0)
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "slice proportions only, never shown as a number"
)]
#[must_use]
pub fn slice_values(choice: SpendChoice, period: &PeriodSpend) -> Vec<f64> {
    let basis = basis_of(choice.basis(), period);
    period
        .by_provider
        .iter()
        .map(|spend| measure(basis, spend.cost_usd_micros, spend.total_tokens) as f64)
        .collect()
}

#[must_use]
pub fn per_mtok_text(micros: Option<i64>) -> String {
    micros.map_or_else(|| MISSING.to_owned(), usd)
}

#[must_use]
pub fn per_mtok_of(cost_micros: i64, tokens: u64, partial: bool) -> Option<i64> {
    if partial || tokens == 0 || cost_micros < 0 {
        return None;
    }
    let cost = u128::try_from(cost_micros).ok()?;
    let tokens = u128::from(tokens);
    let micros = (cost * MICROS_PER_MILLION_TOKENS * 2 + tokens) / (tokens * 2);
    i64::try_from(micros).ok()
}

#[must_use]
pub fn ring_center(lang: Lang, unit: SpendUnit, period: &PeriodSpend) -> RingCenter {
    match unit {
        SpendUnit::Cost => RingCenter {
            amount: ring_usd(period.cost_usd_micros),
            unit_line: None,
        },
        SpendUnit::Tokens => RingCenter {
            amount: compact_tokens(lang, period.total_tokens),
            unit_line: Some(lang.tr("tokens")),
        },
        SpendUnit::CostPerMtok => RingCenter {
            amount: per_mtok_text(period.cost_per_mtok_usd_micros),
            unit_line: Some(lang.tr("blended")),
        },
    }
}

#[must_use]
pub fn unit_value(lang: Lang, unit: SpendUnit, figures: Figures) -> String {
    match unit {
        SpendUnit::Cost => usd(figures.cost_micros),
        SpendUnit::Tokens => compact_tokens(lang, figures.tokens),
        SpendUnit::CostPerMtok => per_mtok_text(figures.per_mtok),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Figures {
    pub cost_micros: i64,
    pub tokens: u64,
    pub per_mtok: Option<i64>,
}

impl From<&ProviderSpend> for Figures {
    fn from(spend: &ProviderSpend) -> Self {
        Self {
            cost_micros: spend.cost_usd_micros,
            tokens: spend.total_tokens,
            per_mtok: spend.cost_per_mtok_usd_micros,
        }
    }
}

#[must_use]
pub fn permille(part: u128, whole: u128) -> u32 {
    if whole == 0 {
        return 0;
    }
    u32::try_from((part.min(whole) * 1000) / whole).unwrap_or(1000)
}

#[must_use]
pub fn share_text(lang: Lang, permille: u32) -> String {
    let text = format!("{}.{}%", permille / 10, permille % 10);
    match lang {
        Lang::En => text,
        Lang::Ru => text.replace('.', ","),
    }
}

#[must_use]
pub fn whole_share_text(permille: u32) -> String {
    format!("{}%", permille / 10)
}

#[cfg(test)]
#[path = "spend_view_tests.rs"]
mod tests;
