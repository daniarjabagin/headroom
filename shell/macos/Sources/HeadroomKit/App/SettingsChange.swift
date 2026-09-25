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
    case density(Density)
    case timeFormat(TimeFormat)
    case panel(PanelChange)
    case spend(SpendChange)
    case starredAccounts([String])
    case collapseUnstarred(Bool)
    case hideOnScreenShare(Bool)
    case adaptiveRefresh(Bool)
    case alerts(AlertChange)
    case statusPages(Bool)
    case shortcut(String)
    case logLevel(LogLevel)
    case onboardingCompleted(Bool)

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
        case .density(let value): Self.display("density", .string(value.rawValue))
        case .timeFormat(let value): Self.display("time_format", .string(value.rawValue))
        case .panel(let change): Self.display(change.key, change.value)
        case .spend(let change): Self.display(change.key, change.value)
        case .starredAccounts(let ids):
            Self.display("starred_accounts", Self.strings(SettingsRules.uniqueNonBlank(ids)))
        case .collapseUnstarred(let value): Self.display("collapse_unstarred", .bool(value))
        case .hideOnScreenShare(let value): Self.display("hide_on_screen_share", .bool(value))
        case .adaptiveRefresh(let value): ["adaptive_refresh": .bool(value)]
        case .alerts(let change): ["notifications": .object(change.patch)]
        case .statusPages(let value): ["status_pages": .object(["enabled": .bool(value)])]
        case .shortcut(let value): ["shortcuts": .object(["open": .string(value)])]
        case .logLevel(let value): ["logging": .object(["level": .string(value.rawValue)])]
        case .onboardingCompleted(let value): ["onboarding": .object(["completed": .bool(value)])]
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
        case .density(let value): result.display.density = value
        case .timeFormat(let value): result.display.timeFormat = value
        case .panel(let change): change.apply(to: &result.display)
        case .spend(let change): change.apply(to: &result.display)
        case .starredAccounts(let ids): result.display.starredAccounts = SettingsRules.uniqueNonBlank(ids)
        case .collapseUnstarred(let value): result.display.collapseUnstarred = value
        case .hideOnScreenShare(let value): result.display.hideOnScreenShare = value
        case .adaptiveRefresh(let value): result.adaptiveRefresh = value
        case .alerts(let change): change.apply(to: &result.notifications)
        case .statusPages(let value): result.statusPages.enabled = value
        case .shortcut(let value): result.shortcuts.open = value
        case .logLevel(let value): result.logging.level = value
        case .onboardingCompleted(let value): result.onboarding.completed = value
        }
        return result
    }

    static func clampedInterval(_ seconds: Int64) -> Int64 {
        clamped(seconds, to: SettingsRules.refreshIntervalRange)
    }

    static func clamped<Value: Comparable>(_ value: Value, to range: ClosedRange<Value>) -> Value {
        min(max(value, range.lowerBound), range.upperBound)
    }

    static func strings(_ values: [String]) -> JSONValue {
        .array(values.map(JSONValue.string))
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
        let value: JSONValue = windows.isEmpty ? .null : strings(windows)
        return display("hidden_windows", .object([accountID: value]))
    }

    private static func unique(_ windows: [String]) -> [String] {
        SettingsRules.unique(windows.filter { !$0.isEmpty })
    }
}
