extension SpendPeriodPreference: Identifiable {
    public var id: String { rawValue }

    public func popoverTitle(_ strings: UIStrings) -> String {
        self == .last30Days ? strings.text(.last30Days) : title(strings)
    }
}

extension SpendUnit: Identifiable {
    public var id: String { rawValue }
}

extension SpendBreakdown: Identifiable {
    public var id: String { rawValue }
}

public struct SpendSelection: Sendable, Hashable {
    public static let legacyDefault = SpendSelection(period: .today, unit: .cost, breakdown: .models)

    public var period: SpendPeriodPreference
    public var unit: SpendUnit
    public var breakdown: SpendBreakdown

    public init(period: SpendPeriodPreference, unit: SpendUnit, breakdown: SpendBreakdown) {
        self.period = period
        self.unit = unit
        self.breakdown = breakdown
    }

    public init(display: DisplaySettings) {
        self.init(period: display.spendPeriod, unit: display.spendUnit, breakdown: display.spendBreakdown)
    }

    public static func seed(display: DisplaySettings, features: DaemonFeatures) -> SpendSelection {
        features.release06 ? SpendSelection(display: display) : legacyDefault
    }

    public func effectivePeriod(in spend: Spend) -> SpendPeriodPreference {
        spend.period(period) == nil ? .last30Days : period
    }

    public static func change(from old: SpendSelection, to new: SpendSelection) -> [SettingsChange] {
        var changes: [SettingsChange] = []
        if old.period != new.period { changes.append(.spend(.period(new.period))) }
        if old.unit != new.unit { changes.append(.spend(.unit(new.unit))) }
        if old.breakdown != new.breakdown { changes.append(.spend(.breakdown(new.breakdown))) }
        return changes
    }
}
