public enum PopupExtraText: LocalizedText {
    case other, modelsFolded, clickShowUsed, clickShowLeft, clickShowResetTime, clickShowCountdown
    case refreshProvider, hideFromPopup, alwaysShow, statusPage, openDashboard, usagePage, shareImage, copyText
    case linkTip, notPinned, showLess, moreCount
    case imageCopied, savedToPictures, textCopied, shareFailed
    case updatedAgo, nextUpdateAt, outdatedUpdated, offlineRetrying, liveEvery, liveTip, liveTipInterval
    case shareLimits, shareOf, tagline, settingsVersion

    public var translations: (english: String, russian: String) {
        switch self {
        case .other: ("Other", "Другие")
        case .modelsFolded:
            ("Models after the top 5 are folded into Other.", "Модели после первых пяти собраны в «Другие».")
        case .clickShowUsed: ("Click to show used", "Нажмите, чтобы показать использованное")
        case .clickShowLeft: ("Click to show what's left", "Нажмите, чтобы показать остаток")
        case .clickShowResetTime: ("Click to show the reset time", "Нажмите, чтобы показать время сброса")
        case .clickShowCountdown: ("Click to show the countdown", "Нажмите, чтобы показать обратный отсчёт")
        default: menuTranslations
        }
    }

    private var menuTranslations: (english: String, russian: String) {
        switch self {
        case .refreshProvider: ("Refresh {provider}", "Обновить {provider}")
        case .hideFromPopup: ("Hide from popup", "Скрыть из окна")
        case .alwaysShow: ("Always show", "Всегда показывать")
        case .statusPage: ("Status page", "Страница статуса")
        case .openDashboard: ("Open dashboard", "Открыть панель управления")
        case .usagePage: ("Usage page", "Страница использования")
        case .shareImage: ("Share as image…", "Поделиться картинкой…")
        case .copyText: ("Copy as text", "Копировать текстом")
        case .linkTip: ("{title} · {host}", "{title} · {host}")
        case .notPinned: ("Not pinned", "Не закреплены")
        case .showLess: ("Show less", "Свернуть")
        case .moreCount: ("{count} more", "Ещё {count}")
        default: toastTranslations
        }
    }

    private var toastTranslations: (english: String, russian: String) {
        switch self {
        case .imageCopied: ("Image copied", "Картинка скопирована")
        case .savedToPictures: ("saved to Pictures/Headroom", "сохранена в Изображения/Headroom")
        case .textCopied: ("Copied as text", "Скопировано текстом")
        case .shareFailed: ("Couldn't save the image", "Не удалось сохранить картинку")
        default: footerTranslations
        }
    }

    private var footerTranslations: (english: String, russian: String) {
        switch self {
        case .updatedAgo: ("Updated {ago}", "Обновлено {ago}")
        case .nextUpdateAt: ("Next update at {time}", "Следующее обновление в {time}")
        case .outdatedUpdated: ("Outdated · updated {ago}", "Устарело · обновлено {ago}")
        case .offlineRetrying: ("Offline — retrying in {duration}", "Нет сети — повтор через {duration}")
        case .liveEvery:
            ("Live — {interval} while {providers} is active", "Live — {interval}, пока {providers} активен")
        case .liveTip: liveTipTranslations
        case .liveTipInterval:
            (
                "Back to {interval} after 10 min without activity.", "После 10 мин без активности — снова {interval}."
            )
        default: shareTranslations
        }
    }

    private var liveTipTranslations: (english: String, russian: String) {
        (
            "{providers} is writing local logs, so it is checked {interval}.",
            "{providers} пишет локальные журналы, поэтому проверяется {interval}."
        )
    }

    private var shareTranslations: (english: String, russian: String) {
        switch self {
        case .shareLimits: ("{provider} limits · {date}", "Лимиты {provider} · {date}")
        case .shareOf: ("of {capacity}%", "из {capacity}%")
        case .tagline: ("Know what's left.", "Знай, сколько осталось.")
        case .settingsVersion: ("{settings} · {version}", "{settings} · {version}")
        default: ("", "")
        }
    }
}

public enum PopupPlural: Sendable, CaseIterable {
    case models, projects, otherModels, otherProjects

    func forms(_ language: UILanguage) -> [String] {
        switch (self, language) {
        case (.models, .en): ["{count} model", "{count} models"]
        case (.models, .ru): ["{count} модель", "{count} модели", "{count} моделей"]
        case (.projects, .en): ["{count} project", "{count} projects"]
        case (.projects, .ru): ["{count} проект", "{count} проекта", "{count} проектов"]
        case (.otherModels, .en): ["· {count} model", "· {count} models"]
        case (.otherModels, .ru): ["· {count} модель", "· {count} модели", "· {count} моделей"]
        case (.otherProjects, .en): ["· {count} project", "· {count} projects"]
        case (.otherProjects, .ru): ["· {count} проект", "· {count} проекта", "· {count} проектов"]
        }
    }
}

extension UIStrings {
    public func fill(_ plural: PopupPlural, count: UInt64) -> String {
        let forms = plural.forms(language)
        let index = language.pluralIndex(count)
        let pattern = forms.indices.contains(index) ? forms[index] : forms.last ?? ""
        return Self.substitute(pattern, ["count": "\(count)"])
    }
}
