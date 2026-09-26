public struct LegendEntry: Sendable, Hashable, Identifiable {
    public let provider: String
    public let name: String
    public let amount: String
    public let value: Int64
    public let tokensLine: String?
    public let color: SeriesColor
    public let tip: TipContent

    public var id: String { provider }
}

public struct SpendCardModel: Sendable, Hashable {
    public let period: SpendPeriodPreference
    public let unit: SpendUnit
    public let title: String
    public let centerAmount: String
    public let centerCaption: String?
    public let value: Int64
    public let fractions: [Double]
    public let entries: [LegendEntry]
    public let info: String
    public let breakdown: SpendBreakdownModel?

    public var isEmpty: Bool { entries.isEmpty }
    public var providerKey: [String] { entries.map(\.provider) }

    public static func shows(_ state: DaemonState) -> Bool {
        state.display.showSpend && !state.usage.isEmpty
    }

    public static func units(_ features: DaemonFeatures) -> [SpendUnit] {
        features.release06 ? SpendUnit.allCases : [.cost]
    }

    public static func make(
        spend: Spend, selection: SpendSelection, formatter: DisplayFormatter, showBreakdown: Bool = true
    ) -> SpendCardModel {
        let period = selection.effectivePeriod(in: spend)
        let totals = spend.period(period) ?? spend.last30Days
        let unit = selection.unit
        let withTokens = unit == .cost && totals.byProvider.count == 1
        let ring = SpendMetric.forRing(unit)
        let entries = totals.byProvider.map { providerSpend in
            legendEntry(providerSpend, unit: unit, withTokens: withTokens, period: period, formatter: formatter)
        }
        let center = center(totals, unit: unit, formatter: formatter)
        return SpendCardModel(
            period: period, unit: unit, title: unit.title(formatter.strings), centerAmount: center.amount,
            centerCaption: center.caption, value: center.value,
            fractions: DonutGeometry.visibleFractions(totals.byProvider.map { ringValue($0, ring) }),
            entries: entries, info: info(partial: totals.partial, strings: formatter.strings),
            breakdown: showBreakdown
                ? SpendBreakdownModel.make(totals, mode: selection.breakdown, unit: unit, formatter: formatter) : nil)
    }

    static func ringValue(_ spend: ProviderSpend, _ metric: SpendMetric) -> Int64 {
        Int64(clamping: metric.value(costMicros: spend.costUSDMicros, tokens: spend.totalTokens))
    }

    static func center(
        _ totals: PeriodSpend, unit: SpendUnit, formatter: DisplayFormatter
    ) -> (amount: String, caption: String?, value: Int64) {
        let strings = formatter.strings
        switch unit {
        case .cost:
            return (formatter.ringUSD(micros: totals.costUSDMicros), nil, totals.costUSDMicros)
        case .tokens:
            return (
                formatter.compactTokens(totals.totalTokens), strings.text(SpendOptionText.tokensCaption),
                Int64(clamping: totals.totalTokens)
            )
        case .costPerMTok:
            return (
                formatter.costPerMTok(micros: totals.costPerMTokUSDMicros),
                strings.text(SpendOptionText.blendedCaption),
                totals.costPerMTokUSDMicros ?? 0
            )
        }
    }

    static func amount(_ spend: ProviderSpend, unit: SpendUnit, formatter: DisplayFormatter) -> (String, Int64) {
        switch unit {
        case .cost: (formatter.usd(micros: spend.costUSDMicros), spend.costUSDMicros)
        case .tokens: (formatter.compactTokens(spend.totalTokens), Int64(clamping: spend.totalTokens))
        case .costPerMTok:
            (formatter.costPerMTok(micros: spend.costPerMTokUSDMicros), spend.costPerMTokUSDMicros ?? 0)
        }
    }

    private static func info(partial: Bool, strings: UIStrings) -> String {
        let estimate = strings.text(.spendEstimate)
        return partial ? "\(estimate) \(strings.text(.spendUnpricedModels))" : estimate
    }

    private static func legendEntry(
        _ spend: ProviderSpend, unit: SpendUnit, withTokens: Bool, period: SpendPeriodPreference,
        formatter: DisplayFormatter
    ) -> LegendEntry {
        let popover = ModelPopover.make(spend, period: period, formatter: formatter)
        let fallback = formatter.exactSpendLine(
            costMicros: spend.costUSDMicros, totalTokens: spend.totalTokens, partial: spend.partial)
        let (amount, value) = amount(spend, unit: unit, formatter: formatter)
        return LegendEntry(
            provider: spend.provider, name: spend.providerName, amount: amount, value: value,
            tokensLine: withTokens ? formatter.exactTokensText(spend.totalTokens) : nil,
            color: ProviderStyle.seriesColor(for: spend.provider),
            tip: popover.map(TipContent.models) ?? .text(fallback))
    }
}
