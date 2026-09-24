import Foundation

extension DisplayFormatter {
    private static let minute: Int64 = 60
    private static let hour: Int64 = 3600
    private static let day: Int64 = 86_400
    private static let week = 7

    public func duration(seconds interval: TimeInterval, withSeconds: Bool = false) -> String {
        let total = Int64(max(0, interval.rounded(.down)))
        if withSeconds && total < Self.hour { return shortDuration(total) }
        let days = total / Self.day
        let hours = total % Self.day / Self.hour
        let minutes = total % Self.hour / Self.minute
        if days > 0 { return strings.fill(.daysHours, ["days": "\(days)", "hours": "\(hours)"]) }
        if hours > 0 { return strings.fill(.hoursMinutes, ["hours": "\(hours)", "minutes": "\(minutes)"]) }
        return strings.fill(.minutes, ["minutes": "\(max(1, minutes))"])
    }

    private func shortDuration(_ total: Int64) -> String {
        let minutes = total / Self.minute
        let seconds = total % Self.minute
        if minutes == 0 { return strings.fill(.seconds, ["seconds": "\(max(1, seconds))"]) }
        let padded = seconds < 10 ? "0\(seconds)" : "\(seconds)"
        return strings.fill(.minutesSeconds, ["minutes": "\(minutes)", "seconds": padded])
    }

    public func resetPhrase(
        resetsAt: Timestamp, now: Timestamp, format: ResetFormat, withSeconds: Bool = false
    ) -> String {
        let left = resetsAt.seconds(since: now)
        if left <= 0 { return strings.text(.resetPending) }
        if format == .exact {
            return strings.fill(.resetsMoment, ["moment": exactMoment(resetsAt.date, now: now.date)])
        }
        if left < (withSeconds ? 1 : 60) { return strings.text(.resetsSoon) }
        return strings.fill(.resetsIn, ["duration": duration(seconds: left, withSeconds: withSeconds)])
    }

    public func resetText(
        resetsAt: Timestamp?, now: Timestamp, format: ResetFormat, withSeconds: Bool = false
    ) -> String {
        guard let resetsAt else { return strings.text(.notStarted) }
        return capitalized(resetPhrase(resetsAt: resetsAt, now: now, format: format, withSeconds: withSeconds))
    }

    public func clockTime(_ date: Date) -> String {
        let parts = calendar.dateComponents([.hour, .minute], from: date)
        return "\(twoDigits(parts.hour ?? 0)):\(twoDigits(parts.minute ?? 0))"
    }

    public func exactMoment(_ date: Date, now: Date) -> String {
        let time = clockTime(date)
        let days = calendarDays(from: now, to: date)
        if days <= 0 { return strings.fill(.todayAt, ["time": time]) }
        if days == 1 { return strings.fill(.tomorrowAt, ["time": time]) }
        if days < Self.week {
            let weekday = calendar.component(.weekday, from: date) - 1
            return strings.fill(.weekdayAt, ["weekday": strings.weekdayOn(weekday), "time": time])
        }
        return strings.fill(.dayAt, ["day": monthDay(date), "time": time])
    }

    public func nextUpdateText(_ nextRefreshAt: Timestamp, now: Timestamp) -> String {
        let left = nextRefreshAt.seconds(since: now)
        if left < 60 { return strings.text(.nextUpdateSoon) }
        return strings.fill(.nextUpdateIn, ["duration": duration(seconds: left)])
    }

    public func agoText(_ date: Timestamp, now: Timestamp) -> String {
        let elapsed = now.seconds(since: date)
        if elapsed < 60 { return strings.text(.justNow) }
        return strings.fill(.ago, ["duration": duration(seconds: elapsed)])
    }

    private var calendar: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = timeZone
        return calendar
    }

    private func calendarDays(from start: Date, to end: Date) -> Int {
        let from = calendar.startOfDay(for: start)
        let to = calendar.startOfDay(for: end)
        return calendar.dateComponents([.day], from: from, to: to).day ?? 0
    }

    private func monthDay(_ date: Date) -> String {
        let parts = calendar.dateComponents([.month, .day], from: date)
        let month = strings.month((parts.month ?? 1) - 1)
        return strings.fill(.monthDay, ["month": month, "date": "\(parts.day ?? 1)"])
    }

    private func twoDigits(_ value: Int) -> String {
        value < 10 ? "0\(value)" : "\(value)"
    }

    private func capitalized(_ text: String) -> String {
        guard let first = text.first else { return text }
        return first.uppercased() + text.dropFirst()
    }
}
