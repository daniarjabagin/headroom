public enum LayoutOptionText: LocalizedText {
    case densityNormal, densityCompact, timeAuto, timeTwelveHour, timeTwentyFourHour, panelHeadline, panelSeveral,
        panelIcon, indicatorRing, indicatorBar, indicatorNone, labelNone

    public var translations: (english: String, russian: String) {
        switch self {
        case .densityNormal: ("Default", "Обычная")
        case .densityCompact: ("Compact", "Компактная")
        case .timeAuto: ("Automatic", "Автоматически")
        case .timeTwelveHour: ("12-hour", "12-часовой")
        case .timeTwentyFourHour: ("24-hour", "24-часовой")
        case .panelHeadline: ("One limit", "Один лимит")
        case .panelSeveral: ("Several limits", "Несколько лимитов")
        case .panelIcon: ("Icon only", "Только значок")
        case .indicatorRing: ("Ring", "Кольцо")
        case .indicatorBar: ("Bar", "Полоса")
        case .indicatorNone: ("None", "Без индикатора")
        case .labelNone: ("No label", "Без подписи")
        }
    }
}

public enum SpendOptionText: LocalizedText {
    case today, yesterday, sevenDays, thirtyDays, cost, costDetail, tokens, tokensDetail, costPerMTok,
        costPerMTokDetail, models, projects, noProject, tokensCaption, blendedCaption

    public var translations: (english: String, russian: String) {
        switch self {
        case .today: ("Today", "Сегодня")
        case .yesterday: ("Yesterday", "Вчера")
        case .sevenDays: ("7 Days", "7 дней")
        case .thirtyDays: ("30 Days", "30 дней")
        case .cost: ("Total Spend", "Всего потрачено")
        case .costDetail: ("Dollars, from local logs and public prices", "Доллары по локальным журналам и ценам")
        case .tokens: ("Total Tokens", "Всего токенов")
        case .tokensDetail: ("Input, output and cache tokens", "Входные, выходные и кэш-токены")
        case .costPerMTok: ("Cost per MTok", "Цена за 1 млн токенов")
        case .costPerMTokDetail: ("Spend divided by million tokens", "Расходы на миллион токенов")
        case .models: ("Models", "Модели")
        case .projects: ("Projects", "Проекты")
        case .noProject: ("No project", "Без проекта")
        case .tokensCaption: ("tokens", "токенов")
        case .blendedCaption: ("blended", "в среднем")
        }
    }
}

public enum AdvancedOptionText: LocalizedText {
    case logError, logWarn, logInfo, logDebug, thresholdOff, thresholdDefault, thresholdPercent, logFile

    public var translations: (english: String, russian: String) {
        switch self {
        case .logError: ("Errors", "Ошибки")
        case .logWarn: ("Warnings", "Предупреждения")
        case .logInfo: ("Info", "Сведения")
        case .logDebug: ("Debug", "Отладка")
        case .thresholdOff: ("Off", "Выкл.")
        case .thresholdDefault: ("Default ({percent}%)", "По умолчанию ({percent}%)")
        case .thresholdPercent: ("Under {percent}% left", "Осталось меньше {percent}%")
        case .logFile: ("Log file", "Файл журнала")
        }
    }
}
