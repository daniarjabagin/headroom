use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;

use crate::preferences::choices::Choice;

#[derive(Clone, Default)]
struct Guard(Rc<Cell<bool>>);

impl Guard {
    fn quietly(&self, update: impl FnOnce()) {
        self.0.set(true);
        update();
        self.0.set(false);
    }

    fn active(&self) -> bool {
        !self.0.get()
    }
}

pub struct SwitchRow {
    pub row: adw::SwitchRow,
    guard: Guard,
}

impl SwitchRow {
    pub fn new(title: &str, subtitle: &str, on_change: impl Fn(bool) + 'static) -> Self {
        let row = adw::SwitchRow::builder()
            .title(title)
            .subtitle(subtitle)
            .use_markup(false)
            .build();
        let guard = Guard::default();
        let watch = guard.clone();
        row.connect_active_notify(move |row| {
            if watch.active() {
                on_change(row.is_active());
            }
        });
        Self { row, guard }
    }

    pub fn set(&self, active: bool) {
        if self.row.is_active() != active {
            self.guard.quietly(|| self.row.set_active(active));
        }
    }
}

pub struct SegmentedRow<T> {
    pub row: adw::ActionRow,
    buttons: Vec<(T, gtk::ToggleButton)>,
    guard: Guard,
}

impl<T: Clone + PartialEq + 'static> SegmentedRow<T> {
    pub fn new(
        title: &str,
        subtitle: &str,
        options: Vec<Choice<T>>,
        on_change: impl Fn(T) + 'static,
    ) -> Self {
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(subtitle)
            .use_markup(false)
            .build();
        let group = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        group.add_css_class("linked");
        group.set_valign(gtk::Align::Center);
        let guard = Guard::default();
        let on_change: Rc<dyn Fn(T)> = Rc::new(on_change);
        let mut buttons: Vec<(T, gtk::ToggleButton)> = Vec::new();
        for option in options {
            let button = gtk::ToggleButton::with_label(&option.label);
            if let Some((_, first)) = buttons.first() {
                button.set_group(Some(first));
            }
            let (watch, notify, value) =
                (guard.clone(), Rc::clone(&on_change), option.value.clone());
            button.connect_toggled(move |button| {
                if button.is_active() && watch.active() {
                    notify(value.clone());
                }
            });
            group.append(&button);
            buttons.push((option.value, button));
        }
        row.add_suffix(&group);
        Self {
            row,
            buttons,
            guard,
        }
    }

    pub fn set(&self, value: &T) {
        if let Some((_, button)) = self.buttons.iter().find(|(option, _)| option == value)
            && !button.is_active()
        {
            self.guard.quietly(|| button.set_active(true));
        }
    }
}

pub struct ComboRow<T> {
    pub row: adw::ComboRow,
    values: Rc<RefCell<Vec<T>>>,
    labels: RefCell<Vec<String>>,
    guard: Guard,
}

impl<T: Clone + PartialEq + 'static> ComboRow<T> {
    pub fn new(title: &str, subtitle: &str, on_change: impl Fn(T) + 'static) -> Self {
        let row = adw::ComboRow::builder()
            .title(title)
            .subtitle(subtitle)
            .use_markup(false)
            .build();
        let values: Rc<RefCell<Vec<T>>> = Rc::default();
        let guard = Guard::default();
        let (watch, known) = (guard.clone(), Rc::clone(&values));
        row.connect_selected_notify(move |row| {
            let index = usize::try_from(row.selected()).ok();
            let value = index.and_then(|index| known.borrow().get(index).cloned());
            if let Some(value) = value.filter(|_| watch.active()) {
                on_change(value);
            }
        });
        Self {
            row,
            values,
            labels: RefCell::default(),
            guard,
        }
    }

    pub fn set(&self, choices: Vec<Choice<T>>, current: &T) {
        let labels: Vec<String> = choices.iter().map(|choice| choice.label.clone()).collect();
        let index = choices
            .iter()
            .position(|choice| choice.value == *current)
            .and_then(|index| u32::try_from(index).ok())
            .unwrap_or(0);
        self.guard.quietly(|| {
            if *self.labels.borrow() != labels {
                let names: Vec<&str> = labels.iter().map(String::as_str).collect();
                *self.values.borrow_mut() =
                    choices.into_iter().map(|choice| choice.value).collect();
                self.row.set_model(Some(&gtk::StringList::new(&names)));
                *self.labels.borrow_mut() = labels;
            }
            if self.row.selected() != index {
                self.row.set_selected(index);
            }
        });
    }
}

pub fn group(title: &str, description: &str, rows: &[&gtk::Widget]) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title(title)
        .description(description)
        .build();
    for row in rows {
        group.add(*row);
    }
    group
}

pub fn pill_button(label: &str, suggested: bool, on_click: impl Fn() + 'static) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class("pill");
    if suggested {
        button.add_css_class("suggested-action");
    }
    button.set_halign(gtk::Align::Center);
    button.connect_clicked(move |_| on_click());
    button
}
