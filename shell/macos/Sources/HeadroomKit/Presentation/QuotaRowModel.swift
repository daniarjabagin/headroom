public struct PaceNote: Sendable, Hashable {
    public let flame: Bool
    public let text: String
}

public struct QuotaRowModel: Sendable, Hashable, Identifiable {
    static let liveCountdownSeconds: Double = 3600

    public let id: String
    public let label: String
    public let percent: Double
    public let headline: String
    public let fill: Double
    public let tick: Double?
    public let tone: Tone
    public let note: PaceNote?
    public let trailing: String
    public let forecast: String?

    public static func make(
        _ window: QuotaWindow, display: DisplaySettings, now: Timestamp, formatter: DisplayFormatter
    ) -> QuotaRowModel {
        let percent = DisplayFormatter.readingPercent(window, mode: display.valueMode)
        return QuotaRowModel(
            id: window.id, label: formatter.windowLabel(id: window.id, label: window.label), percent: percent,
            headline: formatter.percentReading(percent, mode: display.valueMode),
            fill: min(1, max(0, percent / 100)), tick: tick(window, display: display), tone: window.tone,
            note: note(window, now: now, showForecast: display.showForecast, formatter: formatter),
            trailing: formatter.resetText(
                resetsAt: window.resetsAt, now: now, format: display.resetFormat, withSeconds: true),
            forecast: display.showForecast ? forecast(window, now: now, display: display, formatter: formatter) : nil)
    }

    public static func needsSecondTicks(_ window: QuotaWindow, display: DisplaySettings, now: Timestamp) -> Bool {
        guard display.resetFormat == .countdown, let resetsAt = window.resetsAt else { return false }
        let left = resetsAt.seconds(since: now)
        return left > 0 && left < liveCountdownSeconds
    }

    static func tick(_ window: QuotaWindow, display: DisplaySettings) -> Double? {
        guard let even = window.pace.evenPacePercent, even.isFinite else { return nil }
        let urgent = window.tone == .warning || window.tone == .critical
        guard display.showForecast || urgent else { return nil }
        let position = display.valueMode == .used ? even / 100 : 1 - even / 100
        return min(1, max(0, position))
    }

    static func note(
        _ window: QuotaWindow, now: Timestamp, showForecast: Bool, formatter: DisplayFormatter
    ) -> PaceNote? {
        note(window.pace, now: now, showForecast: showForecast, formatter: formatter)
    }

    static func note(_ pace: Pace, now: Timestamp, showForecast: Bool, formatter: DisplayFormatter) -> PaceNote? {
        let strings = formatter.strings
        switch pace.severity {
        case .spent:
            return PaceNote(flame: true, text: strings.text(.limitReached))
        case .runningOut:
            let text = showForecast ? strings.text(.overPace) : limitText(pace.runsOutAt, now, formatter)
            return PaceNote(flame: true, text: text)
        case .close:
            guard let spare = pace.sparePercent, !showForecast else { return nil }
            return PaceNote(flame: false, text: spareText(spare, strings))
        case .healthy, .untracked:
            return nil
        }
    }

    static func forecast(
        _ window: QuotaWindow, now: Timestamp, display: DisplaySettings, formatter: DisplayFormatter
    ) -> String? {
        forecast(window.pace, resetsAt: window.resetsAt, now: now, display: display, formatter: formatter)
    }

    static func forecast(
        _ pace: Pace, resetsAt: Timestamp?, now: Timestamp, display: DisplaySettings, formatter: DisplayFormatter
    ) -> String? {
        switch pace.severity {
        case .runningOut:
            return runOutForecast(pace, resetsAt: resetsAt, now: now, format: display.resetFormat, formatter: formatter)
        case .healthy, .close:
            guard let spare = pace.sparePercent else { return nil }
            return atResetForecast(pace, spare: spare, mode: display.valueMode, strings: formatter.strings)
        case .spent, .untracked:
            return nil
        }
    }

    private static func limitText(_ runsOutAt: Timestamp?, _ now: Timestamp, _ formatter: DisplayFormatter) -> String {
        guard let runsOutAt, runsOutAt > now else { return formatter.strings.text(.limitSoon) }
        return formatter.strings.fill(
            .limitIn, ["duration": formatter.duration(seconds: runsOutAt.seconds(since: now))])
    }

    private static func spareText(_ spare: Double, _ strings: UIStrings) -> String {
        strings.fill(.spare, ["percent": wholePercent(spare)])
    }

    private static func runOutForecast(
        _ pace: Pace, resetsAt: Timestamp?, now: Timestamp, format: ResetFormat, formatter: DisplayFormatter
    ) -> String {
        let strings = formatter.strings
        guard let runsOutAt = pace.runsOutAt, runsOutAt > now else { return strings.text(.runsOutAnyMinute) }
        let runsOut = strings.fill(.runsOutIn, ["duration": formatter.duration(seconds: runsOutAt.seconds(since: now))])
        guard let resetsAt else { return strings.fill(.paceRunsOut, ["runsOut": runsOut]) }
        let resets = formatter.resetPhrase(resetsAt: resetsAt, now: now, format: format)
        return strings.fill(.paceRunsOutResets, ["runsOut": runsOut, "resets": resets])
    }

    private static func atResetForecast(_ pace: Pace, spare: Double, mode: ValueMode, strings: UIStrings) -> String {
        if mode == .used, let projected = pace.projectedPercent {
            return strings.fill(.paceUsedAtReset, ["percent": wholePercent(projected)])
        }
        return strings.fill(.paceLeftAtReset, ["percent": wholePercent(spare)])
    }

    static func wholePercent(_ value: Double) -> String {
        DisplayFormatter.roundedPercent(value).map { "\($0)" } ?? DisplayFormatter.dash
    }
}
