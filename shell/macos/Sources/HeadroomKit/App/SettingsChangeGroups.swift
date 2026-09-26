public enum PanelChange: Sendable, Hashable {
    case mode(PanelMode)
    case indicator(PanelIndicator)
    case limits([PanelLimit])
    case position(PanelPosition)

    var key: String {
        switch self {
        case .mode: "panel_mode"
        case .indicator: "panel_indicator"
        case .limits: "panel_limits"
        case .position: "panel_position"
        }
    }

    var value: JSONValue {
        switch self {
        case .mode(let mode): .string(mode.rawValue)
        case .indicator(let indicator): .string(indicator.rawValue)
        case .limits(let limits): .array(Self.kept(limits).map(Self.limitValue))
        case .position(let position):
            .object(["box": .string(position.box.rawValue), "index": .integer(Int64(position.index))])
        }
    }

    func apply(to display: inout DisplaySettings) {
        switch self {
        case .mode(let mode): display.panelMode = mode
        case .indicator(let indicator): display.panelIndicator = indicator
        case .limits(let limits): display.panelLimits = Self.kept(limits)
        case .position(let position): display.panelPosition = position
        }
    }

    static func kept(_ limits: [PanelLimit]) -> [PanelLimit] {
        SettingsRules.unique(
            limits.filter { !SettingsRules.isBlank($0.accountID) && !SettingsRules.isBlank($0.window) })
    }

    private static func limitValue(_ limit: PanelLimit) -> JSONValue {
        .object(["account_id": .string(limit.accountID), "window": .string(limit.window)])
    }
}

public enum SpendChange: Sendable, Hashable {
    case period(SpendPeriodPreference)
    case unit(SpendUnit)
    case breakdown(SpendBreakdown)
    case showBreakdown(Bool)

    var key: String {
        switch self {
        case .period: "spend_period"
        case .unit: "spend_unit"
        case .breakdown: "spend_breakdown"
        case .showBreakdown: "show_breakdown"
        }
    }

    var value: JSONValue {
        switch self {
        case .period(let period): .string(period.rawValue)
        case .unit(let unit): .string(unit.rawValue)
        case .breakdown(let breakdown): .string(breakdown.rawValue)
        case .showBreakdown(let shown): .bool(shown)
        }
    }

    func apply(to display: inout DisplaySettings) {
        switch self {
        case .period(let period): display.spendPeriod = period
        case .unit(let unit): display.spendUnit = unit
        case .breakdown(let breakdown): display.spendBreakdown = breakdown
        case .showBreakdown(let shown): display.showBreakdown = shown
        }
    }
}

public enum QuietHoursField: Sendable, Hashable {
    case enabled(Bool)
    case from(TimeOfDay)
    case to(TimeOfDay)
    case allowCritical(Bool)
}

public enum AlertChange: Sendable, Hashable {
    case threshold(Int)
    case providerThreshold(provider: String, percent: Int?)
    case quietHours(QuietHoursField)

    var patch: [String: JSONValue] {
        switch self {
        case .threshold(let percent):
            ["threshold_percent": .integer(Int64(Self.general(percent)))]
        case .providerThreshold(let provider, let percent):
            ["provider_thresholds": .object([provider: Self.providerValue(percent)])]
        case .quietHours(let field):
            ["quiet_hours": .object(Self.entry(field))]
        }
    }

    func apply(to notifications: inout NotificationSettings) {
        switch self {
        case .threshold(let percent): notifications.thresholdPercent = Self.general(percent)
        case .providerThreshold(let provider, let percent):
            notifications.providerThresholds[provider] = percent.map(Self.forProvider)
        case .quietHours(.enabled(let value)): notifications.quietHours.enabled = value
        case .quietHours(.from(let time)): notifications.quietHours.from = time
        case .quietHours(.to(let time)): notifications.quietHours.to = time
        case .quietHours(.allowCritical(let value)): notifications.quietHours.allowCritical = value
        }
    }

    private static func general(_ percent: Int) -> Int {
        SettingsChange.clamped(percent, to: SettingsRules.thresholdPercentRange)
    }

    private static func forProvider(_ percent: Int) -> Int {
        SettingsChange.clamped(percent, to: SettingsRules.providerThresholdRange)
    }

    private static func providerValue(_ percent: Int?) -> JSONValue {
        guard let percent else { return .null }
        return .integer(Int64(forProvider(percent)))
    }

    private static func entry(_ field: QuietHoursField) -> [String: JSONValue] {
        switch field {
        case .enabled(let value): ["enabled": .bool(value)]
        case .from(let time): ["from": .string(time.text)]
        case .to(let time): ["to": .string(time.text)]
        case .allowCritical(let value): ["allow_critical": .bool(value)]
        }
    }
}
