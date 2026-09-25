import Foundation

public struct CompactLimitLine: Sendable, Hashable {
    public let trailing: String
    public let flame: Bool
    public let meterTip: String

    public static func make(
        _ row: QuotaRowModel, resetsAt: Timestamp?, display: DisplaySettings, now: Timestamp,
        formatter: DisplayFormatter
    ) -> CompactLimitLine {
        let tip = [row.trailing, row.note?.text, row.forecast].compactMap { $0 }
        return CompactLimitLine(
            trailing: trailing(resetsAt: resetsAt, format: display.resetFormat, now: now, formatter: formatter),
            flame: row.note?.flame ?? false, meterTip: tip.joined(separator: "\n"))
    }

    static func trailing(
        resetsAt: Timestamp?, format: ResetFormat, now: Timestamp, formatter: DisplayFormatter
    ) -> String {
        let strings = formatter.strings
        guard let resetsAt else { return strings.text(.notStarted) }
        let left = resetsAt.seconds(since: now)
        if left <= 0 { return strings.text(.resetPending) }
        if format == .exact { return formatter.compactMoment(resetsAt.date, now: now.date) }
        return formatter.duration(seconds: left, withSeconds: true)
    }
}

extension DisplayFormatter {
    static let compactWeek = 7

    public func compactMoment(_ date: Date, now: Date) -> String {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = timeZone
        let time = clockTime(date)
        let days =
            calendar.dateComponents([.day], from: calendar.startOfDay(for: now), to: calendar.startOfDay(for: date)).day
            ?? 0
        if days <= 0 { return time }
        if days < Self.compactWeek {
            return "\(strings.shortWeekday(calendar.component(.weekday, from: date) - 1)) \(time)"
        }
        let parts = calendar.dateComponents([.month, .day], from: date)
        let day = strings.fill(
            .monthDay, ["month": strings.month((parts.month ?? 1) - 1), "date": "\(parts.day ?? 1)"])
        return "\(day) \(time)"
    }
}
