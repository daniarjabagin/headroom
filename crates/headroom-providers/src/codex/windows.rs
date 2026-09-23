use headroom_core::quota::{QuotaWindow, WindowId};
use headroom_core::units::Percent;
use jiff::{SignedDuration, Timestamp};

use super::labels::{limit_label, period_label};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Slot {
    Primary,
    Secondary,
}

impl Slot {
    fn name(self) -> &'static str {
        match self {
            Slot::Primary => "primary",
            Slot::Secondary => "secondary",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Slot::Primary => "Primary",
            Slot::Secondary => "Secondary",
        }
    }

    fn default_kind(self) -> Kind {
        match self {
            Slot::Primary => Kind::Session,
            Slot::Secondary => Kind::Weekly,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct WindowReading {
    pub slot: Slot,
    pub used: Percent,
    pub period: Option<SignedDuration>,
    pub resets_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Family<'a> {
    Main,
    Model(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Session,
    Weekly,
    Custom(SignedDuration),
    Slot(Slot),
}

impl Kind {
    fn exact(period: SignedDuration) -> Kind {
        match WindowId::from_period(period) {
            Some(WindowId::Session) => Kind::Session,
            Some(WindowId::Weekly) => Kind::Weekly,
            _ => Kind::Custom(period),
        }
    }

    fn default_period(self) -> Option<SignedDuration> {
        match self {
            Kind::Session => Some(WindowId::SESSION_PERIOD),
            Kind::Weekly => Some(WindowId::WEEKLY_PERIOD),
            Kind::Custom(period) => Some(period),
            Kind::Slot(_) => None,
        }
    }
}

pub(super) fn classify(family: Family<'_>, readings: Vec<WindowReading>) -> Vec<QuotaWindow> {
    let mut taken: Vec<Kind> = Vec::new();
    let exact: Vec<Option<Kind>> = readings
        .iter()
        .map(|reading| {
            reading
                .period
                .map(Kind::exact)
                .filter(|kind| claim(&mut taken, *kind))
        })
        .collect();
    readings
        .into_iter()
        .zip(exact)
        .map(|(reading, kind)| {
            let kind = kind.unwrap_or_else(|| fallback_kind(&mut taken, reading.slot));
            window(family, kind, reading)
        })
        .collect()
}

fn claim(taken: &mut Vec<Kind>, kind: Kind) -> bool {
    if taken.contains(&kind) {
        return false;
    }
    taken.push(kind);
    true
}

fn fallback_kind(taken: &mut Vec<Kind>, slot: Slot) -> Kind {
    let preferred = slot.default_kind();
    if claim(taken, preferred) {
        preferred
    } else {
        taken.push(Kind::Slot(slot));
        Kind::Slot(slot)
    }
}

fn window(family: Family<'_>, kind: Kind, reading: WindowReading) -> QuotaWindow {
    let (id, label) = identify(family, kind);
    QuotaWindow {
        id,
        label,
        used: reading.used,
        resets_at: reading.resets_at,
        period: reading.period.or_else(|| kind.default_period()),
    }
}

fn identify(family: Family<'_>, kind: Kind) -> (WindowId, String) {
    match family {
        Family::Main => main_identity(kind),
        Family::Model(name) => model_identity(name, kind),
    }
}

fn main_identity(kind: Kind) -> (WindowId, String) {
    match kind {
        Kind::Session => (WindowId::Session, "Session".to_owned()),
        Kind::Weekly => (WindowId::Weekly, "Weekly".to_owned()),
        Kind::Custom(period) => {
            let label = period_label(period);
            (WindowId::Other(label.clone()), label)
        }
        Kind::Slot(slot) => (
            WindowId::Other(slot.name().to_owned()),
            slot.label().to_owned(),
        ),
    }
}

fn model_identity(name: &str, kind: Kind) -> (WindowId, String) {
    let base = limit_label(name);
    let (suffix, qualifier) = match kind {
        Kind::Session => return (WindowId::Model(name.to_owned()), base),
        Kind::Weekly => ("weekly".to_owned(), "Weekly".to_owned()),
        Kind::Custom(period) => (period_label(period), period_label(period)),
        Kind::Slot(slot) => (slot.name().to_owned(), slot.label().to_owned()),
    };
    (
        WindowId::Model(format!("{name}:{suffix}")),
        format!("{base} {qualifier}"),
    )
}

#[cfg(test)]
#[path = "windows_tests.rs"]
mod tests;
