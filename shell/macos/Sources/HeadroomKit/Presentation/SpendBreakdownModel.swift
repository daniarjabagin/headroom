public enum BreakdownIcon: Sendable, Hashable {
    case dot(SeriesColor)
    case folder
}

public struct BarSegment: Sendable, Hashable, Identifiable {
    public let id: String
    public let color: SeriesColor
    public let fraction: Double

    public init(id: String, color: SeriesColor, fraction: Double) {
        self.id = id
        self.color = color
        self.fraction = fraction
    }
}

public struct BreakdownRow: Sendable, Hashable, Identifiable {
    public let id: String
    public let icon: BreakdownIcon
    public let name: String
    public let detail: String?
    public let share: String
    public let value: String
    public let segments: [BarSegment]
}

public struct SpendBreakdownModel: Sendable, Hashable {
    static let projectNameLength = 26
    static let neutral = SeriesColor(light: 0x8E8E93, dark: 0x98989D)

    public let modes: [SpendBreakdown]
    public let mode: SpendBreakdown
    public let caption: String
    public let rows: [BreakdownRow]

    public var showsSwitch: Bool { modes.count > 1 }

    public static func make(
        _ period: PeriodSpend, mode: SpendBreakdown, unit: SpendUnit, formatter: DisplayFormatter
    ) -> SpendBreakdownModel? {
        guard let projects = period.projects else { return nil }
        let hasProjects = !projects.isEmpty || period.projectsOther != nil
        let hasModels = period.byProvider.contains { !$0.models.isEmpty || $0.modelsOther != nil }
        guard hasModels || hasProjects else { return nil }
        let modes: [SpendBreakdown] = [hasModels ? .models : nil, hasProjects ? .projects : nil].compactMap { $0 }
        let shown = modes.contains(mode) ? mode : modes.first ?? .models
        let context = BreakdownContext(period: period, unit: unit, formatter: formatter)
        switch shown {
        case .models:
            let list = ModelRanking.rank(period, metric: context.metric)
            return SpendBreakdownModel(
                modes: modes, mode: .models, caption: formatter.strings.fill(.models, count: list.count),
                rows: context.modelRows(list))
        case .projects:
            let count = UInt64(projects.count) + (period.projectsOther?.count ?? 0)
            return SpendBreakdownModel(
                modes: modes, mode: .projects, caption: formatter.strings.fill(.projects, count: count),
                rows: context.projectRows(projects, other: period.projectsOther))
        }
    }
}

struct BreakdownContext {
    let metric: SpendMetric
    let total: UInt64
    let listMetric: SpendMetric
    let formatter: DisplayFormatter

    init(period: PeriodSpend, unit: SpendUnit, formatter: DisplayFormatter) {
        listMetric = SpendMetric.forList(unit)
        metric = listMetric.basis(costMicros: period.costUSDMicros, tokens: period.totalTokens)
        total = metric.value(costMicros: period.costUSDMicros, tokens: period.totalTokens)
        self.formatter = formatter
    }

    func value(cost: Int64, tokens: UInt64) -> String {
        listMetric == .tokens ? formatter.compactTokens(tokens) : formatter.usd(micros: cost)
    }

    func share(cost: Int64, tokens: UInt64) -> String {
        let permille = SpendShare.permille(metric.value(costMicros: cost, tokens: tokens), of: total)
        return SpendShare.decimalPercent(permille, language: formatter.language)
    }

    func segment(_ id: String, color: SeriesColor, cost: Int64, tokens: UInt64) -> BarSegment {
        BarSegment(
            id: id, color: color,
            fraction: SpendShare.fraction(metric.value(costMicros: cost, tokens: tokens), of: total))
    }

    func modelRows(_ list: ModelRanking) -> [BreakdownRow] {
        let rows = list.shown.map { entry in
            BreakdownRow(
                id: "model:\(entry.provider):\(entry.model.model)",
                icon: .dot(ProviderStyle.seriesColor(for: entry.provider)), name: entry.model.model, detail: nil,
                share: share(cost: entry.model.costUSDMicros, tokens: entry.model.totalTokens),
                value: value(cost: entry.model.costUSDMicros, tokens: entry.model.totalTokens),
                segments: [
                    segment(
                        entry.provider, color: ProviderStyle.seriesColor(for: entry.provider),
                        cost: entry.model.costUSDMicros, tokens: entry.model.totalTokens)
                ])
        }
        guard let other = list.other else { return rows }
        return rows + [otherModelsRow(other)]
    }

    private func otherModelsRow(_ other: FoldedModels) -> BreakdownRow {
        let segments = other.parts.map { part in
            segment(
                part.provider, color: ProviderStyle.seriesColor(for: part.provider), cost: part.cost,
                tokens: part.tokens)
        }
        let leading = other.parts.max { $0.cost < $1.cost }.map { ProviderStyle.seriesColor(for: $0.provider) }
        return BreakdownRow(
            id: "model:other", icon: .dot(leading ?? SpendBreakdownModel.neutral),
            name: formatter.strings.text(PopupExtraText.other),
            detail: formatter.strings.fill(.otherModels, count: other.count),
            share: share(cost: other.cost, tokens: other.tokens), value: value(cost: other.cost, tokens: other.tokens),
            segments: segments)
    }

    func projectRows(_ projects: [ProjectSpend], other: OtherProjects?) -> [BreakdownRow] {
        let rows = projects.enumerated().map { index, project in
            BreakdownRow(
                id: "project:\(index):\(project.project ?? "")", icon: .folder,
                name: formatter.projectName(project.project, maxCharacters: SpendBreakdownModel.projectNameLength),
                detail: nil, share: share(cost: project.costUSDMicros, tokens: project.totalTokens),
                value: value(cost: project.costUSDMicros, tokens: project.totalTokens),
                segments: project.byProvider.map { part in
                    segment(
                        part.provider, color: ProviderStyle.seriesColor(for: part.provider), cost: part.costUSDMicros,
                        tokens: part.totalTokens)
                })
        }
        guard let other else { return rows }
        return rows + [
            BreakdownRow(
                id: "project:other", icon: .folder, name: formatter.strings.text(PopupExtraText.other),
                detail: formatter.strings.fill(.otherProjects, count: other.count),
                share: share(cost: other.costUSDMicros, tokens: other.totalTokens),
                value: value(cost: other.costUSDMicros, tokens: other.totalTokens),
                segments: [
                    segment(
                        "other", color: SpendBreakdownModel.neutral, cost: other.costUSDMicros,
                        tokens: other.totalTokens)
                ])
        ]
    }
}
