public enum AlertSettingsText: LocalizedText {
    case almostOutDetail, alertThreshold, alertWhenLessThan, alertWhenLessThanDetail, perProvider, quietHours,
        quietHoursDetail, from, to, allowCritical, allowCriticalDetail, quietHoursFooter

    public var translations: (english: String, russian: String) {
        switch self {
        case .almostOutDetail: ("A limit drops under {percent}% left", "Остаток лимита опускается ниже {percent}%")
        case .alertThreshold: ("Alert Threshold", "Порог уведомлений")
        case .alertWhenLessThan: ("Alert when less than", "Уведомлять, когда осталось меньше")
        case .alertWhenLessThanDetail: ("Used by Almost out", "Для уведомления «Почти исчерпан»")
        case .perProvider: ("Per provider", "По провайдерам")
        case .quietHours: ("Quiet Hours", "Тихие часы")
        case .quietHoursDetail: ("Hold notifications while you are away", "Придерживать уведомления, пока вас нет")
        case .from: ("From", "С")
        case .to: ("To", "До")
        case .allowCritical: ("Still show critical alerts", "Всё равно показывать критичные")
        case .allowCriticalDetail:
            ("Will run out and Almost out come through", "«Закончится раньше сброса» и «Почти исчерпан» приходят сразу")
        case .quietHoursFooter:
            (
                "Held notifications arrive together when quiet hours end.",
                "Отложенные уведомления придут вместе, когда тихие часы закончатся."
            )
        }
    }
}

public enum AdvancedText: LocalizedText {
    case advanced, service, headroomService, logging, logLevel, logLevelDetail, copyPath, copied, troubleshooting,
        troubleshootingDetail, copyDiagnostics, copyDiagnosticsDetail, copy, diagnosticsCopied, resetAll,
        resetTitle, resetBody, reset, cancel, logLevelFromEnvironment

    public var translations: (english: String, russian: String) {
        switch self {
        case .advanced: ("Advanced", "Дополнительно")
        case .service: ("Service", "Служба")
        case .headroomService: ("Headroom service", "Служба Headroom")
        case .logging: ("Logging", "Журнал")
        case .logLevel: ("Log level", "Уровень журнала")
        case .logLevelDetail:
            ("Debug adds provider responses without tokens", "«Отладка» добавляет ответы провайдеров без токенов")
        case .copyPath: ("Copy Path", "Скопировать путь")
        case .copied: ("Copied", "Скопировано")
        case .troubleshooting: ("Troubleshooting", "Диагностика")
        case .troubleshootingDetail: ("Paste the diagnostics into a bug report.", "Вставьте отчёт в описание ошибки.")
        case .copyDiagnostics: ("Copy diagnostics", "Скопировать отчёт")
        case .copyDiagnosticsDetail:
            (
                "Versions, system and account states. No tokens or emails.",
                "Версии, система и состояние аккаунтов. Без токенов и почты."
            )
        case .copy: ("Copy", "Скопировать")
        case .diagnosticsCopied: ("Diagnostics copied", "Отчёт скопирован")
        case .resetAll: ("Reset all settings…", "Сбросить все настройки…")
        case .resetTitle: ("Reset all settings?", "Сбросить все настройки?")
        case .resetBody:
            (
                "Accounts stay signed in; appearance, notifications and hidden limits return to defaults.",
                "Аккаунты останутся подключены; внешний вид, уведомления и скрытые лимиты вернутся к исходным."
            )
        case .reset: ("Reset", "Сбросить")
        case .cancel: ("Cancel", "Отмена")
        case .logLevelFromEnvironment:
            ("RUST_LOG is set and overrides this level.", "Задана переменная RUST_LOG, она важнее этого уровня.")
        }
    }
}

public enum SupportText: LocalizedText {
    case supportHeadroom, starOnGitHub, starOnGitHubDetail, openGitHub

    public var translations: (english: String, russian: String) {
        switch self {
        case .supportHeadroom: ("Support Headroom", "Поддержать Headroom")
        case .starOnGitHub: ("Star Headroom on GitHub", "Поставьте звезду на GitHub")
        case .starOnGitHubDetail:
            (
                "Stars help other people find it. It's free and takes a second.",
                "Звёзды помогают другим найти Headroom. Это бесплатно и занимает секунду."
            )
        case .openGitHub: ("Open GitHub", "Открыть GitHub")
        }
    }
}
