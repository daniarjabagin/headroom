struct RankedModel: Sendable, Hashable {
    let provider: String
    let model: ModelUsage
}

struct FoldedPart: Sendable, Hashable {
    let provider: String
    var cost: Int64
    var tokens: UInt64
}

struct FoldedModels: Sendable, Hashable {
    let count: UInt64
    let parts: [FoldedPart]

    var cost: Int64 { parts.reduce(0) { $0 + $1.cost } }
    var tokens: UInt64 { parts.reduce(0) { $0 + $1.tokens } }
}

struct ModelRanking: Sendable, Hashable {
    static let maxRows = 5
    static let minimumPermille: Int64 = 50

    let shown: [RankedModel]
    let other: FoldedModels?
    let count: UInt64

    static func rank(_ period: PeriodSpend, metric: SpendMetric) -> ModelRanking {
        let basis = metric.basis(costMicros: period.costUSDMicros, tokens: period.totalTokens)
        let total = basis.value(costMicros: period.costUSDMicros, tokens: period.totalTokens)
        let all = sorted(
            period.byProvider.flatMap { spend in spend.models.map { RankedModel(provider: spend.provider, model: $0) }
            }, basis)
        let kept = all.prefix(maxRows).filter { entry in
            SpendShare.permille(value(entry, basis), of: total) >= minimumPermille
        }
        let folded = all.filter { !kept.contains($0) }
        let others = period.byProvider.compactMap { spend in spend.modelsOther.map { (spend.provider, $0) } }
        let otherCount = UInt64(folded.count) + others.reduce(0) { $0 + $1.1.count }
        let parts = foldedParts(folded, others: others, order: period.byProvider.map(\.provider))
        return ModelRanking(
            shown: Array(kept), other: otherCount == 0 ? nil : FoldedModels(count: otherCount, parts: parts),
            count: UInt64(all.count) + others.reduce(0) { $0 + $1.1.count })
    }

    static func value(_ entry: RankedModel, _ metric: SpendMetric) -> UInt64 {
        metric.value(costMicros: entry.model.costUSDMicros, tokens: entry.model.totalTokens)
    }

    static func sorted(_ entries: [RankedModel], _ metric: SpendMetric) -> [RankedModel] {
        entries.sorted { lhs, rhs in
            let left = value(lhs, metric)
            let right = value(rhs, metric)
            if left != right { return left > right }
            if lhs.model.totalTokens != rhs.model.totalTokens { return lhs.model.totalTokens > rhs.model.totalTokens }
            return lhs.model.model < rhs.model.model
        }
    }

    private static func foldedParts(
        _ folded: [RankedModel], others: [(String, OtherModels)], order: [String]
    ) -> [FoldedPart] {
        var parts: [String: FoldedPart] = [:]
        for entry in folded {
            parts[entry.provider, default: FoldedPart(provider: entry.provider, cost: 0, tokens: 0)].add(
                cost: entry.model.costUSDMicros, tokens: entry.model.totalTokens)
        }
        for (provider, other) in others {
            parts[provider, default: FoldedPart(provider: provider, cost: 0, tokens: 0)].add(
                cost: other.costUSDMicros, tokens: other.totalTokens)
        }
        return order.compactMap { parts[$0] }
    }
}

extension FoldedPart {
    mutating func add(cost: Int64, tokens: UInt64) {
        self.cost += cost
        self.tokens += tokens
    }
}
