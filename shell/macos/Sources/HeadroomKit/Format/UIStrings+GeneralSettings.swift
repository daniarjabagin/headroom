public enum LayoutSettingsText: LocalizedText {
    case timeFormat, timeFormatDetail, density, densityDetail

    public var translations: (english: String, russian: String) {
        switch self {
        case .timeFormat: ("Time format", "Формат времени")
        case .timeFormatDetail: ("Exact reset times and chart labels", "Точное время сброса и подписи графиков")
        case .density: ("Density", "Плотность")
        case .densityDetail: ("Compact fits more accounts in the popup", "Компактная вмещает больше аккаунтов")
        }
    }
}

public enum MenuBarSettingsText: LocalizedText {
    case menuBarShows, menuBarShowsDetail, limitsInMenuBar, limitsInMenuBarDetail, mostCritical, indicatorStyle,
        indicatorStyleDetail, dragHint

    public var translations: (english: String, russian: String) {
        switch self {
        case .menuBarShows: ("Menu bar shows", "В строке меню")
        case .menuBarShowsDetail: ("What sits in the menu bar", "Что показывать в строке меню")
        case .limitsInMenuBar: ("Limits in the menu bar", "Лимиты в строке меню")
        case .limitsInMenuBarDetail: ("Up to 3, in this order", "До трёх, в этом порядке")
        case .mostCritical: ("Most critical", "Самые критичные")
        case .indicatorStyle: ("Indicator style", "Индикатор")
        case .indicatorStyleDetail: ("The mark in front of each figure", "Значок перед каждым числом")
        case .dragHint:
            (
                "⌘-drag items in the menu bar to move them.",
                "Чтобы переместить значки в строке меню, перетащите их с нажатой ⌘."
            )
        }
    }
}

public enum SpendSettingsText: LocalizedText {
    case spend, showSpend, showSpendDetail, defaultPeriod, defaultPeriodDetail, units, breakdown, breakdownDetail

    public var translations: (english: String, russian: String) {
        switch self {
        case .spend: ("Spend", "Расходы")
        case .showSpend: ("Show spend", "Показывать расходы")
        case .showSpendDetail: ("Spend ring for all tools at the top", "Кольцо расходов по всем инструментам сверху")
        case .defaultPeriod: ("Default period", "Период по умолчанию")
        case .defaultPeriodDetail: ("The popup opens on this period", "Всплывающее окно открывается на этом периоде")
        case .units: ("Units", "Единицы")
        case .breakdown: ("Breakdown on hover", "Разбивка при наведении")
        case .breakdownDetail: ("What a legend entry splits into", "На что делится строка легенды")
        }
    }
}

public enum CardsSettingsText: LocalizedText {
    case popupCards, collapseUnstarred, collapseUnstarredDetail, alwaysOpen, onDemand, starFooter

    public var translations: (english: String, russian: String) {
        switch self {
        case .popupCards: ("Popup Cards", "Карточки")
        case .collapseUnstarred: ("Collapse unstarred accounts", "Сворачивать аккаунты без звёздочки")
        case .collapseUnstarredDetail:
            ("They open on their own when a limit runs low", "Они раскрываются сами, когда лимит на исходе")
        case .alwaysOpen: ("Always open", "Всегда открыт")
        case .onDemand: ("On demand", "По запросу")
        case .starFooter:
            ("Starred accounts always stay open in the popup.", "Аккаунты со звёздочкой всегда раскрыты.")
        }
    }
}

public enum RefreshSettingsText: LocalizedText {
    case dataRefresh, adaptive, adaptiveDetail

    public var translations: (english: String, russian: String) {
        switch self {
        case .dataRefresh: ("Data Refresh", "Обновление данных")
        case .adaptive: ("Faster while coding tools run", "Чаще, пока работают ИИ-инструменты")
        case .adaptiveDetail:
            (
                "Every minute while Claude Code, Codex or Cursor is running",
                "Раз в минуту, пока запущены Claude Code, Codex или Cursor"
            )
        }
    }
}

public enum PrivacySettingsText: LocalizedText {
    case privacy, hideOnShare, hideOnShareDetail, statusPages, statusPagesDetail

    public var translations: (english: String, russian: String) {
        switch self {
        case .privacy: ("Privacy", "Конфиденциальность")
        case .hideOnShare: ("Hide while screen sharing", "Скрывать при демонстрации экрана")
        case .hideOnShareDetail: ("Shows the Headroom symbol instead of figures", "Вместо чисел — значок Headroom")
        case .statusPages: ("Status pages", "Страницы статуса")
        case .statusPagesDetail:
            ("Show provider incidents from public status pages", "Показывать сбои с публичных страниц статуса")
        }
    }
}

public enum KeyboardSettingsText: LocalizedText {
    case keyboard, openHeadroom, openHeadroomDetail, recordShortcut, pressShortcut, clearShortcut, needsModifier,
        unsupportedKey

    public var translations: (english: String, russian: String) {
        switch self {
        case .keyboard: ("Keyboard", "Клавиатура")
        case .openHeadroom: ("Open Headroom", "Открыть Headroom")
        case .openHeadroomDetail: ("Opens the popup from any app", "Открывает окно из любого приложения")
        case .recordShortcut: ("Record Shortcut", "Задать сочетание")
        case .pressShortcut: ("Press shortcut…", "Нажмите сочетание…")
        case .clearShortcut: ("Clear shortcut", "Удалить сочетание")
        case .needsModifier: ("Add ⌘, ⌥ or ⌃ to the key", "Добавьте к клавише ⌘, ⌥ или ⌃")
        case .unsupportedKey: ("This key can't be used", "Эту клавишу нельзя использовать")
        }
    }
}
