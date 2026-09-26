use gtk::prelude::*;

use crate::account::NoticeView;
use crate::i18n::Lang;
use crate::notices::NoticeKind;
use crate::recovery::{ButtonModel, ButtonState, NoticeButton, button_model, button_rows};
use crate::ui::widgets::{button, column, icon, label, row, wrapping_label};

const TILE_ICON: i32 = 14;
const LINE_ICON: i32 = 12;
const SPINNER_SIZE: i32 = 10;

struct ButtonHandle {
    kind: NoticeButton,
    button: gtk::Button,
    spinner: gtk::Spinner,
    label: gtk::Label,
}

pub struct MountedNotice {
    pub widget: gtk::Box,
    buttons: Vec<ButtonHandle>,
    motion: bool,
}

impl ButtonHandle {
    fn new(kind: &NoticeButton, on_press: Box<dyn Fn()>) -> Self {
        let content = row(4, &[]);
        let spinner = gtk::Spinner::new();
        spinner.set_size_request(SPINNER_SIZE, SPINNER_SIZE);
        let label = label("", &[]);
        content.append(&spinner);
        content.append(&label);
        let button = button(&content, &["headroom-small-button"], on_press);
        Self {
            kind: kind.clone(),
            button,
            spinner,
            label,
        }
    }

    fn apply(&self, model: ButtonModel, motion: bool) {
        self.label.set_text(model.label);
        self.spinner.set_visible(model.busy);
        self.spinner.set_spinning(model.busy && motion);
        self.button.set_sensitive(!model.busy);
        if model.primary {
            self.button.add_css_class("primary");
        } else {
            self.button.remove_css_class("primary");
        }
    }
}

impl MountedNotice {
    pub fn apply(&self, lang: Lang, state: impl Fn(&NoticeButton) -> ButtonState) {
        for handle in &self.buttons {
            handle.apply(
                button_model(lang, &handle.kind, state(&handle.kind)),
                self.motion,
            );
        }
    }
}

fn kind_icon(kind: NoticeKind) -> &'static str {
    match kind {
        NoticeKind::Error => "dialog-error-symbolic",
        NoticeKind::SignIn => "system-users-symbolic",
        NoticeKind::Warning | NoticeKind::Info => "dialog-warning-symbolic",
    }
}

fn kind_class(kind: NoticeKind) -> &'static str {
    match kind {
        NoticeKind::Error => "error",
        NoticeKind::SignIn => "signin",
        NoticeKind::Warning | NoticeKind::Info => "warning",
    }
}

fn tile(kind: NoticeKind) -> gtk::Box {
    let tile = row(0, &["headroom-notice-tile"]);
    tile.set_valign(gtk::Align::Start);
    let image = icon(kind_icon(kind), TILE_ICON, &[]);
    image.set_hexpand(true);
    image.set_halign(gtk::Align::Center);
    tile.append(&image);
    tile
}

fn texts(notice: &NoticeView) -> gtk::Box {
    let texts = column(1, &[]);
    texts.set_hexpand(true);
    texts.set_valign(gtk::Align::Center);
    texts.append(&wrapping_label(&notice.title, &["headroom-notice-title"]));
    if let Some(detail) = &notice.detail {
        texts.append(&wrapping_label(detail, &["headroom-notice-detail"]));
    }
    if let Some(note) = &notice.note {
        texts.append(&wrapping_label(
            note,
            &["headroom-notice-detail", "headroom-notice-note"],
        ));
    }
    texts
}

fn stacks_buttons(lang: Lang, buttons: &[NoticeButton]) -> bool {
    buttons
        .iter()
        .any(|kind| button_model(lang, kind, ButtonState::default()).primary)
}

fn button_line<'a>(handles: impl Iterator<Item = &'a ButtonHandle>) -> gtk::Box {
    let line = row(4, &[]);
    line.set_valign(gtk::Align::Center);
    for handle in handles {
        line.append(&handle.button);
    }
    line
}

fn stack_buttons(texts: &gtk::Box, handles: &[ButtonHandle], kinds: &[NoticeButton]) {
    for (index, kinds) in button_rows(kinds).iter().enumerate() {
        let line = button_line(handles.iter().filter(|handle| kinds.contains(&handle.kind)));
        line.set_margin_top(if index == 0 { 6 } else { 4 });
        line.set_halign(gtk::Align::Start);
        texts.append(&line);
    }
}

pub fn notice_row(
    lang: Lang,
    motion: bool,
    notice: &NoticeView,
    on_press: impl Fn(&NoticeButton) -> Box<dyn Fn()>,
) -> MountedNotice {
    let body = row(8, &["headroom-notice", kind_class(notice.kind)]);
    let texts = texts(notice);
    body.append(&tile(notice.kind));
    body.append(&texts);
    let buttons: Vec<ButtonHandle> = notice
        .buttons
        .iter()
        .map(|kind| ButtonHandle::new(kind, on_press(kind)))
        .collect();
    if stacks_buttons(lang, &notice.buttons) {
        stack_buttons(&texts, &buttons, &notice.buttons);
    } else if !buttons.is_empty() {
        body.append(&button_line(buttons.iter()));
    }
    MountedNotice {
        widget: body,
        buttons,
        motion,
    }
}

pub fn notice_line(text: &str) -> gtk::Box {
    let line = row(6, &["headroom-notice-line"]);
    let image = icon(
        "dialog-information-symbolic",
        LINE_ICON,
        &["headroom-notice-line-icon"],
    );
    image.set_valign(gtk::Align::Start);
    line.append(&image);
    line.append(&wrapping_label(text, &["headroom-notice-line-text"]));
    line
}
