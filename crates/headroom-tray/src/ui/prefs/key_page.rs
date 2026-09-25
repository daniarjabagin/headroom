use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use super::add_account::DialogCtx;
use super::flow_page::{
    DONE, ERROR, FORM, PROGRESS, busy_line, flow_body, navigation_page, pill, result_page,
    stack_of, wrapping,
};
use super::login_page::with;
use super::target::Target;
use crate::i18n::fill;
use crate::preferences::flow::{Flow, Phase, parse_add_event};
use crate::preferences::registry::{AddMethod, ProviderInfo};
use crate::process::ProgressProcess;

struct Widgets {
    description: gtk::Label,
    stack: gtk::Stack,
    key: adw::PasswordEntryRow,
    label: adw::EntryRow,
    add: gtk::Button,
    error: gtk::Label,
}

pub struct KeyPage {
    pub page: adw::NavigationPage,
    ctx: DialogCtx,
    provider: ProviderInfo,
    method: AddMethod,
    target: Target,
    widgets: Widgets,
    flow: RefCell<Flow>,
    process: RefCell<Option<ProgressProcess>>,
}

fn key_label(ctx: &DialogCtx, method: &AddMethod) -> String {
    match method {
        AddMethod::ApiKey {
            label: Some(label), ..
        } => label.clone(),
        _ => ctx.lang.tr("API key").to_owned(),
    }
}

fn hint(method: &AddMethod) -> &str {
    match method {
        AddMethod::ApiKey {
            hint: Some(hint), ..
        } => hint,
        _ => "",
    }
}

fn console_url(method: &AddMethod) -> Option<String> {
    match method {
        AddMethod::ApiKey { console_url, .. } => console_url.clone(),
        _ => None,
    }
}

impl KeyPage {
    pub fn new(
        ctx: &DialogCtx,
        provider: &ProviderInfo,
        method: &AddMethod,
        target: &Target,
    ) -> Rc<Self> {
        let lang = ctx.lang;
        let heading = fill(
            lang.tr("Connect {provider}"),
            &[("provider", &provider.display_name)],
        );
        let body = flow_body(&provider.id, &ctx.logo_color, &heading);
        let label = adw::EntryRow::builder()
            .title(lang.tr("Label (optional)"))
            .build();
        let widgets = Widgets {
            description: body.description,
            stack: body.stack,
            key: adw::PasswordEntryRow::builder()
                .title(key_label(ctx, method))
                .use_markup(false)
                .build(),
            label,
            add: gtk::Button::new(),
            error: wrapping("", &["error"]),
        };
        widgets.label.set_visible(!target.is_login());
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
        page
    }

    fn assemble(self: &Rc<Self>) {
        let (lang, widgets) = (self.ctx.lang, &self.widgets);
        widgets.stack.add_named(&self.form_page(), Some(FORM));
        let close = Rc::clone(&self.ctx.close);
        let cancel = pill(lang.tr("Cancel"), false, move || close());
        let checking = busy_line(&gtk::Label::new(Some(lang.tr("Checking the key…"))));
        let progress = stack_of(&[checking.upcast_ref(), cancel.upcast_ref()]);
        widgets.stack.add_named(&progress, Some(PROGRESS));
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

    fn form_page(self: &Rc<Self>) -> gtk::Box {
        let (lang, widgets) = (self.ctx.lang, &self.widgets);
        let group = adw::PreferencesGroup::builder()
            .description(hint(&self.method))
            .build();
        group.add(&widgets.key);
        group.add(&widgets.label);
        for entry in [
            widgets.key.upcast_ref::<gtk::Editable>(),
            widgets.label.upcast_ref(),
        ] {
            let weak = Rc::downgrade(self);
            entry.connect_changed(move |_| with(&weak, |page| page.sync_add()));
        }
        let weak = Rc::downgrade(self);
        widgets
            .key
            .connect_entry_activated(move |_| with(&weak, Self::start));
        widgets.add.set_label(lang.tr("Add"));
        widgets.add.add_css_class("pill");
        widgets.add.add_css_class("suggested-action");
        let weak = Rc::downgrade(self);
        widgets
            .add
            .connect_clicked(move |_| with(&weak, Self::start));
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        buttons.set_halign(gtk::Align::Center);
        if let Some(url) = console_url(&self.method) {
            let weak = Rc::downgrade(self);
            let get_key = pill(lang.tr("Get a Key"), false, move || {
                with(&weak, |page| page.open(&url));
            });
            buttons.append(&get_key);
        }
        buttons.append(&widgets.add);
        stack_of(&[group.upcast_ref(), buttons.upcast_ref()])
    }

    fn sync_add(&self) {
        let ready = !self.widgets.key.text().trim().is_empty();
        self.widgets.add.set_sensitive(ready);
    }

    fn start(self: &Rc<Self>) {
        let key = self.widgets.key.text().trim().to_owned();
        if key.is_empty() || self.process.borrow().is_some() {
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
            Ok(process) => {
                process.send_last_line(&key);
                *self.process.borrow_mut() = Some(process);
            }
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

    fn open(&self, url: &str) {
        let window = self.page.root().and_downcast::<gtk::Window>();
        gtk::UriLauncher::new(url).launch(window.as_ref(), gtk::gio::Cancellable::NONE, |result| {
            if let Err(error) = result {
                tracing::warn!(%error, "could not open the key page");
            }
        });
    }

    fn render(&self) {
        let (lang, widgets) = (self.ctx.lang, &self.widgets);
        let phase = self.flow.borrow().phase.clone();
        let (child, description) = match phase {
            Phase::Form => (
                FORM,
                fill(
                    lang.tr("Headroom checks the key with {provider} and keeps it in your keyring. It never leaves this computer."),
                    &[("provider", &self.provider.display_name)],
                ),
            ),
            Phase::Running => (PROGRESS, lang.tr("This takes a moment.").to_owned()),
            Phase::Done => {
                widgets.key.set_text("");
                (DONE, self.target.done_text(lang).to_owned())
            }
            Phase::Failed(message) => (ERROR, message),
        };
        widgets.stack.set_visible_child_name(child);
        widgets
            .description
            .set_text(if child == ERROR { "" } else { &description });
        widgets.error.set_text(&description);
        self.sync_add();
    }
}
