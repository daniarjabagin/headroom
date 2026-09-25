use std::collections::HashMap;

use gtk::prelude::*;

use crate::payload::State;
use crate::ui::account_section::{MountedSection, SectionInput};
use crate::ui::context::Ctx;
use crate::ui::popup::{
    Entry, Frame, content_column, ready_entries, root_column, scroller, tail, view_widgets,
};
use crate::view::View;

struct Mounted {
    root: gtk::Box,
    scroller: gtk::ScrolledWindow,
    content: gtk::Box,
    sections: HashMap<String, MountedSection>,
}

#[derive(Default)]
pub struct PopupTree {
    mounted: Option<Mounted>,
}

fn arrange(content: &gtk::Box, order: &[gtk::Widget]) {
    let mut child = content.first_child();
    while let Some(current) = child {
        child = current.next_sibling();
        if !order.contains(&current) {
            content.remove(&current);
        }
    }
    let mut previous: Option<&gtk::Widget> = None;
    for widget in order {
        if widget.parent().is_none() {
            content.append(widget);
        }
        content.reorder_child_after(widget, previous);
        previous = Some(widget);
    }
}

impl Mounted {
    fn place(&mut self, ctx: &Ctx, state: &State, frame: &Frame) {
        let mut previous = std::mem::take(&mut self.sections);
        let mut order = Vec::new();
        for entry in ready_entries(ctx, state, frame) {
            order.push(match entry {
                Entry::Widget(widget) => widget,
                Entry::Account(input) => self.section(ctx, &input, &mut previous),
            });
        }
        arrange(&self.content, &order);
    }

    fn section(
        &mut self,
        ctx: &Ctx,
        input: &SectionInput,
        previous: &mut HashMap<String, MountedSection>,
    ) -> gtk::Widget {
        let id = input.account.id.clone();
        let section = match previous.remove(&id) {
            Some(mut section) => {
                section.update(ctx, input);
                section
            }
            None => MountedSection::new(ctx, input),
        };
        let widget = section.widget.clone().upcast();
        self.sections.insert(id, section);
        widget
    }

    fn replace_tail(&self, ctx: &Ctx, view: &View, frame: &Frame) {
        let mut child = self.scroller.next_sibling();
        while let Some(current) = child {
            child = current.next_sibling();
            self.root.remove(&current);
        }
        for widget in tail(ctx, view, frame.now) {
            self.root.append(&widget);
        }
    }

    fn update(&mut self, ctx: &Ctx, view: &View, state: &State, frame: &Frame) {
        if ctx.motion {
            self.root.remove_css_class("reduced-motion");
        } else {
            self.root.add_css_class("reduced-motion");
        }
        self.scroller.set_max_content_height(frame.max_height);
        self.place(ctx, state, frame);
        self.replace_tail(ctx, view, frame);
    }
}

impl PopupTree {
    pub fn mount(&mut self, ctx: &Ctx, view: &View, frame: &Frame) -> gtk::Box {
        let content = content_column();
        let root = root_column(ctx);
        let scroller = scroller(&content, frame);
        root.append(&scroller);
        let mut mounted = Mounted {
            root: root.clone(),
            scroller,
            content,
            sections: HashMap::new(),
        };
        if let View::Ready(state) = view {
            mounted.place(ctx, state, frame);
        } else {
            for widget in view_widgets(ctx, view) {
                mounted.content.append(&widget);
            }
        }
        mounted.replace_tail(ctx, view, frame);
        self.mounted = matches!(view, View::Ready(_)).then_some(mounted);
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
            (View::Ready(state), Some(mounted)) if !rebuild => {
                mounted.update(ctx, view, state, frame);
                None
            }
            _ => Some(self.mount(ctx, view, frame)),
        }
    }
}
