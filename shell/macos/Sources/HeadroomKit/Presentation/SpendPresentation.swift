public enum SpendPeriod: String, Sendable, CaseIterable, Identifiable {
    case today, yesterday, last30Days

    public var id: String { rawValue }

    public func title(_ strings: UIStrings) -> String {
        switch self {
        case .today: strings.text(.today)
        case .yesterday: strings.text(.yesterday)
        case .last30Days: strings.text(.thirtyDays)
        }
    }

    public func spend(in spend: Spend) -> PeriodSpend {
        switch self {
        case .today: spend.today
        case .yesterday: spend.yesterday
        case .last30Days: spend.last30Days
        }
    }
}

public struct LegendEntry: Sendable, Hashable, Identifiable {
    public let provider: String
    public let name: String
    public let amount: String
    public let costMicros: Int64
    public let tokensLine: String?
    public let color: SeriesColor
    public let tip: TipContent

    public var id: String { provider }
}

public struct SpendCardModel: Sendable, Hashable {
    public let period: SpendPeriod
    public let centerAmount: String
    public let costMicros: Int64
    public let fractions: [Double]
    public let entries: [LegendEntry]
    public let info: String

    public var isEmpty: Bool { entries.isEmpty }
    public var providerKey: [String] { entries.map(\.provider) }

    public static func shows(_ state: DaemonState) -> Bool {
        state.display.showSpend && !state.usage.isEmpty
    }

    public static func make(spend: Spend, period: SpendPeriod, formatter: DisplayFormatter) -> SpendCardModel {
        let totals = period.spend(in: spend)
        let withTokens = totals.byProvider.count == 1
        let periodTitle = period.title(formatter.strings)
        let entries = totals.byProvider.map { providerSpend in
            legendEntry(providerSpend, withTokens: withTokens, periodTitle: periodTitle, formatter: formatter)
        }
        return SpendCardModel(
            period: period, centerAmount: formatter.ringUSD(micros: totals.costUSDMicros),
            costMicros: totals.costUSDMicros,
            fractions: DonutGeometry.visibleFractions(totals.byProvider.map(\.costUSDMicros)),
            entries: entries, info: info(partial: totals.partial, strings: formatter.strings))
    }

    private static func info(partial: Bool, strings: UIStrings) -> String {
        let estimate = strings.text(.spendEstimate)
        return partial ? "\(estimate) \(strings.text(.spendUnpricedModels))" : estimate
    }

    private static func legendEntry(
        _ spend: ProviderSpend, withTokens: Bool, periodTitle: String, formatter: DisplayFormatter
    ) -> LegendEntry {
        let breakdown = ModelBreakdown.make(
            title: "\(periodTitle) · \(spend.providerName)", models: spend.models, other: spend.modelsOther,
            costMicros: spend.costUSDMicros, totalTokens: spend.totalTokens, formatter: formatter)
        let fallback = formatter.exactSpendLine(
            costMicros: spend.costUSDMicros, totalTokens: spend.totalTokens, partial: spend.partial)
        return LegendEntry(
            provider: spend.provider, name: spend.providerName, amount: formatter.usd(micros: spend.costUSDMicros),
            costMicros: spend.costUSDMicros,
            tokensLine: withTokens ? formatter.exactTokensText(spend.totalTokens) : nil,
            color: ProviderStyle.seriesColor(for: spend.provider),
            tip: breakdown.map(TipContent.breakdown) ?? .text(fallback))
    }
}
