enum MergedModelLabel: Sendable, Hashable {
    case model(String)
    case other(count: UInt64)
}

struct MergedModelRow: Sendable, Hashable {
    let id: String
    let provider: String?
    let label: MergedModelLabel
    let costUSDMicros: Int64
    let totalTokens: UInt64
    let costPerMTokUSDMicros: Int64?
}

struct MergedModels: Sendable, Hashable {
    let rows: [MergedModelRow]
    let count: UInt64

    var isEmpty: Bool { rows.isEmpty }

    static func make(_ period: PeriodSpend) -> MergedModels {
        guard let models = period.models else { return perProvider(period.byProvider) }
        let folded = period.modelsOther.map { other(provider: nil, $0, rate: $0.costPerMTokUSDMicros) }
        let rows = models.map(row) + [folded].compactMap { $0 }
        return MergedModels(rows: rows, count: UInt64(models.count) + (period.modelsOther?.count ?? 0))
    }

    static func perProvider(_ spends: [ProviderSpend]) -> MergedModels {
        let rows = spends.flatMap { spend in
            spend.models.map { row(spend.provider, $0) }
                + [spend.modelsOther.map { other(provider: spend.provider, $0, rate: nil) }].compactMap { $0 }
        }
        let count = spends.reduce(UInt64(0)) { sum, spend in
            sum + UInt64(spend.models.count) + (spend.modelsOther?.count ?? 0)
        }
        return MergedModels(rows: rows, count: count)
    }

    static func row(_ model: ProviderModelUsage) -> MergedModelRow {
        MergedModelRow(
            id: "model:\(model.provider):\(model.model)", provider: model.provider, label: .model(model.model),
            costUSDMicros: model.costUSDMicros, totalTokens: model.totalTokens,
            costPerMTokUSDMicros: model.costPerMTokUSDMicros)
    }

    static func row(_ provider: String, _ model: ModelUsage) -> MergedModelRow {
        MergedModelRow(
            id: "model:\(provider):\(model.model)", provider: provider, label: .model(model.model),
            costUSDMicros: model.costUSDMicros, totalTokens: model.totalTokens,
            costPerMTokUSDMicros: model.costPerMTokUSDMicros)
    }

    static func other(provider: String?, _ other: OtherModels, rate: Int64?) -> MergedModelRow {
        MergedModelRow(
            id: provider.map { "model:other:\($0)" } ?? "model:other", provider: provider,
            label: .other(count: other.count), costUSDMicros: other.costUSDMicros, totalTokens: other.totalTokens,
            costPerMTokUSDMicros: rate)
    }
}
