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

public enum PanelLabel: String, LenientStringEnum {
    case percent, window
    public static let fallback = PanelLabel.percent
}

public struct DisplaySettings: Codable, Sendable, Hashable {
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
    public var combineAccounts: Bool

    enum CodingKeys: String, CodingKey {
        case theme, language, translucent
        case valueMode = "value_mode"
        case resetFormat = "reset_format"
        case panelLabel = "panel_label"
        case showSpend = "show_spend"
        case showAccountSpend = "show_account_spend"
        case showTrend = "show_trend"
        case showForecast = "show_forecast"
        case hiddenWindows = "hidden_windows"
        case combineAccounts = "combine_accounts"
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
        combineAccounts = try container.decodeIfPresent(Bool.self, forKey: .combineAccounts) ?? false
    }
}
