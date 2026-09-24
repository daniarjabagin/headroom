import Foundation

public enum UpdateText: LocalizedText {
    case updates, checkForUpdatesMenu, automaticChecks, automaticChecksDetail, checkNow, lastChecked, neverChecked

    public var translations: (english: String, russian: String) {
        switch self {
        case .updates: ("App updates", "Обновления приложения")
        case .checkForUpdatesMenu: ("Check for Updates…", "Проверить обновления…")
        case .automaticChecks: ("Automatically check for updates", "Проверять обновления автоматически")
        case .automaticChecksDetail:
            (
                "Once a day. Headroom asks before installing a new version.",
                "Раз в день. Перед установкой новой версии Headroom спросит."
            )
        case .checkNow: ("Check Now", "Проверить сейчас")
        case .lastChecked: ("Last checked {when}", "Последняя проверка: {when}")
        case .neverChecked: ("Not checked yet", "Ещё не проверялось")
        }
    }
}

extension DisplayFormatter {
    public func lastUpdateCheckText(_ lastCheck: Date?, now: Date) -> String {
        guard let lastCheck else { return strings.text(UpdateText.neverChecked) }
        let when = agoText(Timestamp(date: lastCheck), now: Timestamp(date: now))
        return strings.fill(UpdateText.lastChecked, ["when": when])
    }
}
