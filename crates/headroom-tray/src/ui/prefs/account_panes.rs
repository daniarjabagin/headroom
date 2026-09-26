use std::rc::Rc;

use adw::prelude::*;

use super::rows::pill_button;
use crate::i18n::Lang;

const COLLAPSE_AT: &str = "max-width: 600sp";
const MIN_WIDTH: i32 = 360;
const MIN_HEIGHT: i32 = 240;
const MIN_SIDEBAR: f64 = 220.0;
const MAX_SIDEBAR: f64 = 300.0;
const SIDEBAR_FRACTION: f64 = 0.32;
const FOOTER_MARGIN: i32 = 12;
const PEOPLE_ICON: &str = "system-users-symbolic";

pub const EMPTY: &str = "empty";
pub const DETAIL: &str = "detail";

pub struct Panes {
    pub root: adw::BreakpointBin,
    pub split: adw::NavigationSplitView,
    pub list: gtk::ListBox,
    pub content: adw::NavigationPage,
    pub stack: gtk::Stack,
}

fn add_button(lang: Lang, on_add: &Rc<dyn Fn()>) -> gtk::Button {
    let content = adw::ButtonContent::builder()
        .icon_name("list-add-symbolic")
        .label(lang.tr("Add Account…"))
        .build();
    let button = gtk::Button::builder()
        .child(&content)
        .halign(gtk::Align::Start)
        .build();
    button.add_css_class("flat");
    let on_add = Rc::clone(on_add);
    button.connect_clicked(move |_| on_add());
    button
}

fn caption(text: &str) -> gtk::Label {
    let label = gtk::Label::builder()
        .label(text)
        .wrap(true)
        .xalign(0.0)
        .build();
    label.add_css_class("dim-label");
    label.add_css_class("caption");
    label
}

fn list_placeholder(lang: Lang) -> gtk::Box {
    let placeholder = gtk::Box::new(gtk::Orientation::Vertical, 4);
    placeholder.set_margin_top(FOOTER_MARGIN * 2);
    placeholder.set_margin_start(FOOTER_MARGIN);
    placeholder.set_margin_end(FOOTER_MARGIN);
    let title = gtk::Label::new(Some(lang.tr("No accounts yet")));
    title.add_css_class("heading");
    placeholder.append(&title);
    let detail = caption(lang.tr("Sign in with a supported CLI, or add an account."));
    detail.set_xalign(0.5);
    detail.set_justify(gtk::Justification::Center);
    placeholder.append(&detail);
    placeholder
}

fn sidebar(lang: Lang, list: &gtk::ListBox, on_add: &Rc<dyn Fn()>) -> adw::NavigationPage {
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(list)
        .build();
    let footer = gtk::Box::new(gtk::Orientation::Vertical, 6);
    footer.set_margin_start(FOOTER_MARGIN / 2);
    footer.set_margin_end(FOOTER_MARGIN);
    footer.set_margin_bottom(FOOTER_MARGIN);
    footer.append(&add_button(lang, on_add));
    let note =
        caption(lang.tr("Hidden accounts keep updating but leave the tray and notifications."));
    note.set_margin_start(FOOTER_MARGIN / 2);
    footer.append(&note);
    let pane = gtk::Box::new(gtk::Orientation::Vertical, 0);
    pane.append(&scroller);
    pane.append(&footer);
    adw::NavigationPage::builder()
        .title(lang.tr("Accounts"))
        .tag("accounts")
        .child(&pane)
        .build()
}

fn detail_stack(lang: Lang, on_add: &Rc<dyn Fn()>) -> gtk::Stack {
    let stack = gtk::Stack::new();
    let empty = adw::StatusPage::builder()
        .icon_name(PEOPLE_ICON)
        .title(lang.tr("No accounts yet"))
        .description(lang.tr("Sign in with a supported CLI, or add an account."))
        .build();
    let on_add = Rc::clone(on_add);
    empty.set_child(Some(&pill_button(
        lang.tr("Add Account…"),
        true,
        move || {
            on_add();
        },
    )));
    stack.add_named(&empty, Some(EMPTY));
    stack
}

fn content(
    lang: Lang,
    split: &adw::NavigationSplitView,
    stack: &gtk::Stack,
) -> adw::NavigationPage {
    let header = adw::HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .build();
    header.add_css_class("flat");
    split
        .bind_property("collapsed", &header, "visible")
        .sync_create()
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(stack));
    adw::NavigationPage::builder()
        .title(lang.tr("Accounts"))
        .tag("account")
        .child(&toolbar)
        .build()
}

fn collapse_breakpoint(split: &adw::NavigationSplitView) -> Option<adw::Breakpoint> {
    let condition = adw::BreakpointCondition::parse(COLLAPSE_AT).ok()?;
    let breakpoint = adw::Breakpoint::new(condition);
    breakpoint.add_setters(&[(split, "collapsed", true)]);
    Some(breakpoint)
}

impl Panes {
    pub fn new(lang: Lang, on_add: &Rc<dyn Fn()>) -> Self {
        let list = gtk::ListBox::new();
        list.add_css_class("navigation-sidebar");
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.set_placeholder(Some(&list_placeholder(lang)));
        let split = adw::NavigationSplitView::builder()
            .min_sidebar_width(MIN_SIDEBAR)
            .max_sidebar_width(MAX_SIDEBAR)
            .sidebar_width_fraction(SIDEBAR_FRACTION)
            .build();
        let stack = detail_stack(lang, on_add);
        let content = content(lang, &split, &stack);
        split.set_sidebar(Some(&sidebar(lang, &list, on_add)));
        split.set_content(Some(&content));
        let root = adw::BreakpointBin::builder()
            .width_request(MIN_WIDTH)
            .height_request(MIN_HEIGHT)
            .child(&split)
            .build();
        if let Some(breakpoint) = collapse_breakpoint(&split) {
            root.add_breakpoint(breakpoint);
        }
        Self {
            root,
            split,
            list,
            content,
            stack,
        }
    }
}
