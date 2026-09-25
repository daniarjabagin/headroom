public struct ModelPopoverRow: Sendable, Hashable, Identifiable {
    public let id: String
    public let name: String
    public let detail: String?
    public let cost: String
    public let fill: Double
    public let figures: String
}

public struct ModelPopover: Sendable, Hashable {
    public let title: String
    public let total: String
    public let color: SeriesColor
    public let rows: [ModelPopoverRow]
    public let footnotes: [String]

    public static func make(
        _ spend: ProviderSpend, period: SpendPeriodPreference, formatter: DisplayFormatter
    ) -> ModelPopover? {
        guard !spend.models.isEmpty || spend.modelsOther != nil else { return nil }
        let metric = SpendMetric.cost.basis(costMicros: spend.costUSDMicros, tokens: spend.totalTokens)
        let total = metric.value(costMicros: spend.costUSDMicros, tokens: spend.totalTokens)
        var rows = spend.models.map { model in
            row(
                id: "model:\(model.model)", name: model.model, detail: nil, cost: model.costUSDMicros,
                tokens: model.totalTokens, partial: model.partial, metric: metric, total: total, formatter: formatter)
        }
        if let other = spend.modelsOther {
            rows.append(
                row(
                    id: "other", name: formatter.strings.text(PopupExtraText.other),
                    detail: formatter.strings.fill(.otherModels, count: other.count), cost: other.costUSDMicros,
                    tokens: other.totalTokens, partial: other.partial, metric: metric, total: total,
                    formatter: formatter))
        }
        return ModelPopover(
            title: "\(spend.providerName) · \(period.popoverTitle(formatter.strings))",
            total: formatter.usd(micros: spend.costUSDMicros), color: ProviderStyle.seriesColor(for: spend.provider),
            rows: rows, footnotes: footnotes(spend, strings: formatter.strings))
    }

    static func footnotes(_ spend: ProviderSpend, strings: UIStrings) -> [String] {
        let folded = spend.modelsOther == nil ? nil : strings.text(PopupExtraText.modelsFolded)
        let unpriced = spend.partial ? strings.text(.spendUnpricedModels) : nil
        return [folded, strings.text(.spendEstimate), unpriced].compactMap { $0 }
    }

    private static func row(
        id: String, name: String, detail: String?, cost: Int64, tokens: UInt64, partial: Bool, metric: SpendMetric,
        total: UInt64, formatter: DisplayFormatter
    ) -> ModelPopoverRow {
        let part = metric.value(costMicros: cost, tokens: tokens)
        let permille = SpendShare.permille(part, of: total)
        let costText = partial && cost == 0 ? formatter.strings.text(.unpriced) : formatter.usd(micros: cost)
        return ModelPopoverRow(
            id: id, name: name, detail: detail, cost: costText, fill: SpendShare.fraction(part, of: total),
            figures: "\(SpendShare.wholePercent(permille)) · \(formatter.compactTokensText(tokens))")
    }
}
