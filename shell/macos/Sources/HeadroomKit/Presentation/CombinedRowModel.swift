public struct SegmentModel: Sendable, Hashable, Identifiable {
    public let id: String
    public let fill: Double
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
        _ window: CombinedWindow, display: DisplaySettings, now: Timestamp, formatter: DisplayFormatter
    ) -> CombinedRowModel {
        let mode = display.valueMode
        let percent = reading(window, mode: mode)
        return CombinedRowModel(
            id: window.id, label: formatter.windowLabel(id: window.id, label: window.label), percent: percent,
            headline: headline(percent, capacity: window.capacityPercent, mode: mode, strings: formatter.strings),
            segments: window.segments.map { segment($0, mode: mode) }, tone: window.tone,
            note: QuotaRowModel.note(window.pace, now: now, showForecast: display.showForecast, formatter: formatter),
            trailing: formatter.resetText(
                resetsAt: window.resetsAt, now: now, format: display.resetFormat, withSeconds: true),
            forecast: display.showForecast
                ? QuotaRowModel.forecast(
                    window.pace, resetsAt: window.resetsAt, now: now, display: display, formatter: formatter) : nil,
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

    static func segment(_ segment: CombinedSegment, mode: ValueMode) -> SegmentModel {
        SegmentModel(
            id: segment.accountID, fill: SegmentedMeter.clamp(reading(segment, mode: mode) / 100), tone: segment.tone)
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
