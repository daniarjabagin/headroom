use std::cell::RefCell;

use adw::prelude::*;

const MIN_WIDTH: i32 = 360;
const MIN_HEIGHT: i32 = 294;
const NARROW: &str = "max-width: 550sp";

#[derive(Clone)]
pub struct Tab {
    pub name: &'static str,
    pub title: String,
    pub icon: String,
    pub widget: gtk::Widget,
}

impl Tab {
    pub fn of_page(name: &'static str, page: &adw::PreferencesPage) -> Self {
        Self {
            name,
            title: page.title().to_string(),
            icon: page.icon_name().map(String::from).unwrap_or_default(),
            widget: page.clone().upcast(),
        }
    }
}

pub struct Chrome {
    pub window: adw::Window,
    stack: adw::ViewStack,
    toasts: adw::ToastOverlay,
    shown: RefCell<Vec<gtk::Widget>>,
}

fn narrow_breakpoint(
    switcher: &adw::ViewSwitcher,
    bar: &adw::ViewSwitcherBar,
) -> Option<adw::Breakpoint> {
    let condition = adw::BreakpointCondition::parse(NARROW).ok()?;
    let breakpoint = adw::Breakpoint::new(condition);
    breakpoint.add_setter(switcher, "visible", Some(&false.to_value()));
    breakpoint.add_setter(bar, "reveal", Some(&true.to_value()));
    Some(breakpoint)
}

impl Chrome {
    pub fn new(width: i32, height: i32) -> Self {
        let stack = adw::ViewStack::new();
        let switcher = adw::ViewSwitcher::builder()
            .stack(&stack)
            .policy(adw::ViewSwitcherPolicy::Wide)
            .build();
        let bar = adw::ViewSwitcherBar::builder().stack(&stack).build();
        let header = adw::HeaderBar::builder().title_widget(&switcher).build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&stack));
        toolbar.add_bottom_bar(&bar);
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&toolbar));
        let window = adw::Window::builder()
            .default_width(width)
            .default_height(height)
            .width_request(MIN_WIDTH)
            .height_request(MIN_HEIGHT)
            .hide_on_close(true)
            .content(&toasts)
            .build();
        if let Some(breakpoint) = narrow_breakpoint(&switcher, &bar) {
            window.add_breakpoint(breakpoint);
        }
        Self {
            window,
            stack,
            toasts,
            shown: RefCell::default(),
        }
    }

    pub fn clear(&self) {
        for widget in self.shown.take() {
            if widget.parent().is_some() {
                self.stack.remove(&widget);
            }
        }
    }

    pub fn show(&self, tabs: &[Tab]) {
        self.clear();
        for tab in tabs {
            self.stack
                .add_titled_with_icon(&tab.widget, Some(tab.name), &tab.title, &tab.icon);
        }
        *self.shown.borrow_mut() = tabs.iter().map(|tab| tab.widget.clone()).collect();
    }

    pub fn visible_name(&self) -> Option<String> {
        self.stack.visible_child_name().map(String::from)
    }

    pub fn select_name(&self, name: &str) {
        if self.stack.child_by_name(name).is_some() {
            self.stack.set_visible_child_name(name);
        }
    }

    pub fn select(&self, widget: &gtk::Widget) -> bool {
        if widget.parent().is_none() {
            return false;
        }
        self.stack.set_visible_child(widget);
        true
    }

    pub fn toast(&self, toast: adw::Toast) {
        self.toasts.add_toast(toast);
    }
}
