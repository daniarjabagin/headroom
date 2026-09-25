use std::rc::Rc;

use jiff::Timestamp;

use crate::dates::Clock;
use crate::i18n::Lang;
use crate::palette::Scheme;
use crate::payload::Display;
use crate::ui::context::{Ctx, Tick};

pub type SharedTick = Rc<dyn Fn(Timestamp)>;

pub fn capture<T>(ctx: &Ctx, make: impl FnOnce() -> T) -> (T, Vec<SharedTick>) {
    let start = ctx.ticks.borrow().len();
    let value = make();
    let ticks = ctx
        .ticks
        .borrow_mut()
        .drain(start..)
        .map(SharedTick::from)
        .collect();
    (value, ticks)
}

pub fn boxed(ticks: &[SharedTick]) -> Vec<Tick> {
    ticks
        .iter()
        .map(|tick| {
            let tick = Rc::clone(tick);
            Box::new(move |now: Timestamp| tick(now)) as Tick
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct LookKey {
    lang: Lang,
    clock: Clock,
    scheme: Scheme,
    motion: bool,
    offline: bool,
    recent: bool,
    display: Display,
}

impl LookKey {
    pub fn of(ctx: &Ctx) -> Self {
        Self {
            lang: ctx.locale.lang,
            clock: ctx.locale.clock,
            scheme: ctx.palette.scheme(),
            motion: ctx.motion,
            offline: ctx.offline,
            recent: ctx.recent,
            display: ctx.display.clone(),
        }
    }
}

pub struct Keyed<K> {
    key: K,
    pub widget: gtk::Widget,
    pub ticks: Vec<SharedTick>,
}

impl<K: PartialEq> Keyed<K> {
    pub fn reuse(
        previous: Option<Self>,
        key: K,
        ctx: &Ctx,
        build: impl FnOnce() -> gtk::Widget,
    ) -> (Self, bool) {
        match previous {
            Some(kept) if kept.key == key => (kept, false),
            _ => {
                let (widget, ticks) = capture(ctx, build);
                (Self { key, widget, ticks }, true)
            }
        }
    }
}
