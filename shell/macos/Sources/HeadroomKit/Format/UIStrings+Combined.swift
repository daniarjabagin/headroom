public enum CombinedText: LocalizedText {
    case combineAccounts, combineAccountsDetail, percentLeftOf, percentUsedOf

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
        }
    }
}
