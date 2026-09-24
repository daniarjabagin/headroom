use adw::prelude::*;

use super::account_row::provider_image;
use crate::i18n::Lang;

const BODY_MARGIN: i32 = 24;
const LOG_HEIGHT: i32 = 140;
const HEADING_LOGO: i32 = 48;
const RESULT_ICON: i32 = 32;

pub const FORM: &str = "form";
pub const PROGRESS: &str = "progress";
pub const DONE: &str = "done";
pub const ERROR: &str = "error";

pub fn stack_of(children: &[&gtk::Widget]) -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    for child in children {
        column.append(*child);
    }
    column
}

pub fn navigation_page(title: &str, content: &impl IsA<gtk::Widget>) -> adw::NavigationPage {
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .child(content)
        .build();
    toolbar.set_content(Some(&scroller));
    adw::NavigationPage::builder()
        .title(title)
        .child(&toolbar)
        .build()
}

pub fn wrapping(text: &str, classes: &[&str]) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_wrap(true);
    label.set_justify(gtk::Justification::Center);
    for class in classes {
        label.add_css_class(class);
    }
    label
}

pub struct FlowBody {
    pub column: gtk::Box,
    pub description: gtk::Label,
    pub stack: gtk::Stack,
}

pub fn flow_body(provider: &str, logo_color: &str, heading: &str) -> FlowBody {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 18);
    column.set_margin_top(6);
    column.set_margin_bottom(BODY_MARGIN);
    column.set_margin_start(BODY_MARGIN);
    column.set_margin_end(BODY_MARGIN);
    let title = wrapping(heading, &["title-2"]);
    let description = wrapping("", &["dim-label"]);
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .vhomogeneous(false)
        .build();
    column.append(&provider_image(provider, logo_color, HEADING_LOGO));
    column.append(&title);
    column.append(&description);
    column.append(&stack);
    FlowBody {
        column,
        description,
        stack,
    }
}

pub fn pill(label: &str, suggested: bool, on_click: impl Fn() + 'static) -> gtk::Button {
    super::rows::pill_button(label, suggested, on_click)
}

pub fn result_page(done: bool, label: &str, on_click: impl Fn() + 'static) -> gtk::Box {
    let (icon, class) = if done {
        ("object-select-symbolic", "success")
    } else {
        ("dialog-warning-symbolic", "warning")
    };
    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(RESULT_ICON);
    image.add_css_class(class);
    stack_of(&[image.upcast_ref(), pill(label, true, on_click).upcast_ref()])
}

pub fn busy_line(text: &gtk::Label) -> gtk::Box {
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    line.set_halign(gtk::Align::Center);
    let spinner = gtk::Spinner::new();
    spinner.set_spinning(true);
    text.add_css_class("heading");
    line.append(&spinner);
    line.append(text);
    line
}

pub struct LogView {
    pub widget: gtk::Expander,
    view: gtk::TextView,
}

impl LogView {
    pub fn new(lang: Lang) -> Self {
        let view = gtk::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::WordChar)
            .top_margin(8)
            .bottom_margin(8)
            .left_margin(10)
            .right_margin(10)
            .build();
        let scroller = gtk::ScrolledWindow::builder()
            .child(&view)
            .min_content_height(LOG_HEIGHT)
            .css_classes(["card"])
            .build();
        let widget = gtk::Expander::builder()
            .label(lang.tr("Details"))
            .child(&scroller)
            .visible(false)
            .build();
        Self { widget, view }
    }

    pub fn show(&self, lines: &[String]) {
        let text = lines.join("\n");
        let buffer = self.view.buffer();
        if buffer.text(&buffer.start_iter(), &buffer.end_iter(), false) != text {
            buffer.set_text(&text);
            let mut end = buffer.end_iter();
            self.view.scroll_to_iter(&mut end, 0.0, false, 0.0, 1.0);
        }
        self.widget.set_visible(!lines.is_empty());
    }
}
