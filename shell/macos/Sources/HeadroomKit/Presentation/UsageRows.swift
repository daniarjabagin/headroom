import Foundation

public struct ValueRowModel: Sendable, Hashable, Identifiable {
    public let id: String
    public let title: String
    public let value: String
    public let tip: TipContent?
}

public struct TrendBar: Sendable, Hashable, Identifiable {
    public let id: Int
    public let height: Double
    public let tip: TipContent?
}

public enum UsageRows {
    public static let trendDays = 30
    public static let trendHeight = 18.0
    static let trendStub = 2.0
    static let trendMinShare = 0.18

    public static func usage(for account: Account, in usage: [Usage]) -> Usage? {
        usage.first { $0.provider == account.provider && $0.usageHome == account.usageHome }
    }

    public static func spendRows(_ usage: Usage, providerName: String, formatter: DisplayFormatter) -> [ValueRowModel] {
        let strings = formatter.strings
        let periods: [(String, String, UsageTotals)] = [
            ("today", strings.text(.today), usage.today),
            ("yesterday", strings.text(.yesterday), usage.yesterday),
            ("last30Days", strings.text(.last30Days), usage.last30Days),
        ]
        return periods.map { id, title, totals in
            ValueRowModel(
                id: id, title: title,
                value: formatter.spendLine(costMicros: totals.costUSDMicros, totalTokens: totals.tokens.total),
                tip: totalsTip("\(title) · \(providerName)", totals, formatter))
        }
    }

    public static func balanceRows(_ balances: [Balance], formatter: DisplayFormatter) -> [ValueRowModel] {
        balances.map { balance in
            ValueRowModel(
                id: "balance:\(balance.id)", title: balanceTitle(balance, formatter.strings),
                value: balanceValue(balance, formatter), tip: nil)
        }
    }

    public static func trendBars(_ daily: [DailyUsage], formatter: DisplayFormatter) -> [TrendBar] {
        let days = Array(daily.suffix(trendDays))
        let padding = trendDays - days.count
        let peak = days.map(\.totalTokens).max() ?? 0
        return (0..<trendDays).map { index in
            guard index >= padding else { return TrendBar(id: index, height: trendStub, tip: nil) }
            let day = days[index - padding]
            return TrendBar(
                id: index, height: barHeight(day.totalTokens, peak: peak), tip: dayTip(day, formatter: formatter))
        }
    }

    static func barHeight(_ value: UInt64, peak: UInt64) -> Double {
        guard value > 0, peak > 0 else { return trendStub }
        let scaled = (trendHeight * Double(value) / Double(peak)).rounded()
        return max((trendHeight * trendMinShare).rounded(), scaled)
    }

    static func dayTip(_ day: DailyUsage, formatter: DisplayFormatter) -> TipContent {
        let strings = formatter.strings
        let figures =
            day.totalTokens == 0
            ? strings.text(.noUsage)
            : "\(formatter.compactTokensText(day.totalTokens)) · \(formatter.usd(micros: day.costUSDMicros))"
        let partial = day.partial ? strings.text(.someModelsUnpriced) : ""
        return .day(title: formatter.dayTitle(day.date), detail: "\(figures)\(partial)")
    }

    private static func totalsTip(_ title: String, _ totals: UsageTotals, _ formatter: DisplayFormatter) -> TipContent?
    {
        guard totals.tokens.total > 0 else { return nil }
        let breakdown = ModelBreakdown.make(
            title: title, models: totals.models, other: totals.modelsOther, costMicros: totals.costUSDMicros,
            totalTokens: totals.tokens.total, formatter: formatter)
        if let breakdown { return .breakdown(breakdown) }
        return .text(
            formatter.exactSpendLine(
                costMicros: totals.costUSDMicros, totalTokens: totals.tokens.total, partial: totals.partial))
    }

    private static func balanceTitle(_ balance: Balance, _ strings: UIStrings) -> String {
        LabelTranslation.translate(balance.label, language: strings.language)
    }

    private static func balanceValue(_ balance: Balance, _ formatter: DisplayFormatter) -> String {
        switch balance.kind {
        case .usd:
            if let micros = balance.usdMicros { return formatter.usd(micros: micros) }
        case .money:
            if let currency = balance.currency, let micros = balance.micros {
                return formatter.money(currency: currency, micros: micros)
            }
        case .count:
            if let value = balance.value {
                let sign = value < 0 ? "-" : ""
                let number = "\(sign)\(formatter.exactTokens(value.magnitude))"
                return [number, balance.unit ?? ""].filter { !$0.isEmpty }.joined(separator: " ")
            }
        case .unknown:
            break
        }
        return formatter.strings.text(.noData)
    }
}

extension DisplayFormatter {
    public func dayTitle(_ isoDate: String) -> String {
        let parts = isoDate.split(separator: "-").compactMap { Int($0) }
        guard parts.count == 3, isoDate.count == 10 else { return isoDate }
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "UTC") ?? timeZone
        let components = DateComponents(year: parts[0], month: parts[1], day: parts[2])
        guard components.isValidDate(in: calendar), let date = calendar.date(from: components) else { return isoDate }
        let weekday = strings.shortWeekday(calendar.component(.weekday, from: date) - 1)
        let day = strings.fill(.monthDay, ["month": strings.month(parts[1] - 1), "date": "\(parts[2])"])
        return strings.fill(.dayTitle, ["weekday": weekday, "day": day])
    }
}
