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
        let models = MergedModels.make(period)
        let hasModels = !models.isEmpty
        guard hasModels || hasProjects else { return nil }
        let modes: [SpendBreakdown] = [hasModels ? .models : nil, hasProjects ? .projects : nil].compactMap { $0 }
        let shown = modes.contains(mode) ? mode : modes.first ?? .models
        let context = BreakdownContext(period: period, unit: unit, formatter: formatter)
        switch shown {
        case .models:
            return SpendBreakdownModel(
                modes: modes, mode: .models, caption: formatter.strings.fill(.models, count: models.count),
                rows: models.rows.map(context.modelRow))
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
    let unit: SpendUnit
    let formatter: DisplayFormatter

    init(period: PeriodSpend, unit: SpendUnit, formatter: DisplayFormatter) {
        self.unit = unit
        metric = SpendMetric.forList(unit).basis(costMicros: period.costUSDMicros, tokens: period.totalTokens)
        total = metric.value(costMicros: period.costUSDMicros, tokens: period.totalTokens)
        self.formatter = formatter
    }

    func value(cost: Int64, tokens: UInt64, rate: Int64?) -> String {
        switch unit {
        case .cost: formatter.usd(micros: cost)
        case .tokens: formatter.compactTokens(tokens)
        case .costPerMTok: formatter.costPerMTok(micros: rate)
        }
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

    func modelRow(_ entry: MergedModelRow) -> BreakdownRow {
        let color = entry.provider.map(ProviderStyle.seriesColor(for:)) ?? SpendBreakdownModel.neutral
        return BreakdownRow(
            id: entry.id, icon: .dot(color), name: modelName(entry.label), detail: modelDetail(entry.label),
            share: share(cost: entry.costUSDMicros, tokens: entry.totalTokens),
            value: value(cost: entry.costUSDMicros, tokens: entry.totalTokens, rate: entry.costPerMTokUSDMicros),
            segments: [
                segment(
                    entry.provider ?? "other", color: color, cost: entry.costUSDMicros, tokens: entry.totalTokens)
            ])
    }

    private func modelName(_ label: MergedModelLabel) -> String {
        switch label {
        case .model(let name): name
        case .other: formatter.strings.text(PopupExtraText.other)
        }
    }

    private func modelDetail(_ label: MergedModelLabel) -> String? {
        switch label {
        case .model: nil
        case .other(let count): formatter.strings.fill(.otherModels, count: count)
        }
    }

    func projectRows(_ projects: [ProjectSpend], other: OtherProjects?) -> [BreakdownRow] {
        let rows = projects.enumerated().map { index, project in
            BreakdownRow(
                id: "project:\(index):\(project.project ?? "")", icon: .folder,
                name: formatter.projectName(project.project, maxCharacters: SpendBreakdownModel.projectNameLength),
                detail: nil, share: share(cost: project.costUSDMicros, tokens: project.totalTokens),
                value: value(
                    cost: project.costUSDMicros, tokens: project.totalTokens, rate: project.costPerMTokUSDMicros),
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
                value: value(
                    cost: other.costUSDMicros, tokens: other.totalTokens, rate: other.costPerMTokUSDMicros),
                segments: [
                    segment(
                        "other", color: SpendBreakdownModel.neutral, cost: other.costUSDMicros,
                        tokens: other.totalTokens)
                ])
        ]
    }
}
