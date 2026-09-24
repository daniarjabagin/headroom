public protocol LocalizedText: Sendable, CaseIterable {
    var translations: (english: String, russian: String) { get }
}

public enum SettingsText: LocalizedText {
    case windowTitle, general, notifications, service, refreshNow, settingsMenu, settingsUnavailable,
        settingsUnavailableDetail

    public var translations: (english: String, russian: String) {
        switch self {
        case .windowTitle: ("Headroom Settings", "Настройки Headroom")
        case .general: ("General", "Общие")
        case .notifications: ("Notifications", "Уведомления")
        case .service: ("Service", "Служба")
        case .refreshNow: ("Refresh Now", "Обновить сейчас")
        case .settingsMenu: ("Settings…", "Настройки…")
        case .settingsUnavailable: ("Headroom service isn't running", "Служба Headroom не запущена")
        case .settingsUnavailableDetail:
            (
                "Settings live in the Headroom service. Headroom restarts it on its own; the Service tab shows why it stopped.",
                "Настройки хранятся в службе Headroom. Headroom перезапускает её сам; причина остановки — на вкладке «Служба»."
            )
        }
    }
}

public enum MenuText: LocalizedText {
    case edit, undo, redo, cut, copy, paste, selectAll, window, minimize, closeWindow

    public var translations: (english: String, russian: String) {
        switch self {
        case .edit: ("Edit", "Правка")
        case .undo: ("Undo", "Отменить")
        case .redo: ("Redo", "Повторить")
        case .cut: ("Cut", "Вырезать")
        case .copy: ("Copy", "Скопировать")
        case .paste: ("Paste", "Вставить")
        case .selectAll: ("Select All", "Выбрать все")
        case .window: ("Window", "Окно")
        case .minimize: ("Minimize", "Свернуть")
        case .closeWindow: ("Close Window", "Закрыть окно")
        }
    }
}

public enum AppearanceText: LocalizedText {
    case appearance, theme, system, light, dark, language, translucent, translucentDetail, reducedMotion,
        reducedMotionDetail, popup, valueMode, valueModeDetail, left, used, resetFormat, resetFormatDetail, countdown,
        exactTime

    public var translations: (english: String, russian: String) {
        switch self {
        case .appearance: ("Appearance", "Внешний вид")
        case .theme: ("Theme", "Тема")
        case .system: ("System", "Система")
        case .light: ("Light", "Светлая")
        case .dark: ("Dark", "Тёмная")
        case .language: ("Language", "Язык")
        case .translucent: ("Translucent background", "Полупрозрачный фон")
        case .translucentDetail: ("Blur what is behind the popup", "Размывать то, что под всплывающим окном")
        case .reducedMotion: ("Reduce motion", "Меньше анимации")
        case .reducedMotionDetail: ("Turn off popup animations", "Отключить анимацию всплывающего окна")
        case .popup: ("Popup", "Всплывающее окно")
        case .valueMode: ("Show values as", "Показывать значения")
        case .valueModeDetail: ("Click a reading in the popup to switch", "Переключается нажатием на значение")
        case .left: ("Left", "Остаток")
        case .used: ("Used", "Расход")
        case .resetFormat: ("Reset time", "Время сброса")
        case .resetFormatDetail: ("Click a reset time in the popup to switch", "Переключается нажатием на время")
        case .countdown: ("Countdown", "Таймер")
        case .exactTime: ("Exact time", "Время")
        }
    }
}

public enum MenuBarText: LocalizedText {
    case menuBar, menuBarLimit, menuBarLimitDetail, autoMostCritical, pinnedUnavailable, menuBarLabel, percent,
        providerAndLimit, sections, totalSpend, totalSpendDetail, accountSpend, accountSpendDetail, usageTrend,
        usageTrendDetail, paceForecast, paceForecastDetail, updates, refreshInterval, refreshIntervalDetail, everyMinute

    public var translations: (english: String, russian: String) {
        switch self {
        case .menuBar: ("Menu Bar", "Строка меню")
        case .menuBarLimit: ("Menu bar limit", "Лимит в строке меню")
        case .menuBarLimitDetail: ("The limit shown in the menu bar", "Лимит, который показан в строке меню")
        case .autoMostCritical: ("Auto — most critical", "Самый критичный")
        case .pinnedUnavailable: ("Pinned limit (not available now)", "Закреплённый лимит (сейчас недоступен)")
        case .menuBarLabel: ("Menu bar label", "Подпись в строке меню")
        case .percent: ("Percent", "Проценты")
        case .providerAndLimit: ("Provider + limit", "Провайдер + лимит")
        case .sections: ("Sections", "Разделы")
        case .totalSpend: ("Total spend", "Всего потрачено")
        case .totalSpendDetail: ("Spend ring for all tools at the top", "Кольцо расходов по всем инструментам сверху")
        case .accountSpend: ("Per-account spend", "Расходы по аккаунтам")
        case .accountSpendDetail:
            ("Today, yesterday and 30 days under each account", "Сегодня, вчера и за 30 дней под каждым аккаунтом")
        case .usageTrend: ("Usage trend", "Динамика использования")
        case .usageTrendDetail: ("Daily token bars for the last 30 days", "Токены по дням за последние 30 дней")
        case .paceForecast: ("Pace forecast", "Прогноз по темпу")
        case .paceForecastDetail:
            ("Where each limit lands at the current pace", "Где окажется каждый лимит при текущем темпе")
        case .updates: ("Updates", "Обновление")
        case .refreshInterval: ("Refresh interval", "Интервал обновления")
        case .refreshIntervalDetail:
            ("How often the service asks each provider", "Как часто служба опрашивает каждого провайдера")
        case .everyMinute: ("Every minute", "Каждую минуту")
        }
    }
}

public enum NotificationText: LocalizedText {
    case notifyMeWhen, notificationsFooter, almostOut, almostOutDetail, cuttingItClose, cuttingItCloseDetail,
        willRunOut, willRunOutDetail, limitReset, limitResetDetail, notificationsDenied, openSystemSettings

    public var translations: (english: String, russian: String) {
        switch self {
        case .notifyMeWhen: ("Notify Me When", "Уведомлять, когда")
        case .notificationsFooter:
            ("Hidden accounts and hidden limits never notify.", "Скрытые аккаунты и лимиты не присылают уведомлений.")
        case .almostOut: ("Almost out", "Почти исчерпан")
        case .almostOutDetail: ("A limit drops under 10% left", "Остаток лимита опускается ниже 10%")
        case .cuttingItClose: ("Cutting it close", "На грани")
        case .cuttingItCloseDetail:
            ("The pace says a limit will barely last until reset", "По текущему темпу лимита едва хватит до сброса")
        case .willRunOut: ("Will run out", "Закончится раньше сброса")
        case .willRunOutDetail:
            ("The pace says a limit runs out before it resets", "По текущему темпу лимит закончится до сброса")
        case .limitReset: ("Limit reset", "Лимит сброшен")
        case .limitResetDetail: ("A limit that was running low resets", "Сбросился лимит, который был на исходе")
        case .notificationsDenied:
            (
                "Notifications for Headroom are turned off in System Settings.",
                "Уведомления Headroom выключены в Системных настройках."
            )
        case .openSystemSettings: ("Open System Settings", "Открыть Системные настройки")
        }
    }
}

public enum ServiceText: LocalizedText {
    case version, serviceStatus, running, notRunning, connectingToHeadroom, logs, daemonLog, openLog, showInFinder,
        startup, launchAtLogin, launchAtLoginDetail, loginItemNeedsApproval, openLoginItems

    public var translations: (english: String, russian: String) {
        switch self {
        case .version: ("Version {version}", "Версия {version}")
        case .serviceStatus: ("Status", "Состояние")
        case .running: ("Running", "Работает")
        case .notRunning: ("Service not running", "Служба не запущена")
        case .connectingToHeadroom: ("Connecting to Headroom…", "Подключение к Headroom…")
        case .logs: ("Logs", "Журналы")
        case .daemonLog: ("Service log", "Журнал службы")
        case .openLog: ("Open Log", "Открыть журнал")
        case .showInFinder: ("Show in Finder", "Показать в Finder")
        case .startup: ("Startup", "Запуск")
        case .launchAtLogin: ("Launch at login", "Открывать при входе")
        case .launchAtLoginDetail:
            ("Start Headroom when you log in to this Mac", "Запускать Headroom при входе в систему")
        case .loginItemNeedsApproval:
            (
                "Allow Headroom in System Settings → General → Login Items.",
                "Разрешите Headroom в Системных настройках → Основные → Объекты входа."
            )
        case .openLoginItems: ("Open Login Items", "Открыть объекты входа")
        }
    }
}

public enum PluralTemplate: Sendable, CaseIterable {
    case everySeconds, everyMinutes

    func forms(_ language: UILanguage) -> [String] {
        switch (self, language) {
        case (.everySeconds, .en): ["Every {count} second", "Every {count} seconds"]
        case (.everySeconds, .ru): ["Каждую {count} секунду", "Каждые {count} секунды", "Каждые {count} секунд"]
        case (.everyMinutes, .en): ["Every {count} minute", "Every {count} minutes"]
        case (.everyMinutes, .ru): ["Каждую {count} минуту", "Каждые {count} минуты", "Каждые {count} минут"]
        }
    }
}

extension UIStrings {
    public func text(_ key: some LocalizedText) -> String {
        language == .ru ? key.translations.russian : key.translations.english
    }

    public func fill(_ key: some LocalizedText, _ values: [String: String]) -> String {
        Self.substitute(text(key), values)
    }

    public func fill(_ template: PluralTemplate, count: UInt64) -> String {
        let forms = template.forms(language)
        let index = language.pluralIndex(count)
        let pattern = forms.indices.contains(index) ? forms[index] : forms.last ?? ""
        return Self.substitute(pattern, ["count": "\(count)"])
    }

    public func refreshInterval(seconds: Int64) -> String {
        let magnitude = seconds.magnitude
        guard magnitude % 60 == 0 else { return fill(.everySeconds, count: magnitude) }
        let minutes = magnitude / 60
        guard minutes != 1 else { return text(MenuBarText.everyMinute) }
        return fill(.everyMinutes, count: minutes)
    }

    static func substitute(_ pattern: String, _ values: [String: String]) -> String {
        values.reduce(pattern) { text, entry in
            text.replacingOccurrences(of: "{\(entry.key)}", with: entry.value)
        }
    }
}
