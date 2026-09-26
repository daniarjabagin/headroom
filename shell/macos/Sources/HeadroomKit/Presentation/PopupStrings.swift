public enum PopupText: Sendable, CaseIterable {
    case today, yesterday, last30Days, noUsageInPeriod
    case spendEstimate, spendUnpricedModels, usageTrend, noUsage, someModelsUnpriced, unpriced, partlyUnpriced
    case outdated, refreshFailed, retry, retrying, signIn, signInAgain, signedOutDetail, signInThenRetry
    case copyCommand, copied, accountChangedDetail
    case noSubscriptionDetail, overPace, limitSoon, runsOutAnyMinute
    case serviceDownTitle, serviceDownDetail, stateUnreadable, tryAgain, noToolsFound, checkAgain, settings

    var english: String {
        switch self {
        case .today: "Today"
        case .yesterday: "Yesterday"
        case .last30Days: "Last 30 Days"
        case .noUsageInPeriod: "No usage in this period"
        case .spendEstimate: "Estimated from local logs and public pricing."
        case .spendUnpricedModels: "Some models have no public price yet."
        case .usageTrend: "Usage Trend"
        case .noUsage: "No usage"
        case .someModelsUnpriced: " · some models unpriced"
        case .unpriced: "unpriced"
        case .partlyUnpriced: "Partly unpriced, cost leaves it out"
        case .outdated: "Outdated"
        case .refreshFailed: "Refresh failed"
        case .retry: "Retry"
        case .retrying: "Retrying…"
        case .signIn: "Sign in"
        case .signInAgain: "Sign in again…"
        case .signedOutDetail:
            "Sign in again through Headroom (Settings → Accounts → Add account), or remove the account there."
        case .signInThenRetry: "Sign in again with the provider's app, then press Retry."
        case .copyCommand: "Copy command"
        case .copied: "Copied"
        case .accountChangedDetail: "Press Retry to switch to it."
        case .noSubscriptionDetail:
            "Limits aren't available for this account. Renew the plan or sign in with another account."
        case .overPace: "Over pace"
        case .limitSoon: "Limit soon"
        case .runsOutAnyMinute: "At this pace: runs out any minute"
        case .serviceDownTitle: "Headroom service isn't running"
        case .serviceDownDetail: "Headroom keeps trying to start it."
        case .stateUnreadable: "Couldn't read Headroom's state"
        case .tryAgain: "Try again"
        case .noToolsFound: "No AI coding tools found."
        case .checkAgain: "Check again"
        case .settings: "Settings"
        }
    }

    var russian: String {
        switch self {
        case .today: "Сегодня"
        case .yesterday: "Вчера"
        case .last30Days: "За 30 дней"
        case .noUsageInPeriod: "За этот период расходов нет"
        case .spendEstimate: "Оценка по локальным журналам и публичным ценам."
        case .spendUnpricedModels: "У некоторых моделей пока нет публичной цены."
        case .usageTrend: "Динамика"
        case .noUsage: "Без использования"
        case .someModelsUnpriced: " · часть моделей без цены"
        case .unpriced: "без цены"
        case .partlyUnpriced: "Частично без цены, в стоимость не входит"
        case .outdated: "Устарело"
        case .refreshFailed: "Не удалось обновить"
        case .retry: "Повторить"
        case .retrying: "Повторяем…"
        case .signIn: "Войти"
        case .signInAgain: "Войти снова…"
        case .signedOutDetail:
            "Войдите снова через Headroom (Настройки → Аккаунты → Добавить аккаунт) или удалите там этот аккаунт."
        case .signInThenRetry: "Войдите снова в приложении провайдера, затем нажмите «Повторить»."
        case .copyCommand: "Скопировать команду"
        case .copied: "Скопировано"
        case .accountChangedDetail: "Нажмите «Повторить», чтобы переключиться на него."
        case .noSubscriptionDetail: "Данные о лимитах недоступны. Продлите подписку или войдите в другой аккаунт."
        case .overPace: "Темп превышен"
        case .limitSoon: "Скоро лимит"
        case .runsOutAnyMinute: "При текущем темпе закончится с минуты на минуту"
        case .serviceDownTitle: "Служба Headroom не запущена"
        case .serviceDownDetail: "Headroom пытается запустить её снова."
        case .stateUnreadable: "Не удалось прочитать состояние Headroom"
        case .tryAgain: "Повторить"
        case .noToolsFound: "ИИ-инструменты для программирования не найдены."
        case .checkAgain: "Проверить снова"
        case .settings: "Настройки"
        }
    }
}

public enum PopupTemplate: Sendable, CaseIterable {
    case spare, limitIn, runsOutIn, paceRunsOut, paceRunsOutResets, paceUsedAtReset, paceLeftAtReset
    case lastUpdated, signedOutOf, couldNotRefresh, otherModels, dayTitle, version
    case accountChanged, runInTerminal

    var english: String {
        switch self {
        case .spare: "~{percent}% spare"
        case .limitIn: "Limit in {duration}"
        case .runsOutIn: "runs out in {duration}"
        case .paceRunsOut: "At this pace: {runsOut}"
        case .paceRunsOutResets: "At this pace: {runsOut} · {resets}"
        case .paceUsedAtReset: "At this pace: ~{percent}% used at reset"
        case .paceLeftAtReset: "At this pace: ~{percent}% left at reset"
        case .lastUpdated: "Last updated {ago}"
        case .signedOutOf: "Signed out of {provider}"
        case .couldNotRefresh: "Couldn't refresh {provider}"
        case .otherModels: "Other ({count})"
        case .dayTitle: "{weekday}, {day}"
        case .version: "Headroom {version}"
        case .accountChanged: "Another account is signed in to {provider}"
        case .runInTerminal: "Run `{command}` in Terminal, then press Retry."
        }
    }

    var russian: String {
        switch self {
        case .spare: "~{percent}% запаса"
        case .limitIn: "Лимит через {duration}"
        case .runsOutIn: "закончится через {duration}"
        case .paceRunsOut: "При текущем темпе: {runsOut}"
        case .paceRunsOutResets: "При текущем темпе: {runsOut} · {resets}"
        case .paceUsedAtReset: "При текущем темпе к сбросу будет использовано ~{percent}%"
        case .paceLeftAtReset: "При текущем темпе к сбросу останется ~{percent}%"
        case .lastUpdated: "Обновлено {ago}"
        case .signedOutOf: "Выполнен выход из {provider}"
        case .couldNotRefresh: "Не удалось обновить {provider}"
        case .otherModels: "Другие ({count})"
        case .dayTitle: "{weekday}, {day}"
        case .version: "Headroom {version}"
        case .accountChanged: "В {provider} выполнен вход в другой аккаунт"
        case .runInTerminal: "Выполните `{command}` в Терминале, затем нажмите «Повторить»."
        }
    }
}

extension UIStrings {
    public func text(_ key: PopupText) -> String {
        language == .ru ? key.russian : key.english
    }

    public func fill(_ template: PopupTemplate, _ values: [String: String]) -> String {
        let pattern = language == .ru ? template.russian : template.english
        return values.reduce(pattern) { text, entry in
            text.replacingOccurrences(of: "{\(entry.key)}", with: entry.value)
        }
    }

    func shortWeekday(_ index: Int) -> String {
        let english = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
        let russian = ["вс", "пн", "вт", "ср", "чт", "пт", "сб"]
        let names = language == .ru ? russian : english
        return names.indices.contains(index) ? names[index] : ""
    }
}
