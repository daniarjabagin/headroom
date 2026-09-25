use std::rc::Rc;

use gtk::prelude::*;
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
        build: impl FnOnce(&K) -> gtk::Widget,
    ) -> (Self, bool) {
        match previous {
            Some(kept) if kept.key == key => (kept, false),
            _ => {
                let (widget, ticks) = capture(ctx, || build(&key));
                (Self { key, widget, ticks }, true)
            }
        }
    }

    pub fn reuse_optional(
        previous: Option<Self>,
        key: K,
        ctx: &Ctx,
        build: impl FnOnce() -> Option<gtk::Widget>,
    ) -> Option<Self> {
        match previous {
            Some(kept) if kept.key == key => Some(kept),
            _ => {
                let (widget, ticks) = capture(ctx, build);
                widget.map(|widget| Self { key, widget, ticks })
            }
        }
    }
}

pub fn arrange(container: &gtk::Box, order: &[gtk::Widget], after: Option<&gtk::Widget>) {
    let mut child = after.map_or_else(|| container.first_child(), WidgetExt::next_sibling);
    while let Some(current) = child {
        child = current.next_sibling();
        if !order.contains(&current) {
            container.remove(&current);
        }
    }
    let mut previous: Option<&gtk::Widget> = after;
    for widget in order {
        if widget.parent().is_none() {
            container.append(widget);
        }
        container.reorder_child_after(widget, previous);
        previous = Some(widget);
    }
}
