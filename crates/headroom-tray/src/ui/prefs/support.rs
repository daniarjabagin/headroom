use std::rc::Rc;

use adw::prelude::*;

use super::rows::action_row;
use super::{Act, PrefsAction};
use crate::i18n::Lang;

pub const REPOSITORY_URL: &str = "https://github.com/daniarjabagin/headroom";
const LINK_ICON: &str = "adw-external-link-symbolic";
const GROUP_TITLE: &str = "Support Headroom";
const ROW_TITLE: &str = "Star Headroom on GitHub";
const ROW_SUBTITLE: &str = "Stars help other people find it. It's free and takes a second.";
const BUTTON_LABEL: &str = "Open GitHub";

fn open_button(lang: Lang, act: &Act) -> gtk::Button {
    let content = adw::ButtonContent::builder()
        .label(lang.tr(BUTTON_LABEL))
        .icon_name(LINK_ICON)
        .build();
    let button = gtk::Button::builder()
        .child(&content)
        .valign(gtk::Align::Center)
        .tooltip_text(REPOSITORY_URL)
        .build();
    let act = Rc::clone(act);
    button.connect_clicked(move |_| act(PrefsAction::OpenUrl(REPOSITORY_URL.to_owned())));
    button
}

pub fn support_group(lang: Lang, act: &Act) -> adw::PreferencesGroup {
    let row = action_row(lang.tr(ROW_TITLE), lang.tr(ROW_SUBTITLE));
    row.add_suffix(&open_button(lang, act));
    let group = adw::PreferencesGroup::builder()
        .title(lang.tr(GROUP_TITLE))
        .build();
    group.add(&row);
    group
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_url_has_no_tracking_parameters() {
        assert_eq!(REPOSITORY_URL, "https://github.com/daniarjabagin/headroom");
        assert!(!REPOSITORY_URL.contains('?'));
    }

    #[test]
    fn support_strings_are_translated_to_russian() {
        let expected = [
            (GROUP_TITLE, "Поддержать Headroom"),
            (ROW_TITLE, "Поставьте звезду на GitHub"),
            (
                ROW_SUBTITLE,
                "Звёзды помогают другим найти Headroom. Это бесплатно и занимает секунду.",
            ),
            (BUTTON_LABEL, "Открыть GitHub"),
        ];
        for (msgid, russian) in expected {
            assert_eq!(Lang::Ru.tr(msgid), russian);
            assert_eq!(Lang::En.tr(msgid), msgid);
        }
    }
}
