public struct HeadlineOption: Sendable, Hashable, Identifiable {
    public let setting: HeadlineSetting
    public let label: String

    public var id: HeadlineSetting { setting }
}

public enum SettingsOptions {
    public static let refreshPresets: [Int64] = [60, 120, 300, 600, 900, 1800, 3600]

    public static func refreshIntervals(current: Int64) -> [Int64] {
        refreshPresets.contains(current) ? refreshPresets : (refreshPresets + [current]).sorted()
    }

    public static func headlines(
        accounts: [Account], current: HeadlineSetting, formatter: DisplayFormatter
    ) -> [HeadlineOption] {
        let strings = formatter.strings
        let visible = accounts.filter { !$0.hidden }
        var options = [HeadlineOption(setting: .auto, label: strings.text(MenuBarText.autoMostCritical))]
        for account in visible {
            let title = accountTitle(account, among: visible)
            for window in account.windows {
                let label = "\(title) — \(formatter.windowLabel(id: window.id, label: window.label))"
                options.append(HeadlineOption(setting: .pinned(accountID: account.id, window: window.id), label: label))
            }
        }
        if case .pinned = current, !options.contains(where: { $0.setting == current }) {
            options.append(HeadlineOption(setting: current, label: strings.text(MenuBarText.pinnedUnavailable)))
        }
        return options
    }

    public static func accountTitle(_ account: Account, among accounts: [Account]) -> String {
        let sameProvider = accounts.filter { $0.provider == account.provider }.count
        guard sameProvider > 1 || account.label != nil else { return account.providerName }
        return "\(account.providerName) · \(account.displayName)"
    }

    public static func accountSubtitle(_ account: Account, strings: UIStrings) -> String {
        var parts = [account.providerName]
        if let plan = account.plan { parts.append(plan) }
        if account.label != nil, let email = account.email { parts.append(email) }
        if account.owner == .headroom { parts.append(strings.text(AccountsText.addedInHeadroom)) }
        return parts.joined(separator: " · ")
    }

    public static func removalBody(_ account: Account, strings: UIStrings) -> String {
        guard account.owner != .headroom else { return strings.text(RemovalText.removeHeadroomBody) }
        return strings.fill(RemovalText.removeCLIBody, ["provider": account.providerName])
    }

    public static func removalDetail(_ account: Account, strings: UIStrings) -> String {
        strings.text(account.owner == .headroom ? RemovalText.removeHeadroomDetail : RemovalText.removeCLIDetail)
    }

    public static func reordered(_ ids: [String], moving offsets: [Int], to destination: Int) -> [String] {
        let moving = offsets.filter(ids.indices.contains).sorted()
        guard !moving.isEmpty, (0...ids.count).contains(destination) else { return ids }
        let moved = moving.map { ids[$0] }
        let before = moving.filter { $0 < destination }.count
        var remaining = ids
        for offset in moving.reversed() { remaining.remove(at: offset) }
        remaining.insert(contentsOf: moved, at: destination - before)
        return remaining
    }
}

extension ProviderInfo {
    public var supportedMethods: [AddAccountMethod] {
        addAccount.filter { method in
            if case .unsupported = method { return false }
            return true
        }
    }
}

extension AddAccountMethod {
    public func summary(_ strings: UIStrings) -> String {
        switch self {
        case .cliLogin(let program):
            program.isEmpty
                ? strings.text(AddAccountText.signIn) : strings.fill(AddAccountText.signInWith, ["program": program])
        case .apiKey: strings.text(AddAccountText.apiKey)
        case .autoDetect, .unsupported: strings.text(AddAccountText.detectedAutomatically)
        }
    }
}
