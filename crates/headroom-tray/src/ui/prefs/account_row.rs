use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;

use super::{Act, PrefsAction};
use crate::assets::{CLAUDE_COLOR, Tint, provider_logo};
use crate::i18n::Lang;
use crate::payload::Account;
use crate::preferences::change::Change;
use crate::preferences::choices::{account_name, account_subtitle, removal_subtitle};
use crate::preferences::model::DisplaySettings;
use crate::ui::widgets::svg_image;

const LOGO_SIZE: i32 = 24;

pub type Mover = Rc<dyn Fn(&str, isize)>;
pub type Remover = Rc<dyn Fn(&Account)>;

pub struct AccountRow {
    pub widget: adw::ExpanderRow,
    pub id: String,
    pub shape: Vec<(String, String)>,
    visible: gtk::Switch,
    label: adw::EntryRow,
    windows: Vec<(String, adw::SwitchRow)>,
    syncing: Rc<Cell<bool>>,
    display: Rc<RefCell<DisplaySettings>>,
    account: Rc<RefCell<Account>>,
}

pub fn provider_image(provider: &str, text_color: &str, size: i32) -> gtk::Image {
    let (svg, tint) = provider_logo(provider);
    let color = match tint {
        Tint::Brand => CLAUDE_COLOR,
        Tint::Text => text_color,
    };
    svg_image(svg, color, size, &[])
}

fn icon_button(icon: &str, tooltip: &str, on_click: impl Fn() + 'static) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.set_tooltip_text(Some(tooltip));
    button.set_valign(gtk::Align::Center);
    button.connect_clicked(move |_| on_click());
    button
}

#[must_use]
pub fn shape(account: &Account) -> Vec<(String, String)> {
    let head = (format!("{:?}", account.owner), account.provider.clone());
    std::iter::once(head)
        .chain(
            account
                .windows
                .iter()
                .map(|window| (window.id.clone(), window.label.clone())),
        )
        .collect()
}

fn window_rows(account: &Account) -> Vec<(String, adw::SwitchRow)> {
    account
        .windows
        .iter()
        .map(|window| {
            let row = adw::SwitchRow::builder()
                .title(&window.label)
                .use_markup(false)
                .build();
            (window.id.clone(), row)
        })
        .collect()
}

impl AccountRow {
    pub fn new(lang: Lang, account: &Account, logo_color: &str) -> Self {
        let widget = adw::ExpanderRow::builder().use_markup(false).build();
        widget.add_prefix(&provider_image(&account.provider, logo_color, LOGO_SIZE));
        let visible = gtk::Switch::new();
        visible.set_valign(gtk::Align::Center);
        visible.set_tooltip_text(Some(lang.tr("Show in the tray and popup")));
        widget.add_suffix(&visible);
        let label = adw::EntryRow::builder()
            .title(lang.tr("Label"))
            .show_apply_button(true)
            .build();
        Self {
            widget,
            id: account.id.clone(),
            shape: shape(account),
            visible,
            label,
            windows: window_rows(account),
            syncing: Rc::default(),
            display: Rc::default(),
            account: Rc::new(RefCell::new(account.clone())),
        }
    }

    pub fn assemble(&self, lang: Lang, act: &Act, mover: &Mover, remover: &Remover) {
        self.connect_visibility(act);
        self.connect_label(act);
        self.widget.add_row(&self.label);
        for (window_id, switch) in &self.windows {
            self.connect_window(act, window_id, switch);
            self.widget.add_row(switch);
        }
        self.widget.add_row(&self.position_row(lang, mover));
        self.widget.add_row(&self.remove_row(lang, remover));
    }

    fn connect_visibility(&self, act: &Act) {
        let (act, id, syncing) = (Rc::clone(act), self.id.clone(), Rc::clone(&self.syncing));
        self.visible.connect_active_notify(move |switch| {
            if !syncing.get() {
                act(PrefsAction::SetHidden {
                    account_id: id.clone(),
                    hidden: !switch.is_active(),
                });
            }
        });
    }

    fn connect_label(&self, act: &Act) {
        let (act, id) = (Rc::clone(act), self.id.clone());
        self.label.connect_apply(move |entry| {
            act(PrefsAction::SetLabel {
                account_id: id.clone(),
                label: entry.text().trim().to_owned(),
            });
        });
    }

    fn connect_window(&self, act: &Act, window_id: &str, switch: &adw::SwitchRow) {
        let act = Rc::clone(act);
        let (id, window) = (self.id.clone(), window_id.to_owned());
        let (syncing, display) = (Rc::clone(&self.syncing), Rc::clone(&self.display));
        switch.connect_active_notify(move |switch| {
            if syncing.get() {
                return;
            }
            let windows = display
                .borrow()
                .hidden_windows_after(&id, &window, !switch.is_active());
            act(PrefsAction::Change(Change::HiddenWindows {
                account_id: id.clone(),
                windows,
            }));
        });
    }

    fn position_row(&self, lang: Lang, mover: &Mover) -> adw::ActionRow {
        let row = adw::ActionRow::builder().title(lang.tr("Position")).build();
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        buttons.add_css_class("linked");
        buttons.set_valign(gtk::Align::Center);
        for (icon, tooltip, delta) in [
            ("go-up-symbolic", lang.tr("Move up"), -1),
            ("go-down-symbolic", lang.tr("Move down"), 1),
        ] {
            let (mover, id) = (Rc::clone(mover), self.id.clone());
            buttons.append(&icon_button(icon, tooltip, move || mover(&id, delta)));
        }
        row.add_suffix(&buttons);
        row
    }

    fn remove_row(&self, lang: Lang, remover: &Remover) -> adw::ActionRow {
        let owner = self.account.borrow().owner;
        let row = adw::ActionRow::builder()
            .title(lang.tr("Remove from Headroom"))
            .subtitle(removal_subtitle(lang, owner))
            .build();
        let button = gtk::Button::with_label(lang.tr("Remove"));
        button.add_css_class("destructive-action");
        button.set_valign(gtk::Align::Center);
        let (remover, account) = (Rc::clone(remover), Rc::clone(&self.account));
        button.connect_clicked(move |_| remover(&account.borrow()));
        row.add_suffix(&button);
        row
    }

    pub fn update(&self, lang: Lang, account: &Account, display: &DisplaySettings) {
        self.syncing.set(true);
        self.widget.set_title(&account_name(account));
        self.widget.set_subtitle(&account_subtitle(lang, account));
        self.visible.set_active(!account.hidden);
        let editing = self
            .label
            .state_flags()
            .contains(gtk::StateFlags::FOCUS_WITHIN);
        let label = account.label.as_deref().unwrap_or_default();
        if !editing && self.label.text() != label {
            self.label.set_text(label);
        }
        for (window_id, switch) in &self.windows {
            switch.set_active(!display.is_window_hidden(&account.id, window_id));
        }
        *self.display.borrow_mut() = display.clone();
        *self.account.borrow_mut() = account.clone();
        self.syncing.set(false);
    }
}
