extension Density {
    public func title(_ strings: UIStrings) -> String {
        strings.text(self == .compact ? LayoutOptionText.densityCompact : .densityNormal)
    }
}

extension TimeFormat {
    public func title(_ strings: UIStrings) -> String {
        switch self {
        case .auto: strings.text(LayoutOptionText.timeAuto)
        case .twelveHour: strings.text(LayoutOptionText.timeTwelveHour)
        case .twentyFourHour: strings.text(LayoutOptionText.timeTwentyFourHour)
        }
    }
}

extension PanelMode {
    public func title(_ strings: UIStrings) -> String {
        switch self {
        case .headline: strings.text(LayoutOptionText.panelHeadline)
        case .several: strings.text(LayoutOptionText.panelSeveral)
        case .icon: strings.text(LayoutOptionText.panelIcon)
        }
    }
}

extension PanelIndicator {
    public func title(_ strings: UIStrings) -> String {
        switch self {
        case .ring: strings.text(LayoutOptionText.indicatorRing)
        case .bar: strings.text(LayoutOptionText.indicatorBar)
        case .none: strings.text(LayoutOptionText.indicatorNone)
        }
    }
}

extension PanelLabel {
    public func title(_ strings: UIStrings) -> String {
        switch self {
        case .percent: strings.text(MenuBarText.percent)
        case .window: strings.text(MenuBarText.providerAndLimit)
        case .none: strings.text(LayoutOptionText.labelNone)
        }
    }
}

extension SpendPeriodPreference {
    public static func available(in spend: Spend) -> [SpendPeriodPreference] {
        allCases.filter { spend.period($0) != nil }
    }

    public func title(_ strings: UIStrings) -> String {
        switch self {
        case .today: strings.text(SpendOptionText.today)
        case .yesterday: strings.text(SpendOptionText.yesterday)
        case .last7Days: strings.text(SpendOptionText.sevenDays)
        case .last30Days: strings.text(SpendOptionText.thirtyDays)
        }
    }
}

extension SpendUnit {
    public func title(_ strings: UIStrings) -> String {
        switch self {
        case .cost: strings.text(SpendOptionText.cost)
        case .tokens: strings.text(SpendOptionText.tokens)
        case .costPerMTok: strings.text(SpendOptionText.costPerMTok)
        }
    }

    public func detail(_ strings: UIStrings) -> String {
        switch self {
        case .cost: strings.text(SpendOptionText.costDetail)
        case .tokens: strings.text(SpendOptionText.tokensDetail)
        case .costPerMTok: strings.text(SpendOptionText.costPerMTokDetail)
        }
    }
}

extension SpendBreakdown {
    public func title(_ strings: UIStrings) -> String {
        strings.text(self == .projects ? SpendOptionText.projects : .models)
    }
}

extension LogLevel {
    public func title(_ strings: UIStrings) -> String {
        switch self {
        case .error: strings.text(AdvancedOptionText.logError)
        case .warn: strings.text(AdvancedOptionText.logWarn)
        case .info: strings.text(AdvancedOptionText.logInfo)
        case .debug: strings.text(AdvancedOptionText.logDebug)
        }
    }
}

extension SettingsOptions {
    public static let thresholdPresets = [5, 10, 20, 30]

    public static func thresholds(current: Int) -> [Int] {
        thresholdPresets.contains(current) ? thresholdPresets : (thresholdPresets + [current]).sorted()
    }

    public static func providerThresholds(current: Int?) -> [Int?] {
        let percents = current.map { $0 == 0 ? thresholdPresets : thresholds(current: $0) } ?? thresholdPresets
        return [nil, 0] + percents.map(Optional.some)
    }

    public static func thresholdLabel(_ percent: Int?, general: Int, strings: UIStrings) -> String {
        switch percent {
        case nil: strings.fill(AdvancedOptionText.thresholdDefault, ["percent": "\(general)"])
        case 0: strings.text(AdvancedOptionText.thresholdOff)
        case .some(let value): strings.fill(AdvancedOptionText.thresholdPercent, ["percent": "\(value)"])
        }
    }
}
