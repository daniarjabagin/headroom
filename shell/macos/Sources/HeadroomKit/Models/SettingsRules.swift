public enum SettingsProblem: Error, Sendable, Hashable {
    case refreshInterval(Int64)
    case emptyHeadlineTarget
    case blankHiddenWindow
    case blankStarredAccount
    case blankPanelLimit
    case tooManyPanelLimits(Int)
    case thresholdPercent(Int)
    case blankProviderThreshold
    case providerThreshold(provider: String, value: Int)
    case emptyQuietHours
    case shortcutTooLong
    case invalidShortcut(String)
}

public enum SettingsRules {
    public static let refreshIntervalRange: ClosedRange<Int64> = 60...3600
    public static let thresholdPercentRange: ClosedRange<Int> = 1...50
    public static let providerThresholdRange: ClosedRange<Int> = 0...50
    public static let defaultThresholdPercent = 10
    public static let maxPanelLimits = 3
    public static let maxShortcutCharacters = 64

    public static func problem(in settings: Settings) -> SettingsProblem? {
        if !refreshIntervalRange.contains(settings.refreshIntervalSecs) {
            return .refreshInterval(settings.refreshIntervalSecs)
        }
        if case .pinned(let accountID, let window) = settings.headline, isBlank(accountID) || isBlank(window) {
            return .emptyHeadlineTarget
        }
        return problem(in: settings.display) ?? problem(in: settings.notifications)
            ?? problem(shortcut: settings.shortcuts.open)
    }

    static func problem(in display: DisplaySettings) -> SettingsProblem? {
        let blankWindow = display.hiddenWindows.contains { account, windows in
            isBlank(account) || windows.contains(where: isBlank)
        }
        if blankWindow { return .blankHiddenWindow }
        if display.starredAccounts.contains(where: isBlank) { return .blankStarredAccount }
        if display.panelLimits.contains(where: { isBlank($0.accountID) || isBlank($0.window) }) {
            return .blankPanelLimit
        }
        let distinct = Set(display.panelLimits).count
        return distinct > maxPanelLimits ? .tooManyPanelLimits(distinct) : nil
    }

    static func problem(in notifications: NotificationSettings) -> SettingsProblem? {
        if !thresholdPercentRange.contains(notifications.thresholdPercent) {
            return .thresholdPercent(notifications.thresholdPercent)
        }
        for (provider, value) in notifications.providerThresholds.sorted(by: { $0.key < $1.key }) {
            if isBlank(provider) { return .blankProviderThreshold }
            if !providerThresholdRange.contains(value) { return .providerThreshold(provider: provider, value: value) }
        }
        let quiet = notifications.quietHours
        return quiet.enabled && quiet.from == quiet.to ? .emptyQuietHours : nil
    }

    static func problem(shortcut: String) -> SettingsProblem? {
        if shortcut.count > maxShortcutCharacters { return .shortcutTooLong }
        return shortcut.isEmpty || isAccelerator(shortcut) ? nil : .invalidShortcut(shortcut)
    }

    public static func isAccelerator(_ text: String) -> Bool {
        var rest = Substring(text)
        while rest.first == "<" {
            guard let close = rest.firstIndex(of: ">") else { return false }
            let modifier = rest[rest.index(after: rest.startIndex)..<close]
            guard !modifier.isEmpty, modifier.allSatisfy(isASCIILetter) else { return false }
            rest = rest[rest.index(after: close)...]
        }
        return !rest.isEmpty && rest.allSatisfy { isASCIILetter($0) || ($0.isASCII && $0.isNumber) || $0 == "_" }
    }

    static func uniqueNonBlank(_ items: [String]) -> [String] {
        unique(items.filter { !isBlank($0) })
    }

    static func unique<Item: Hashable>(_ items: [Item]) -> [Item] {
        var seen: Set<Item> = []
        return items.filter { seen.insert($0).inserted }
    }

    static func isBlank(_ text: String) -> Bool {
        text.allSatisfy(\.isWhitespace)
    }

    private static func isASCIILetter(_ character: Character) -> Bool {
        character.isASCII && character.isLetter
    }
}
