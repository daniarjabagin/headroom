use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;

use super::found::{Found, FoundRow, found_rows};
use super::{Ctx, page_column, title_block};
use crate::payload::State;
use crate::preferences::registry::ProviderInfo;
use crate::ui::prefs::{PrefsAction, pill_button, provider_image};

const LOGO_SIZE: i32 = 32;

struct Line {
    shape: FoundRow,
    row: adw::ActionRow,
    switch: Option<gtk::Switch>,
}

pub struct Welcome {
    pub widget: gtk::ScrolledWindow,
    list: adw::PreferencesGroup,
    add: adw::ActionRow,
    rows: RefCell<Vec<Line>>,
    syncing: Rc<Cell<bool>>,
    ctx: Ctx,
}

fn without_hidden(row: &FoundRow) -> FoundRow {
    let mut shape = row.clone();
    if let Found::SignedIn { hidden, .. } = &mut shape.found {
        *hidden = false;
    }
    shape
}

fn add_row(ctx: &Ctx) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(ctx.lang.tr("Add another account…"))
        .activatable(true)
        .build();
    row.add_prefix(&gtk::Image::from_icon_name("list-add-symbolic"));
    row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    let on_add = Rc::clone(&ctx.on_add);
    row.connect_activated(move |_| on_add(None));
    row
}

fn later_button(ctx: &Ctx) -> gtk::Button {
    let later = gtk::Button::with_label(ctx.lang.tr("Choose later"));
    later.add_css_class("link");
    later.set_halign(gtk::Align::Center);
    let on_later = Rc::clone(&ctx.on_later);
    later.connect_clicked(move |_| on_later());
    later
}

impl Welcome {
    pub fn new(ctx: &Ctx) -> Self {
        let lang = ctx.lang;
        let column = page_column();
        column.append(&title_block(
            ctx,
            lang.tr("Welcome to Headroom"),
            lang.tr("Your AI coding limits, right in your tray."),
        ));
        let list = adw::PreferencesGroup::builder()
            .title(lang.tr("Here’s what we found"))
            .description(lang.tr("Turn off anything you don’t want to track."))
            .build();
        let add = add_row(ctx);
        list.add(&add);
        column.append(&list);
        let on_start = Rc::clone(&ctx.on_start);
        let start = pill_button(lang.tr("Start"), true, move || on_start());
        column.append(&start);
        column.append(&later_button(ctx));
        let widget = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&column)
            .build();
        Self {
            widget,
            list,
            add,
            rows: RefCell::default(),
            syncing: Rc::default(),
            ctx: ctx.clone(),
        }
    }

    pub fn update(&self, state: &State, providers: &[ProviderInfo]) {
        let found: Vec<FoundRow> = found_rows(self.ctx.lang, state, providers);
        let shape: Vec<FoundRow> = found.iter().map(without_hidden).collect();
        let known: Vec<FoundRow> = self
            .rows
            .borrow()
            .iter()
            .map(|row| row.shape.clone())
            .collect();
        if known != shape {
            self.rebuild(&found);
        }
        self.syncing.set(true);
        for (row, item) in self.rows.borrow().iter().zip(&found) {
            if let (Some(switch), Found::SignedIn { hidden, .. }) = (&row.switch, &item.found) {
                switch.set_active(!hidden);
            }
        }
        self.syncing.set(false);
    }

    fn rebuild(&self, found: &[FoundRow]) {
        for old in self.rows.take() {
            self.list.remove(&old.row);
        }
        self.list.remove(&self.add);
        let rows: Vec<Line> = found.iter().map(|item| self.new_row(item)).collect();
        for row in &rows {
            self.list.add(&row.row);
        }
        self.list.add(&self.add);
        *self.rows.borrow_mut() = rows;
    }

    fn new_row(&self, item: &FoundRow) -> Line {
        let row = adw::ActionRow::builder()
            .title(&item.title)
            .subtitle(&item.subtitle)
            .use_markup(false)
            .build();
        row.add_prefix(&provider_image(
            &item.provider,
            &self.ctx.logo_color,
            LOGO_SIZE,
        ));
        let switch = match &item.found {
            Found::SignedIn { account_id, hidden } => {
                Some(self.visibility_switch(&row, account_id, *hidden))
            }
            Found::SignedOut => {
                self.sign_in_button(&row, &item.provider);
                None
            }
            Found::NotInstalled => {
                row.set_sensitive(false);
                let switch = gtk::Switch::new();
                switch.set_valign(gtk::Align::Center);
                row.add_suffix(&switch);
                None
            }
        };
        Line {
            shape: without_hidden(item),
            row,
            switch,
        }
    }

    fn visibility_switch(
        &self,
        row: &adw::ActionRow,
        account_id: &str,
        hidden: bool,
    ) -> gtk::Switch {
        let switch = gtk::Switch::new();
        switch.set_valign(gtk::Align::Center);
        switch.set_active(!hidden);
        row.add_suffix(&switch);
        row.set_activatable_widget(Some(&switch));
        let (act, id) = (Rc::clone(&self.ctx.act), account_id.to_owned());
        let syncing = Rc::clone(&self.syncing);
        switch.connect_active_notify(move |switch| {
            if syncing.get() {
                return;
            }
            act(PrefsAction::SetHidden {
                account_id: id.clone(),
                hidden: !switch.is_active(),
            });
        });
        switch
    }

    fn sign_in_button(&self, row: &adw::ActionRow, provider: &str) {
        let button = gtk::Button::with_label(self.ctx.lang.tr("Sign In"));
        button.set_valign(gtk::Align::Center);
        let (on_add, provider) = (Rc::clone(&self.ctx.on_add), provider.to_owned());
        button.connect_clicked(move |_| on_add(Some(provider.clone())));
        row.add_suffix(&button);
    }
}
