pub(super) const CATALOG: &[(&str, &str)] = &[
    (
        "Hidden from the tray and popup",
        "Скрыт из трея и всплывающего окна",
    ),
    (
        "Stays expanded when unstarred accounts collapse",
        "Не сворачивается вместе с аккаунтами без звезды",
    ),
    ("Limits", "Лимиты"),
    (
        "Hidden limits leave the popup, the tray and notifications.",
        "Скрытые лимиты не появляются во всплывающем окне, в трее и в уведомлениях.",
    ),
    ("Links", "Ссылки"),
    (
        "Order in the tray and popup",
        "Порядок в трее и во всплывающем окне",
    ),
    ("Remove…", "Удалить…"),
    ("Show models and projects", "Показывать модели и проекты"),
    (
        "Model and project lists in the spend card",
        "Списки моделей и проектов в карточке трат",
    ),
];

#[cfg(test)]
mod tests {
    use crate::i18n::Lang;

    #[test]
    fn accounts_page_strings_are_translated() {
        let strings = [
            "Accounts",
            "Add Account…",
            "Always open in the popup",
            "Hidden accounts keep updating but leave the tray and notifications.",
            "Label",
            "Move up",
            "Move down",
            "No accounts yet",
            "Position",
            "Remove from Headroom",
            "Show in the tray and popup",
            "Sign in again…",
            "Sign in again to keep this account up to date.",
            "Sign in with a supported CLI, or add an account.",
            "Status page",
            "Limits",
            "Links",
            "Remove…",
            "Show models and projects",
        ];
        for text in strings {
            assert_ne!(Lang::Ru.tr(text), text, "{text} is not translated");
        }
        assert_eq!(
            Lang::Ru.tr("Show models and projects"),
            "Показывать модели и проекты"
        );
    }
}
