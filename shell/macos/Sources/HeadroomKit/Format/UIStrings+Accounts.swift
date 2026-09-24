public enum AccountsText: LocalizedText {
    case accounts, accountsFooter, noAccounts, noAccountsDetail, selectAccount, addAccountEllipsis
    case showInMenuBar, label, saveLabel, limits, limitsFooter, addedInHeadroom

    public var translations: (english: String, russian: String) {
        switch self {
        case .accounts: ("Accounts", "Аккаунты")
        case .accountsFooter:
            (
                "Drag to reorder. Hidden accounts keep updating but leave the menu bar and notifications.",
                "Перетащите, чтобы изменить порядок. Скрытые аккаунты продолжают обновляться, но не появляются в строке меню и в уведомлениях."
            )
        case .noAccounts: ("No accounts yet", "Аккаунтов пока нет")
        case .noAccountsDetail:
            (
                "Sign in with a supported CLI, or add an account.",
                "Войдите через поддерживаемый CLI или добавьте аккаунт."
            )
        case .selectAccount: ("Select an account", "Выберите аккаунт")
        case .addAccountEllipsis: ("Add Account…", "Добавить аккаунт…")
        case .showInMenuBar: ("Show in the menu bar and popup", "Показывать в строке меню и во всплывающем окне")
        case .label: ("Label", "Название")
        case .saveLabel: ("Save", "Сохранить")
        case .limits: ("Limits", "Лимиты")
        case .limitsFooter:
            (
                "Hidden limits leave the popup, the menu bar and notifications.",
                "Скрытые лимиты не появляются во всплывающем окне, в строке меню и в уведомлениях."
            )
        case .addedInHeadroom: ("added in Headroom", "добавлен в Headroom")
        }
    }
}

public enum RemovalText: LocalizedText {
    case removeFromHeadroom, removeHeadroomDetail, removeCLIDetail, removeEllipsis, remove, cancel
    case removeTitle, removeHeadroomBody, removeCLIBody, accountRemoved, removing

    public var translations: (english: String, russian: String) {
        switch self {
        case .removeFromHeadroom: ("Remove from Headroom", "Удалить из Headroom")
        case .removeHeadroomDetail:
            (
                "Deletes the sign-in Headroom created for this account",
                "Удаляет вход, созданный Headroom для этого аккаунта"
            )
        case .removeCLIDetail:
            (
                "Stops showing this account. Its CLI stays signed in.",
                "Аккаунт перестанет отображаться. Вход в CLI сохранится."
            )
        case .removeEllipsis: ("Remove…", "Удалить…")
        case .remove: ("Remove", "Удалить")
        case .cancel: ("Cancel", "Отмена")
        case .removeTitle: ("Remove {name}?", "Удалить «{name}»?")
        case .removeHeadroomBody:
            (
                "Headroom deletes the sign-in it created for this account. The account itself is not affected.",
                "Headroom удалит вход, который создал для этого аккаунта. Сам аккаунт не пострадает."
            )
        case .removeCLIBody:
            (
                "Headroom will stop showing this account. The {provider} CLI stays signed in; you can sign in again through Headroom.",
                "Headroom перестанет показывать этот аккаунт. Вход в CLI {provider} сохранится; войти снова можно через Headroom."
            )
        case .accountRemoved: ("Account removed", "Аккаунт удалён")
        case .removing: ("Removing…", "Удаление…")
        }
    }
}
