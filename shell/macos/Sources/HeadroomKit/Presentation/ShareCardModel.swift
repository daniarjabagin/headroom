import Foundation

public enum ShareText: LocalizedText {
    case left, used

    public var translations: (english: String, russian: String) {
        switch self {
        case .left: ("left", "осталось")
        case .used: ("used", "использовано")
        }
    }
}

public struct ShareRow: Sendable, Hashable, Identifiable {
    public let id: String
    public let label: String
    public let headline: String
    public let trailing: String
    public let percent: Double
    public let capacity: Double?
    public let tone: Tone
    public let segments: [SegmentModel]
}

public struct ShareCardModel: Sendable, Hashable {
    public static let wordmark = "headroom"

    public let provider: String
    public let providerName: String
    public let subtitle: String?
    public let stamp: String
    public let hero: String
    public let heroWord: String
    public let heroDetail: String
    public let heroMeters: [Double]
    public let rows: [ShareRow]
    public let tagline: String
    public let fileName: String

    public static func make(
        _ section: AccountSectionModel, state: DaemonState, now: Timestamp, formatter: DisplayFormatter
    ) -> ShareCardModel? {
        let rows = ShareRows.make(section, display: state.display, now: now, formatter: formatter)
        guard let heroRow = hero(rows, section: section, headline: state.headline) else { return nil }
        let members = state.accounts.filter { section.memberIDs.contains($0.id) }
        let providerName = members.first?.providerName ?? section.header.title
        let strings = formatter.strings
        let stamp = strings.fill(
            PopupExtraText.shareLimits, ["provider": providerName, "date": ShareDate.day(now.date, formatter)])
        return ShareCardModel(
            provider: section.provider, providerName: providerName,
            subtitle: subtitle(members, strings: strings), stamp: stamp.uppercased(),
            hero: formatter.panelPercent(heroRow.percent),
            heroWord: strings.text(state.display.valueMode == .used ? ShareText.used : .left),
            heroDetail: detail(heroRow, strings: strings), heroMeters: heroRow.segments.map(\.fill), rows: rows,
            tagline: strings.text(PopupExtraText.tagline),
            fileName: ShareDate.fileName(provider: section.provider, date: now.date, formatter: formatter))
    }

    public var text: String {
        let head = [providerName, subtitle].compactMap { $0 }.joined(separator: " ")
        let lines = rows.map { "\($0.label): \($0.headline) · \($0.trailing)" }
        return ([head] + lines).joined(separator: "\n")
    }

    static func hero(_ rows: [ShareRow], section: AccountSectionModel, headline: Headline?) -> ShareRow? {
        if let headline, section.memberIDs.contains(headline.accountID),
            let row = rows.first(where: { $0.id == headline.window })
        {
            return row
        }
        return rows.max { rank($0.tone) < rank($1.tone) } ?? rows.first
    }

    static func rank(_ tone: Tone) -> Int {
        switch tone {
        case .critical: 2
        case .warning: 1
        case .good, .neutral: 0
        }
    }

    static func subtitle(_ members: [Account], strings: UIStrings) -> String? {
        var seen: Set<String> = []
        let plans = members.compactMap(\.plan).filter { seen.insert($0).inserted }
        let count = members.count > 1 ? strings.fill(.accounts, count: UInt64(members.count)) : nil
        let parts = [count, plans.isEmpty ? nil : plans.joined(separator: " + ")].compactMap { $0 }
        return parts.isEmpty ? nil : "· " + parts.joined(separator: " · ")
    }

    static func detail(_ row: ShareRow, strings: UIStrings) -> String {
        let capacity = row.capacity.map {
            strings.fill(PopupExtraText.shareOf, ["capacity": QuotaRowModel.wholePercent($0)])
        }
        return [capacity, row.label, row.trailing].compactMap { $0 }.joined(separator: " · ")
    }
}

enum ShareRows {
    static func make(
        _ section: AccountSectionModel, display: DisplaySettings, now: Timestamp, formatter: DisplayFormatter
    ) -> [ShareRow] {
        switch section.body {
        case .blocked:
            return []
        case .limits(let limits):
            return limits.windows.map {
                single(QuotaRowModel.make($0, display: display, now: now, formatter: formatter))
            }
        case .combined(let limits):
            return limits.windows.map { window in
                switch CombinedLimitRow.make(
                    window, members: limits.members, display: display, now: now, formatter: formatter)
                {
                case .single(let row): single(row)
                case .pooled(let row): pooled(row, capacity: window.capacityPercent)
                }
            }
        }
    }

    static func single(_ row: QuotaRowModel) -> ShareRow {
        ShareRow(
            id: row.id, label: row.label, headline: row.headline, trailing: row.trailing, percent: row.percent,
            capacity: nil, tone: row.tone,
            segments: [SegmentModel(id: row.id, fill: row.fill, tick: row.tick, tone: row.tone)])
    }

    static func pooled(_ row: CombinedRowModel, capacity: Double) -> ShareRow {
        ShareRow(
            id: row.id, label: row.label, headline: row.headline, trailing: row.trailing, percent: row.percent,
            capacity: capacity, tone: row.tone, segments: row.segments)
    }
}

enum ShareDate {
    static func day(_ date: Date, _ formatter: DisplayFormatter) -> String {
        let parts = calendar(formatter).dateComponents([.year, .month, .day], from: date)
        return "\(parts.day ?? 1) \(formatter.strings.month((parts.month ?? 1) - 1)) \(parts.year ?? 1970)"
    }

    static func fileName(provider: String, date: Date, formatter: DisplayFormatter) -> String {
        let parts = calendar(formatter).dateComponents([.year, .month, .day, .hour, .minute], from: date)
        let stamp = [parts.year ?? 1970, parts.month ?? 1, parts.day ?? 1].map(padded).joined(separator: "-")
        let time = padded(parts.hour ?? 0) + padded(parts.minute ?? 0)
        let name = ProviderStyle.iconResource(for: provider) ?? "provider"
        return "headroom-\(name)-\(stamp)-\(time).png"
    }

    private static func calendar(_ formatter: DisplayFormatter) -> Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = formatter.timeZone
        return calendar
    }

    private static func padded(_ value: Int) -> String {
        value < 10 ? "0\(value)" : "\(value)"
    }
}
