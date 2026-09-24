public enum CombinedText: LocalizedText {
    case combineAccounts, combineAccountsDetail, percentLeftOf, percentUsedOf
    case paceLeftOfAtReset, paceUsedOfAtReset, runsOutBeforeReset

    public var translations: (english: String, russian: String) {
        switch self {
        case .combineAccounts: ("Combine accounts of the same provider", "Объединять аккаунты одного провайдера")
        case .combineAccountsDetail:
            (
                "One card per provider with a meter segment for each account",
                "Одна карточка на провайдера, в шкале по сегменту на каждый аккаунт"
            )
        case .percentLeftOf: ("{percent}% left of {capacity}%", "Осталось {percent}% из {capacity}%")
        case .percentUsedOf: ("{percent}% used of {capacity}%", "Использовано {percent}% из {capacity}%")
        case .paceLeftOfAtReset:
            (
                "At this pace: ~{percent}% of {capacity}% left at reset",
                "При текущем темпе к сбросу останется ~{percent}% из {capacity}%"
            )
        case .paceUsedOfAtReset:
            (
                "At this pace: ~{percent}% of {capacity}% used at reset",
                "При текущем темпе к сбросу будет использовано ~{percent}% из {capacity}%"
            )
        case .runsOutBeforeReset: ("At this pace: runs out before reset", "При текущем темпе закончится до сброса")
        }
    }
}
