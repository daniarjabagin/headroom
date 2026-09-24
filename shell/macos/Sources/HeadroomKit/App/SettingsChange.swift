public enum DisplaySection: String, Sendable, CaseIterable {
    case showSpend = "show_spend"
    case showAccountSpend = "show_account_spend"
    case showTrend = "show_trend"
    case showForecast = "show_forecast"
}

public enum Milestone: String, Sendable, CaseIterable {
    case almostOut = "almost_out"
    case cuttingItClose = "cutting_it_close"
    case willRunOut = "will_run_out"
    case reset
}

public enum SettingsChange: Sendable, Hashable {
    public static let refreshIntervalRange: ClosedRange<Int64> = 60...3600

    case theme(ThemePreference)
    case language(LanguagePreference)
    case valueMode(ValueMode)
    case resetFormat(ResetFormat)
    case panelLabel(PanelLabel)
    case translucent(Bool)
    case combineAccounts(Bool)
    case section(DisplaySection, Bool)
    case reducedMotion(Bool)
    case refreshInterval(Int64)
    case headline(HeadlineSetting)
    case notification(Milestone, Bool)
    case hiddenWindows(accountID: String, windows: [String])

    public var patch: [String: JSONValue] {
        switch self {
        case .theme(let value): Self.display("theme", .string(value.rawValue))
        case .language(let value): Self.display("language", .string(value.rawValue))
        case .valueMode(let value): Self.display("value_mode", .string(value.rawValue))
        case .resetFormat(let value): Self.display("reset_format", .string(value.rawValue))
        case .panelLabel(let value): Self.display("panel_label", .string(value.rawValue))
        case .translucent(let value): Self.display("translucent", .bool(value))
        case .combineAccounts(let value): Self.display("combine_accounts", .bool(value))
        case .section(let section, let value): Self.display(section.rawValue, .bool(value))
        case .reducedMotion(let value): ["reduced_motion": .bool(value)]
        case .refreshInterval(let seconds): ["refresh_interval_secs": .integer(Self.clampedInterval(seconds))]
        case .headline(let headline): ["headline": Self.headlineValue(headline)]
        case .notification(let milestone, let value): ["notifications": .object([milestone.rawValue: .bool(value)])]
        case .hiddenWindows(let accountID, let windows): Self.hiddenWindowsPatch(accountID, Self.unique(windows))
        }
    }

    public func applied(to settings: Settings) -> Settings {
        var result = settings
        switch self {
        case .theme(let value): result.display.theme = value
        case .language(let value): result.display.language = value
        case .valueMode(let value): result.display.valueMode = value
        case .resetFormat(let value): result.display.resetFormat = value
        case .panelLabel(let value): result.display.panelLabel = value
        case .translucent(let value): result.display.translucent = value
        case .combineAccounts(let value): result.display.combineAccounts = value
        case .section(let section, let value): result.display.set(section, value)
        case .reducedMotion(let value): result.reducedMotion = value
        case .refreshInterval(let seconds): result.refreshIntervalSecs = Self.clampedInterval(seconds)
        case .headline(let headline): result.headline = headline
        case .notification(let milestone, let value): result.notifications.set(milestone, value)
        case .hiddenWindows(let accountID, let windows):
            let kept = Self.unique(windows)
            result.display.hiddenWindows[accountID] = kept.isEmpty ? nil : kept
        }
        return result
    }

    static func clampedInterval(_ seconds: Int64) -> Int64 {
        min(max(seconds, refreshIntervalRange.lowerBound), refreshIntervalRange.upperBound)
    }

    private static func display(_ key: String, _ value: JSONValue) -> [String: JSONValue] {
        ["display": .object([key: value])]
    }

    private static func headlineValue(_ headline: HeadlineSetting) -> JSONValue {
        switch headline {
        case .auto: .object(["mode": .string("auto")])
        case .pinned(let accountID, let window):
            .object(["mode": .string("pinned"), "account_id": .string(accountID), "window": .string(window)])
        }
    }

    private static func hiddenWindowsPatch(_ accountID: String, _ windows: [String]) -> [String: JSONValue] {
        let value: JSONValue = windows.isEmpty ? .null : .array(windows.map(JSONValue.string))
        return display("hidden_windows", .object([accountID: value]))
    }

    private static func unique(_ windows: [String]) -> [String] {
        var seen: Set<String> = []
        return windows.filter { !$0.isEmpty && seen.insert($0).inserted }
    }
}

extension DisplaySettings {
    public func isShown(_ section: DisplaySection) -> Bool {
        switch section {
        case .showSpend: showSpend
        case .showAccountSpend: showAccountSpend
        case .showTrend: showTrend
        case .showForecast: showForecast
        }
    }

    public func isHidden(accountID: String, windowID: String) -> Bool {
        hiddenWindows[accountID]?.contains(windowID) ?? false
    }

    public func hiddenWindows(after windowID: String, hidden: Bool, accountID: String) -> [String] {
        let others = (hiddenWindows[accountID] ?? []).filter { $0 != windowID }
        return hidden ? others + [windowID] : others
    }

    mutating func set(_ section: DisplaySection, _ value: Bool) {
        switch section {
        case .showSpend: showSpend = value
        case .showAccountSpend: showAccountSpend = value
        case .showTrend: showTrend = value
        case .showForecast: showForecast = value
        }
    }
}

extension NotificationSettings {
    public func isEnabled(_ milestone: Milestone) -> Bool {
        switch milestone {
        case .almostOut: almostOut
        case .cuttingItClose: cuttingItClose
        case .willRunOut: willRunOut
        case .reset: reset
        }
    }

    mutating func set(_ milestone: Milestone, _ value: Bool) {
        switch milestone {
        case .almostOut: almostOut = value
        case .cuttingItClose: cuttingItClose = value
        case .willRunOut: willRunOut = value
        case .reset: reset = value
        }
    }
}
