public struct SegmentModel: Sendable, Hashable, Identifiable {
    public let id: String
    public let fill: Double
    public let tick: Double?
    public let tone: Tone
}

public struct CombinedRowModel: Sendable, Hashable, Identifiable {
    public let id: String
    public let label: String
    public let percent: Double
    public let headline: String
    public let segments: [SegmentModel]
    public let tone: Tone
    public let note: PaceNote?
    public let trailing: String
    public let forecast: String?
    public let tip: TipContent

    public static func make(
        _ window: CombinedWindow, members: [Account], display: DisplaySettings, now: Timestamp,
        formatter: DisplayFormatter
    ) -> CombinedRowModel {
        let mode = display.valueMode
        let percent = reading(window, mode: mode)
        return CombinedRowModel(
            id: window.id, label: formatter.windowLabel(id: window.id, label: window.label), percent: percent,
            headline: headline(percent, capacity: window.capacityPercent, mode: mode, strings: formatter.strings),
            segments: window.segments.map { segment($0, window: window.id, members: members, display: display) },
            tone: window.tone,
            note: note(window.pace, now: now, showForecast: display.showForecast, formatter: formatter),
            trailing: formatter.resetText(
                resetsAt: window.resetsAt, now: now, format: display.resetFormat, withSeconds: true),
            forecast: display.showForecast ? forecast(window, now: now, display: display, formatter: formatter) : nil,
            tip: tip(window.segments, display: display, now: now, formatter: formatter))
    }

    static func reading(_ window: CombinedWindow, mode: ValueMode) -> Double {
        mode == .used ? window.usedPercent : window.remainingPercent
    }

    static func reading(_ segment: CombinedSegment, mode: ValueMode) -> Double {
        mode == .used ? segment.usedPercent : segment.remainingPercent
    }

    static func headline(_ percent: Double, capacity: Double, mode: ValueMode, strings: UIStrings) -> String {
        let values = [
            "percent": QuotaRowModel.wholePercent(percent), "capacity": QuotaRowModel.wholePercent(capacity),
        ]
        return strings.fill(mode == .used ? CombinedText.percentUsedOf : CombinedText.percentLeftOf, values)
    }

    static func segment(
        _ segment: CombinedSegment, window: String, members: [Account], display: DisplaySettings
    ) -> SegmentModel {
        let own = members.first { $0.id == segment.accountID }?.windows.first { $0.id == window }
        return SegmentModel(
            id: segment.accountID, fill: SegmentedMeter.clamp(reading(segment, mode: display.valueMode) / 100),
            tick: own.flatMap { QuotaRowModel.tick($0, display: display) }, tone: segment.tone)
    }

    static func note(_ pace: Pace, now: Timestamp, showForecast: Bool, formatter: DisplayFormatter) -> PaceNote? {
        guard pace.severity == .runningOut, pace.runsOutAt == nil else {
            return QuotaRowModel.note(pace, now: now, showForecast: showForecast, formatter: formatter)
        }
        return PaceNote(flame: true, text: formatter.strings.text(.overPace))
    }

    static func forecast(
        _ window: CombinedWindow, now: Timestamp, display: DisplaySettings, formatter: DisplayFormatter
    ) -> String? {
        let pace = window.pace
        if pace.isPaused { return PausedForecast.text(pace, strings: formatter.strings) }
        switch pace.severity {
        case .runningOut where pace.runsOutAt == nil:
            return formatter.strings.text(CombinedText.runsOutBeforeReset)
        case .healthy, .close:
            guard let spare = pace.sparePercent else { return nil }
            let projected = display.valueMode == .used ? pace.projectedPercent : nil
            let key = projected == nil ? CombinedText.paceLeftOfAtReset : CombinedText.paceUsedOfAtReset
            let values = [
                "percent": QuotaRowModel.wholePercent(projected ?? spare),
                "capacity": QuotaRowModel.wholePercent(window.capacityPercent),
            ]
            return formatter.strings.fill(key, values)
        case .runningOut, .spent, .untracked:
            return QuotaRowModel.forecast(
                pace, resetsAt: window.resetsAt, now: now, display: display, formatter: formatter)
        }
    }

    static func tip(
        _ segments: [CombinedSegment], display: DisplaySettings, now: Timestamp, formatter: DisplayFormatter
    ) -> TipContent {
        let title = segments.map { segment in
            "\(name(segment)) \(QuotaRowModel.wholePercent(reading(segment, mode: display.valueMode)))%"
        }
        let resets = segments.map { segment in
            let reset = formatter.resetText(resetsAt: segment.resetsAt, now: now, format: display.resetFormat)
            return "\(name(segment)) · \(reset)"
        }
        return .lines(title: title.joined(separator: " · "), lines: resets)
    }

    static func name(_ segment: CombinedSegment) -> String {
        segment.label ?? segment.accountID
    }
}
