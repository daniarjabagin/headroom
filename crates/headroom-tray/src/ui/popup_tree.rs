use std::collections::HashMap;

use gtk::prelude::*;

use crate::payload::{Spend, State, Update};
use crate::popup_model::collapse::MoreSummary;
use crate::popup_model::spend_view::SpendChoice;
use crate::ui::account_section::{MountedSection, SectionInput};
use crate::ui::collapsed::{less_divider, more_row};
use crate::ui::combined_section::{GroupInput, MountedGroup};
use crate::ui::context::{Ctx, Tick};
use crate::ui::footer::{MountedFooter, RefreshButton, top_bar};
use crate::ui::keyed::{Keyed, LookKey, SharedTick, arrange, boxed};
use crate::ui::motion::{SLIDE_MS, fade};
use crate::ui::popup::{
    Entry, Frame, apply_root_classes, content_column, ready_entries, root_column, scroller,
    set_density, view_widgets,
};
use crate::ui::spend_card::spend_section;
use crate::ui::status_views::empty_view;
use crate::ui::toast::{Toast, ToastMessage};
use crate::ui::update_row::update_row;
use crate::update::UpdateRun;
use crate::view::View;

#[derive(PartialEq)]
enum LeadingKey {
    TopBar(LookKey),
    Spend {
        look: LookKey,
        spend: Box<Spend>,
        choice: Option<SpendChoice>,
    },
}

#[derive(Clone, PartialEq)]
enum MoreKey {
    More(LookKey, MoreSummary),
    Less(LookKey),
}

#[derive(PartialEq)]
struct UpdateKey {
    look: LookKey,
    update: Update,
    run: UpdateRun,
    open: bool,
    copied: bool,
}

struct Mounted {
    ready: bool,
    root: gtk::Box,
    overlay: gtk::Overlay,
    scroller: gtk::ScrolledWindow,
    content: gtk::Box,
    refresh: RefreshButton,
    toast: Toast,
    leading: Option<Keyed<LeadingKey>>,
    sections: HashMap<String, MountedSection>,
    groups: HashMap<String, MountedGroup>,
    more: Option<Keyed<MoreKey>>,
    empty: Option<Keyed<LookKey>>,
    update: Option<Keyed<UpdateKey>>,
    footer: (LookKey, MountedFooter),
    ticks: Vec<SharedTick>,
}

#[derive(Default)]
pub struct PopupTree {
    mounted: Option<Mounted>,
}

fn entrance(widget: &gtk::Widget, built: bool, ctx: &Ctx, frame: &Frame) {
    if built && !frame.animate {
        fade(widget, 0.0, 1.0, SLIDE_MS, ctx.motion);
    }
}

struct Pass<'m> {
    sections: HashMap<String, MountedSection>,
    groups: HashMap<String, MountedGroup>,
    ticks: &'m mut Vec<SharedTick>,
}

impl Mounted {
    fn leading(&mut self, ctx: &Ctx, spend: Option<&Spend>, frame: &Frame) -> gtk::Widget {
        let key = spend.map_or_else(
            || LeadingKey::TopBar(LookKey::of(ctx)),
            |spend| LeadingKey::Spend {
                look: LookKey::of(ctx),
                spend: Box::new(spend.clone()),
                choice: ctx.spend,
            },
        );
        let refresh: gtk::Widget = self.refresh.widget.clone().upcast();
        let (kept, _) = Keyed::reuse(self.leading.take(), key, ctx, |_| match spend {
            Some(spend) => spend_section(ctx, spend, &refresh, frame.animate).upcast(),
            None => top_bar(ctx, &refresh).upcast(),
        });
        let widget = kept.widget.clone();
        self.leading = Some(kept);
        widget
    }

    fn section(
        &mut self,
        ctx: &Ctx,
        input: &SectionInput,
        frame: &Frame,
        pass: &mut Pass,
    ) -> gtk::Widget {
        let id = input.account.id.clone();
        let (section, built) = match pass.sections.remove(&id) {
            Some(mut section) => {
                section.update(ctx, input);
                (section, false)
            }
            None => (MountedSection::new(ctx, input), true),
        };
        let widget: gtk::Widget = section.widget.clone().upcast();
        entrance(&widget, built, ctx, frame);
        pass.ticks.extend(section.ticks.iter().cloned());
        self.sections.insert(id, section);
        widget
    }

    fn group(
        &mut self,
        ctx: &Ctx,
        input: &GroupInput,
        frame: &Frame,
        pass: &mut Pass,
    ) -> gtk::Widget {
        let id = input.group.provider.clone();
        let (group, built) = match pass.groups.remove(&id) {
            Some(mut group) => {
                group.update(ctx, input);
                (group, false)
            }
            None => (MountedGroup::new(ctx, input), true),
        };
        let widget: gtk::Widget = group.widget.clone().upcast();
        entrance(&widget, built, ctx, frame);
        pass.ticks.extend(group.ticks.iter().cloned());
        self.groups.insert(id, group);
        widget
    }

    fn empty(&mut self, ctx: &Ctx) -> gtk::Widget {
        let (kept, _) = Keyed::reuse(self.empty.take(), LookKey::of(ctx), ctx, |_| {
            empty_view(ctx).upcast()
        });
        let widget = kept.widget.clone();
        self.empty = Some(kept);
        widget
    }

    fn more(&mut self, ctx: &Ctx, key: MoreKey) -> gtk::Widget {
        let shown = key.clone();
        let (kept, _) = Keyed::reuse(self.more.take(), key, ctx, |_| match &shown {
            MoreKey::More(_, summary) => more_row(ctx, summary).upcast(),
            MoreKey::Less(_) => less_divider(ctx).upcast(),
        });
        let widget = kept.widget.clone();
        self.more = Some(kept);
        widget
    }

    fn place(&mut self, ctx: &Ctx, state: &State, frame: &Frame) {
        self.refresh.apply(ctx);
        set_density(ctx, &self.content);
        let mut ticks = Vec::new();
        let mut pass = Pass {
            sections: std::mem::take(&mut self.sections),
            groups: std::mem::take(&mut self.groups),
            ticks: &mut ticks,
        };
        let mut order = Vec::new();
        for entry in ready_entries(ctx, state, frame) {
            order.push(match entry {
                Entry::Empty => self.empty(ctx),
                Entry::Leading(spend) => self.leading(ctx, spend, frame),
                Entry::Account(input) => self.section(ctx, &input, frame, &mut pass),
                Entry::Group(input) => self.group(ctx, &input, frame, &mut pass),
                Entry::More(summary) => self.more(ctx, MoreKey::More(LookKey::of(ctx), summary)),
                Entry::Less => self.more(ctx, MoreKey::Less(LookKey::of(ctx))),
            });
        }
        arrange(&self.content, &order, None);
        self.ticks = ticks;
    }

    fn update_widget(&mut self, ctx: &Ctx, update: Option<&Update>) -> Option<gtk::Widget> {
        let Some(update) = update else {
            self.update = None;
            return None;
        };
        let key = UpdateKey {
            look: LookKey::of(ctx),
            update: update.clone(),
            run: ctx.ui.update_run.clone(),
            open: ctx.ui.update_command_open,
            copied: ctx.ui.copied,
        };
        let (kept, _) = Keyed::reuse(self.update.take(), key, ctx, |_| {
            update_row(ctx, update).upcast()
        });
        let widget = kept.widget.clone();
        self.update = Some(kept);
        Some(widget)
    }

    fn sync_tail(&mut self, ctx: &Ctx, view: &View, frame: &Frame) {
        let look = LookKey::of(ctx);
        if self.footer.0 == look {
            self.footer.1.update(ctx, view, frame.now);
        } else {
            self.footer = (look, MountedFooter::new(ctx, view, frame.now));
        }
        let update = self.update_widget(ctx, view.state().and_then(|s| s.update.as_ref()));
        let order: Vec<gtk::Widget> = update
            .into_iter()
            .chain(std::iter::once(self.footer.1.widget.clone().upcast()))
            .collect();
        arrange(&self.root, &order, Some(self.overlay.upcast_ref()));
    }

    fn update(&mut self, ctx: &Ctx, view: &View, state: &State, frame: &Frame) {
        apply_root_classes(ctx, &self.root);
        self.scroller.set_max_content_height(frame.max_height);
        self.place(ctx, state, frame);
        self.sync_tail(ctx, view, frame);
    }

    fn all_ticks(&self) -> Vec<Tick> {
        let mut ticks = boxed(&self.ticks);
        ticks.extend(boxed(&self.footer.1.ticks));
        if let Some(update) = &self.update {
            ticks.extend(boxed(&update.ticks));
        }
        ticks
    }
}

impl PopupTree {
    pub fn mount(&mut self, ctx: &Ctx, view: &View, frame: &Frame) -> gtk::Box {
        let content = content_column(ctx);
        let root = root_column(ctx);
        let scroller = scroller(&content, frame);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&scroller));
        let toast = Toast::default();
        overlay.add_overlay(&toast.widget);
        root.append(&overlay);
        let mut mounted = Mounted {
            ready: matches!(view, View::Ready(_)),
            root: root.clone(),
            overlay,
            scroller,
            content,
            refresh: RefreshButton::new(ctx),
            toast,
            leading: None,
            sections: HashMap::new(),
            groups: HashMap::new(),
            more: None,
            empty: None,
            update: None,
            footer: (LookKey::of(ctx), MountedFooter::new(ctx, view, frame.now)),
            ticks: Vec::new(),
        };
        if let View::Ready(state) = view {
            mounted.place(ctx, state, frame);
        } else {
            for widget in view_widgets(ctx, view) {
                mounted.content.append(&widget);
            }
        }
        mounted.sync_tail(ctx, view, frame);
        self.mounted = Some(mounted);
        root
    }

    pub fn render(
        &mut self,
        ctx: &Ctx,
        view: &View,
        frame: &Frame,
        rebuild: bool,
    ) -> Option<gtk::Box> {
        match (view, self.mounted.as_mut()) {
            (View::Ready(state), Some(mounted)) if mounted.ready && !rebuild => {
                mounted.update(ctx, view, state, frame);
                None
            }
            _ => Some(self.mount(ctx, view, frame)),
        }
    }

    #[must_use]
    pub fn ticks(&self) -> Vec<Tick> {
        self.mounted
            .as_ref()
            .map(Mounted::all_ticks)
            .unwrap_or_default()
    }

    pub fn toast(&self, message: &ToastMessage, motion: bool) {
        if let Some(mounted) = &self.mounted {
            mounted.toast.show(message, motion);
        }
    }

    pub fn dismiss_toast(&self) {
        if let Some(mounted) = &self.mounted {
            mounted.toast.dismiss();
        }
    }
}
