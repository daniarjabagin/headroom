use crate::format::round_percent;
use crate::payload::{PanelIndicator, PanelItem, PanelLabel, PanelMode, State, Tone, ValueMode};

pub const MAX_GAUGES: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Gauge {
    pub percent: u8,
    pub tick: Option<u8>,
    pub tone: Tone,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Glyph {
    Mark(Option<Tone>),
    Rings(Vec<Gauge>),
    Bars(Vec<Gauge>),
}

fn percent_byte(value: f64) -> u8 {
    u8::try_from(round_percent(value).min(100)).unwrap_or(100)
}

#[must_use]
pub fn tone_rank(tone: Tone) -> u8 {
    match tone {
        Tone::Critical => 3,
        Tone::Warning => 2,
        Tone::Good => 1,
        Tone::Neutral => 0,
    }
}

impl Gauge {
    #[must_use]
    pub fn of(item: &PanelItem, mode: ValueMode) -> Self {
        let tick = item.even_pace_percent.map(|even| match mode {
            ValueMode::Used => percent_byte(even),
            ValueMode::Left => percent_byte(100.0 - even.clamp(0.0, 100.0)),
        });
        Self {
            percent: percent_byte(item.value_percent),
            tick,
            tone: item.headline.tone,
        }
    }
}

#[must_use]
pub fn worst_items(items: &[PanelItem]) -> Vec<&PanelItem> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|index| {
        (
            std::cmp::Reverse(tone_rank(items[*index].headline.tone)),
            *index,
        )
    });
    order.truncate(MAX_GAUGES);
    order.sort_unstable();
    order.into_iter().map(|index| &items[index]).collect()
}

fn alarming(tone: Option<Tone>) -> Option<Tone> {
    tone.filter(|tone| matches!(tone, Tone::Warning | Tone::Critical))
}

impl Glyph {
    #[must_use]
    pub fn mark(tone: Option<Tone>) -> Self {
        Glyph::Mark(alarming(tone))
    }

    #[must_use]
    pub fn from_state(state: Option<&State>) -> Self {
        let Some(state) = state else {
            return Glyph::Mark(None);
        };
        let display = &state.display;
        if display.panel_mode == PanelMode::Icon {
            return Glyph::mark(state.panel_tone);
        }
        let items = state.resolved_panel_items();
        if items.is_empty() {
            return Glyph::Mark(None);
        }
        if display.panel_indicator == PanelIndicator::None
            && display.panel_label == PanelLabel::None
        {
            return Glyph::mark(state.panel_tone);
        }
        let gauges = worst_items(&items)
            .into_iter()
            .map(|item| Gauge::of(item, display.value_mode))
            .collect();
        match display.panel_indicator {
            PanelIndicator::Bar => Glyph::Bars(gauges),
            PanelIndicator::Ring | PanelIndicator::None => Glyph::Rings(gauges),
        }
    }

    #[must_use]
    pub fn is_themed_mark(&self) -> bool {
        *self == Glyph::Mark(None)
    }

    #[must_use]
    pub fn has_critical(&self) -> bool {
        match self {
            Glyph::Mark(tone) => *tone == Some(Tone::Critical),
            Glyph::Rings(gauges) | Glyph::Bars(gauges) => {
                gauges.iter().any(|gauge| gauge.tone == Tone::Critical)
            }
        }
    }
}

#[cfg(test)]
#[path = "glyph_tests.rs"]
mod tests;
