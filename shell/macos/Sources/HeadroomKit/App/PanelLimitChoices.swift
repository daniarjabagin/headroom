public struct PanelLimitOption: Sendable, Hashable, Identifiable {
    public let limit: PanelLimit
    public let label: String
    public let available: Bool

    public var id: PanelLimit { limit }
}

public enum PanelLimitChoices {
    public static let maximum = 3

    public static func options(
        accounts: [Account], current: [PanelLimit], formatter: DisplayFormatter
    ) -> [PanelLimitOption] {
        let visible = accounts.filter { !$0.hidden }
        var options: [PanelLimitOption] = []
        for account in visible {
            let title = SettingsOptions.accountTitle(account, among: visible)
            for window in account.windows {
                let label = "\(title) \(formatter.windowLabel(id: window.id, label: window.label))"
                options.append(
                    PanelLimitOption(
                        limit: PanelLimit(accountID: account.id, window: window.id), label: label, available: true))
            }
        }
        let unavailable = current.filter { limit in !options.contains { $0.limit == limit } }
        let pinned = formatter.strings.text(MenuBarText.pinnedUnavailable)
        return options + unavailable.map { PanelLimitOption(limit: $0, label: pinned, available: false) }
    }

    public static func toggled(_ limit: PanelLimit, in current: [PanelLimit]) -> [PanelLimit] {
        if current.contains(limit) { return current.filter { $0 != limit } }
        guard current.count < maximum else { return current }
        return current + [limit]
    }

    public static func canAdd(_ limit: PanelLimit, to current: [PanelLimit]) -> Bool {
        current.contains(limit) || current.count < maximum
    }

    public static func summary(_ current: [PanelLimit], options: [PanelLimitOption], strings: UIStrings) -> String {
        guard !current.isEmpty else { return strings.text(MenuBarSettingsText.mostCritical) }
        let labels = Dictionary(options.map { ($0.limit, $0.label) }, uniquingKeysWith: { first, _ in first })
        return current.map { labels[$0] ?? strings.text(MenuBarText.pinnedUnavailable) }.joined(separator: ", ")
    }
}
