use std::cell::RefCell;
use std::rc::{Rc, Weak};

use adw::prelude::*;

use super::add_account::DialogCtx;
use super::flow_page::{
    DONE, ERROR, FORM, LogView, PROGRESS, busy_line, flow_body, navigation_page, pill, result_page,
    stack_of, wrapping,
};
use super::target::Target;
use crate::i18n::fill;
use crate::preferences::flow::{Flow, Phase, parse_add_event};
use crate::preferences::registry::{AddMethod, ProviderInfo};
use crate::process::ProgressProcess;

struct Widgets {
    description: gtk::Label,
    stack: gtk::Stack,
    status: gtk::Label,
    open: gtk::Button,
    code_box: gtk::Box,
    code: gtk::Label,
    paste: adw::PreferencesGroup,
    error: gtk::Label,
    log: LogView,
    label: adw::EntryRow,
}

pub struct LoginPage {
    pub page: adw::NavigationPage,
    ctx: DialogCtx,
    provider: ProviderInfo,
    method: AddMethod,
    target: Target,
    widgets: Widgets,
    flow: RefCell<Flow>,
    process: RefCell<Option<ProgressProcess>>,
}

fn label_entry(ctx: &DialogCtx) -> adw::EntryRow {
    adw::EntryRow::builder()
        .title(ctx.lang.tr("Label (optional)"))
        .build()
}

fn code_box(ctx: &DialogCtx, code: &gtk::Label) -> gtk::Box {
    code.add_css_class("title-1");
    code.add_css_class("monospace");
    code.set_selectable(true);
    let copy = gtk::Button::from_icon_name("edit-copy-symbolic");
    copy.set_tooltip_text(Some(ctx.lang.tr("Copy")));
    copy.set_valign(gtk::Align::Center);
    let source = code.clone();
    copy.connect_clicked(move |button| button.clipboard().set_text(&source.text()));
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    line.set_halign(gtk::Align::Center);
    line.append(code);
    line.append(&copy);
    let hint = wrapping(
        ctx.lang.tr("Enter this code on the sign-in page."),
        &["dim-label", "caption"],
    );
    let column = stack_of(&[
        wrapping(ctx.lang.tr("Your code"), &["heading"]).upcast_ref(),
        line.upcast_ref(),
        hint.upcast_ref(),
    ]);
    column.set_spacing(6);
    column
}

fn widgets(ctx: &DialogCtx, description: gtk::Label, stack: gtk::Stack) -> Widgets {
    let code = gtk::Label::new(None);
    Widgets {
        description,
        stack,
        status: gtk::Label::new(None),
        open: gtk::Button::new(),
        code_box: code_box(ctx, &code),
        code,
        paste: adw::PreferencesGroup::builder()
            .description(ctx.lang.tr("Only if the sign-in page asks for a code."))
            .build(),
        error: wrapping("", &["error"]),
        log: LogView::new(ctx.lang),
        label: label_entry(ctx),
    }
}

impl LoginPage {
    pub fn new(
        ctx: &DialogCtx,
        provider: &ProviderInfo,
        method: &AddMethod,
        target: &Target,
    ) -> Rc<Self> {
        let lang = ctx.lang;
        let body = flow_body(
            &provider.id,
            &ctx.logo_color,
            &fill(
                lang.tr("Sign in to {provider}"),
                &[("provider", &provider.display_name)],
            ),
        );
        let widgets = widgets(ctx, body.description, body.stack);
        body.column.append(&widgets.log.widget);
        let title = target.title(lang, &provider.display_name);
        let page = Rc::new(Self {
            page: navigation_page(&title, &body.column),
            ctx: ctx.clone(),
            provider: provider.clone(),
            method: method.clone(),
            target: target.clone(),
            widgets,
            flow: RefCell::default(),
            process: RefCell::default(),
        });
        page.assemble();
        page.render();
        if target.is_login() {
            page.start();
        }
        page
    }

    fn assemble(self: &Rc<Self>) {
        let (lang, widgets) = (self.ctx.lang, &self.widgets);
        let weak = Rc::downgrade(self);
        widgets
            .label
            .connect_entry_activated(move |_| with(&weak, Self::start));
        let group = adw::PreferencesGroup::new();
        group.add(&widgets.label);
        group.set_visible(!self.target.is_login());
        let weak = Rc::downgrade(self);
        let form = stack_of(&[
            group.upcast_ref(),
            pill(lang.tr("Continue"), true, move || with(&weak, Self::start)).upcast_ref(),
        ]);
        widgets.stack.add_named(&form, Some(FORM));
        widgets
            .stack
            .add_named(&self.progress_page(), Some(PROGRESS));
        let close = Rc::clone(&self.ctx.close);
        widgets.stack.add_named(
            &result_page(true, lang.tr("Done"), move || close()),
            Some(DONE),
        );
        let weak = Rc::downgrade(self);
        let retry = result_page(false, lang.tr("Try Again"), move || {
            with(&weak, |page| page.reset());
        });
        retry.prepend(&widgets.error);
        widgets.stack.add_named(&retry, Some(ERROR));
    }

    fn progress_page(self: &Rc<Self>) -> gtk::Box {
        let (lang, widgets) = (self.ctx.lang, &self.widgets);
        widgets.open.set_label(lang.tr("Open Sign-In Page"));
        widgets.open.add_css_class("pill");
        widgets.open.add_css_class("suggested-action");
        widgets.open.set_halign(gtk::Align::Center);
        let weak = Rc::downgrade(self);
        widgets
            .open
            .connect_clicked(move |_| with(&weak, |page| page.open_url()));
        let paste = adw::EntryRow::builder()
            .title(lang.tr("Paste code"))
            .show_apply_button(true)
            .build();
        let weak = Rc::downgrade(self);
        paste.connect_apply(move |entry| with(&weak, |page| page.send_code(entry)));
        widgets.paste.add(&paste);
        let close = Rc::clone(&self.ctx.close);
        let cancel = pill(lang.tr("Cancel"), false, move || close());
        stack_of(&[
            busy_line(&widgets.status).upcast_ref(),
            widgets.open.upcast_ref(),
            widgets.code_box.upcast_ref(),
            widgets.paste.upcast_ref(),
            cancel.upcast_ref(),
        ])
    }

    fn start(self: &Rc<Self>) {
        if self.process.borrow().is_some() {
            return;
        }
        let args = self
            .target
            .args(&self.provider.id, &self.method, &self.widgets.label.text());
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        self.flow.borrow_mut().start();
        let (on_line, on_exit) = (Rc::downgrade(self), Rc::downgrade(self));
        let started = ProgressProcess::start(
            &words,
            move |line| with(&on_line, |page| page.line(line)),
            move |result| {
                with(&on_exit, |page| {
                    page.exited(result.err().map(|error| error.message(page.ctx.lang)));
                });
            },
        );
        match started {
            Ok(process) => *self.process.borrow_mut() = Some(process),
            Err(error) => self.flow.borrow_mut().fail(error.message(self.ctx.lang)),
        }
        self.render();
    }

    fn line(&self, line: &str) {
        if let Some(event) = parse_add_event(line) {
            self.flow.borrow_mut().event(event);
            self.render();
        }
    }

    fn exited(&self, failure: Option<String>) {
        self.process.borrow_mut().take();
        self.flow.borrow_mut().exited(self.ctx.lang, failure);
        self.render();
    }

    fn reset(&self) {
        self.cancel();
        self.flow.borrow_mut().reset();
        self.render();
    }

    pub fn cancel(&self) {
        if let Some(process) = self.process.borrow_mut().take() {
            process.cancel();
        }
    }

    fn open_url(&self) {
        if let Some(url) = self.flow.borrow().url.clone() {
            gtk::UriLauncher::new(&url).launch(
                self.page.root().and_downcast_ref::<gtk::Window>(),
                gtk::gio::Cancellable::NONE,
                |result| {
                    if let Err(error) = result {
                        tracing::warn!(%error, "could not open the sign-in page");
                    }
                },
            );
        }
    }

    fn send_code(&self, entry: &adw::EntryRow) {
        let code = entry.text().trim().to_owned();
        if let (false, Some(process)) = (code.is_empty(), self.process.borrow().as_ref()) {
            process.send_line(&code);
            entry.set_text("");
            self.flow
                .borrow_mut()
                .log
                .push(self.ctx.lang.tr("Code sent").to_owned());
            self.render();
        }
    }

    fn render(&self) {
        let (lang, widgets, flow) = (self.ctx.lang, &self.widgets, self.flow.borrow());
        let waiting = flow.url.is_some();
        let (child, description) = match &flow.phase {
            Phase::Form => (FORM, fill(lang.tr("Headroom signs in with the {provider} CLI in its own folder, so the account you use today stays signed in."), &[("provider", &self.provider.display_name)])),
            Phase::Running if waiting => (PROGRESS, lang.tr("Open the sign-in page and finish in your browser.").to_owned()),
            Phase::Running => (PROGRESS, lang.tr("This takes a moment.").to_owned()),
            Phase::Done => (DONE, self.target.done_text(lang).to_owned()),
            Phase::Failed(message) => (ERROR, message.clone()),
        };
        widgets.stack.set_visible_child_name(child);
        widgets
            .description
            .set_text(if child == ERROR { "" } else { &description });
        widgets.error.set_text(&description);
        widgets.status.set_text(if waiting {
            lang.tr("Waiting for sign-in…")
        } else {
            lang.tr("Starting sign-in…")
        });
        widgets.open.set_visible(waiting);
        widgets.paste.set_visible(waiting);
        widgets
            .code
            .set_text(flow.code.as_deref().unwrap_or_default());
        widgets.code_box.set_visible(flow.code.is_some());
        widgets.log.show(&flow.log);
    }
}

pub fn with<T>(weak: &Weak<T>, action: impl FnOnce(&Rc<T>)) {
    if let Some(page) = weak.upgrade() {
        action(&page);
    }
}
