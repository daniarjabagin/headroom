public enum ThemePreference: String, LenientStringEnum {
    case system, light, dark
    public static let fallback = ThemePreference.system
}

public enum LanguagePreference: String, LenientStringEnum {
    case system, en, ru
    public static let fallback = LanguagePreference.system
}

public enum ValueMode: String, LenientStringEnum {
    case left, used
    public static let fallback = ValueMode.left
}

public enum ResetFormat: String, LenientStringEnum {
    case countdown, exact
    public static let fallback = ResetFormat.countdown
}

public enum PanelLabel: String, LenientStringEnum, CaseIterable {
    case percent, window, none
    public static let fallback = PanelLabel.percent
}

public struct DisplaySettings: Decodable, Sendable, Hashable {
    public var theme: ThemePreference
    public var language: LanguagePreference
    public var valueMode: ValueMode
    public var resetFormat: ResetFormat
    public var panelLabel: PanelLabel
    public var showSpend: Bool
    public var showAccountSpend: Bool
    public var showTrend: Bool
    public var showForecast: Bool
    public var translucent: Bool
    public var hiddenWindows: [String: [String]]
    public var combineAccounts = false
    public var density = Density.normal
    public var timeFormat = TimeFormat.auto
    public var panelMode = PanelMode.headline
    public var panelIndicator = PanelIndicator.ring
    public var panelLimits: [PanelLimit] = []
    public var panelPosition = PanelPosition.standard
    public var spendPeriod = SpendPeriodPreference.last30Days
    public var spendUnit = SpendUnit.cost
    public var spendBreakdown = SpendBreakdown.models
    public var starredAccounts: [String] = []
    public var collapseUnstarred = false
    public var hideOnScreenShare = true

    enum CodingKeys: String, CodingKey {
        case theme, language, translucent, density
        case valueMode = "value_mode"
        case resetFormat = "reset_format"
        case panelLabel = "panel_label"
        case showSpend = "show_spend"
        case showAccountSpend = "show_account_spend"
        case showTrend = "show_trend"
        case showForecast = "show_forecast"
        case hiddenWindows = "hidden_windows"
        case combineAccounts = "combine_accounts"
        case timeFormat = "time_format"
        case panelMode = "panel_mode"
        case panelIndicator = "panel_indicator"
        case panelLimits = "panel_limits"
        case panelPosition = "panel_position"
        case spendPeriod = "spend_period"
        case spendUnit = "spend_unit"
        case spendBreakdown = "spend_breakdown"
        case starredAccounts = "starred_accounts"
        case collapseUnstarred = "collapse_unstarred"
        case hideOnScreenShare = "hide_on_screen_share"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        theme = try container.decode(ThemePreference.self, forKey: .theme)
        language = try container.decode(LanguagePreference.self, forKey: .language)
        valueMode = try container.decode(ValueMode.self, forKey: .valueMode)
        resetFormat = try container.decode(ResetFormat.self, forKey: .resetFormat)
        panelLabel = try container.decode(PanelLabel.self, forKey: .panelLabel)
        showSpend = try container.decode(Bool.self, forKey: .showSpend)
        showAccountSpend = try container.decode(Bool.self, forKey: .showAccountSpend)
        showTrend = try container.decode(Bool.self, forKey: .showTrend)
        showForecast = try container.decode(Bool.self, forKey: .showForecast)
        translucent = try container.decode(Bool.self, forKey: .translucent)
        hiddenWindows = try container.decode([String: [String]].self, forKey: .hiddenWindows)
        try decodeLaterKeys(container)
    }

    private mutating func decodeLaterKeys(_ container: KeyedDecodingContainer<CodingKeys>) throws {
        combineAccounts = try container.value(.combineAccounts, default: combineAccounts)
        density = try container.value(.density, default: density)
        timeFormat = try container.value(.timeFormat, default: timeFormat)
        panelMode = try container.value(.panelMode, default: panelMode)
        panelIndicator = try container.value(.panelIndicator, default: panelIndicator)
        panelLimits = try container.value(.panelLimits, default: panelLimits)
        panelPosition = try container.value(.panelPosition, default: panelPosition)
        spendPeriod = try container.value(.spendPeriod, default: spendPeriod)
        spendUnit = try container.value(.spendUnit, default: spendUnit)
        spendBreakdown = try container.value(.spendBreakdown, default: spendBreakdown)
        starredAccounts = try container.value(.starredAccounts, default: starredAccounts)
        collapseUnstarred = try container.value(.collapseUnstarred, default: collapseUnstarred)
        hideOnScreenShare = try container.value(.hideOnScreenShare, default: hideOnScreenShare)
    }
}
