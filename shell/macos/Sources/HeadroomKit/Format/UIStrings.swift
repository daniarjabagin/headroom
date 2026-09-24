public enum UILanguage: String, Sendable, Hashable {
    case en, ru

    public static func resolve(_ preference: LanguagePreference, preferredLanguages: [String]) -> UILanguage {
        switch preference {
        case .en: .en
        case .ru: .ru
        case .system: preferredLanguages.first.map(fromTag) ?? .en
        }
    }

    static func fromTag(_ tag: String) -> UILanguage {
        tag.lowercased().hasPrefix("ru") ? .ru : .en
    }

    func pluralIndex(_ count: UInt64) -> Int {
        guard self == .ru else { return count == 1 ? 0 : 1 }
        let lastDigit = count % 10
        let lastTwo = count % 100
        if lastDigit == 1 && lastTwo != 11 { return 0 }
        if (2...4).contains(lastDigit) && !(12...14).contains(lastTwo) { return 1 }
        return 2
    }
}

public enum UIText: Sendable, CaseIterable {
    case session, weekly, notStarted, resetPending, resetsSoon, noData, limitReached
    case refresh, quit, serviceNotRunning, connecting, updating, offline, noAccounts
    case justNow, nextUpdateSoon, signedOut, noSubscription, schemaMismatch, helperMismatch, differentService

    var english: String {
        switch self {
        case .session: "Session"
        case .weekly: "Weekly"
        case .notStarted: "Not started"
        case .resetPending: "reset pending"
        case .resetsSoon: "resets soon"
        case .noData: "No data"
        case .limitReached: "Limit reached"
        case .refresh: "Refresh"
        case .quit: "Quit Headroom"
        case .serviceNotRunning: "Service not running"
        case .connecting: "Connecting…"
        case .updating: "Updating…"
        case .offline: "Offline"
        case .noAccounts: "No accounts yet"
        case .justNow: "just now"
        case .nextUpdateSoon: "Next update in <1m"
        case .signedOut: "Signed out"
        case .noSubscription: "No active subscription"
        case .schemaMismatch: "The Headroom service is a different version. Restart Headroom."
        case .helperMismatch: "The bundled Headroom service does not match this app. Reinstall Headroom."
        case .differentService: "A different Headroom service is running."
        }
    }

    var russian: String {
        switch self {
        case .session: "Сессия"
        case .weekly: "Неделя"
        case .notStarted: "Не начато"
        case .resetPending: "ожидается сброс"
        case .resetsSoon: "скоро сброс"
        case .noData: "Нет данных"
        case .limitReached: "Лимит исчерпан"
        case .refresh: "Обновить"
        case .quit: "Выйти из Headroom"
        case .serviceNotRunning: "Служба не запущена"
        case .connecting: "Подключение…"
        case .updating: "Обновление…"
        case .offline: "Нет сети"
        case .noAccounts: "Аккаунтов пока нет"
        case .justNow: "только что"
        case .nextUpdateSoon: "Обновление через <1 мин"
        case .signedOut: "Выполнен выход"
        case .noSubscription: "Нет активной подписки"
        case .schemaMismatch: "Служба Headroom другой версии. Перезапустите Headroom."
        case .helperMismatch: "Встроенная служба Headroom не совпадает с приложением. Переустановите Headroom."
        case .differentService: "Запущена другая служба Headroom."
        }
    }
}

public enum UITemplate: Sendable {
    case percentLeft, percentUsed, seconds, minutesSeconds, daysHours, hoursMinutes, minutes
    case resetsMoment, resetsIn, todayAt, tomorrowAt, weekdayAt, dayAt, monthDay
    case nextUpdateIn, ago, updatedAt

    var english: String {
        switch self {
        case .percentLeft: "{percent}% left"
        case .percentUsed: "{percent}% used"
        case .seconds: "{seconds}s"
        case .minutesSeconds: "{minutes}m {seconds}s"
        case .daysHours: "{days}d {hours}h"
        case .hoursMinutes: "{hours}h {minutes}m"
        case .minutes: "{minutes}m"
        case .resetsMoment: "resets {moment}"
        case .resetsIn: "resets in {duration}"
        case .todayAt: "today at {time}"
        case .tomorrowAt: "tomorrow at {time}"
        case .weekdayAt: "{weekday} at {time}"
        case .dayAt: "{day} at {time}"
        case .monthDay: "{month} {date}"
        case .nextUpdateIn: "Next update in {duration}"
        case .ago: "{duration} ago"
        case .updatedAt: "Updated {time}"
        }
    }

    var russian: String {
        switch self {
        case .percentLeft: "Осталось {percent}%"
        case .percentUsed: "Использовано {percent}%"
        case .seconds: "{seconds} с"
        case .minutesSeconds: "{minutes} мин {seconds} с"
        case .daysHours: "{days} д {hours} ч"
        case .hoursMinutes: "{hours} ч {minutes} мин"
        case .minutes: "{minutes} мин"
        case .resetsMoment: "сброс {moment}"
        case .resetsIn: "сброс через {duration}"
        case .todayAt: "сегодня в {time}"
        case .tomorrowAt: "завтра в {time}"
        case .weekdayAt: "{weekday} в {time}"
        case .dayAt: "{day} в {time}"
        case .monthDay: "{date} {month}"
        case .nextUpdateIn: "Обновление через {duration}"
        case .ago: "{duration} назад"
        case .updatedAt: "Обновлено в {time}"
        }
    }
}

public struct UIStrings: Sendable, Hashable {
    public let language: UILanguage

    public init(language: UILanguage) {
        self.language = language
    }

    public func text(_ key: UIText) -> String {
        language == .ru ? key.russian : key.english
    }

    public func fill(_ template: UITemplate, _ values: [String: String]) -> String {
        let pattern = language == .ru ? template.russian : template.english
        return values.reduce(pattern) { text, entry in
            text.replacingOccurrences(of: "{\(entry.key)}", with: entry.value)
        }
    }

    func tokenWord(_ count: UInt64) -> String {
        switch language {
        case .en: ["token", "tokens"][language.pluralIndex(count)]
        case .ru: ["токен", "токена", "токенов"][language.pluralIndex(count)]
        }
    }

    func month(_ index: Int) -> String {
        let english = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        let russian = [
            "янв.", "февр.", "мар.", "апр.", "мая", "июн.", "июл.", "авг.", "сент.", "окт.", "нояб.", "дек.",
        ]
        return Self.pick(language == .ru ? russian : english, index)
    }

    func weekdayOn(_ index: Int) -> String {
        let english = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
        let russian = [
            "в воскресенье", "в понедельник", "во вторник", "в среду", "в четверг", "в пятницу", "в субботу",
        ]
        return Self.pick(language == .ru ? russian : english, index)
    }

    private static func pick(_ names: [String], _ index: Int) -> String {
        names.indices.contains(index) ? names[index] : ""
    }
}
