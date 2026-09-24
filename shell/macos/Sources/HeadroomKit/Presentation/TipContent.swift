public enum TipContent: Sendable, Hashable {
    case text(String)
    case breakdown(ModelBreakdown)
    case day(title: String, detail: String)
    case lines(title: String, lines: [String])
}

public struct ModelBreakdownRow: Sendable, Hashable {
    public let name: String
    public let tokens: String
    public let cost: String
}

public struct ModelBreakdown: Sendable, Hashable {
    static let partialMark = "*"

    public let title: String
    public let rows: [ModelBreakdownRow]
    public let total: String
    public let partialNote: String?

    public static func make(
        title: String, models: [ModelUsage], other: OtherModels?, costMicros: Int64, totalTokens: UInt64,
        formatter: DisplayFormatter
    ) -> ModelBreakdown? {
        guard !models.isEmpty || other != nil else { return nil }
        var entries = models.map {
            BreakdownEntry(name: $0.model, tokens: $0.totalTokens, cost: $0.costUSDMicros, partial: $0.partial)
        }
        if let other {
            let name = formatter.strings.fill(.otherModels, ["count": "\(other.count)"])
            entries.append(
                BreakdownEntry(name: name, tokens: other.totalTokens, cost: other.costUSDMicros, partial: other.partial)
            )
        }
        let partial = entries.contains { $0.partial }
        let note = "\(partialMark) \(formatter.strings.text(.partlyUnpriced))"
        return ModelBreakdown(
            title: title, rows: entries.map { $0.row(formatter) },
            total: "\(formatter.exactUSD(micros: costMicros)) · \(formatter.exactTokensText(totalTokens))",
            partialNote: partial ? note : nil)
    }
}

private struct BreakdownEntry {
    let name: String
    let tokens: UInt64
    let cost: Int64
    let partial: Bool

    func row(_ formatter: DisplayFormatter) -> ModelBreakdownRow {
        ModelBreakdownRow(
            name: partial ? "\(name) \(ModelBreakdown.partialMark)" : name,
            tokens: formatter.compactTokens(tokens),
            cost: partial && cost == 0 ? formatter.strings.text(.unpriced) : formatter.usd(micros: cost))
    }
}

extension DisplayFormatter {
    public func spendLine(costMicros: Int64, totalTokens: UInt64) -> String {
        if costMicros == 0 && totalTokens == 0 { return strings.text(.noData) }
        return "\(usd(micros: costMicros)) · \(compactTokensText(totalTokens))"
    }

    public func exactSpendLine(costMicros: Int64, totalTokens: UInt64, partial: Bool) -> String {
        let suffix = partial ? strings.text(.someModelsUnpriced) : ""
        return "\(exactUSD(micros: costMicros)) · \(exactTokensText(totalTokens))\(suffix)"
    }

    public func ringUSD(micros: Int64) -> String {
        let cents = Self.cents(of: micros)
        guard cents.magnitude >= 10_000, cents.magnitude < 1_000_000 else { return usd(micros: micros) }
        let dollars = (cents.magnitude + 50) / 100
        return "\(cents < 0 ? "-" : "")$\(dollars)"
    }
}
