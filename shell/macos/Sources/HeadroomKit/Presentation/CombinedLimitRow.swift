public enum CombinedLimitRow: Sendable, Hashable, Identifiable {
    case single(QuotaRowModel)
    case pooled(CombinedRowModel)

    public var id: String {
        switch self {
        case .single(let row): row.id
        case .pooled(let row): row.id
        }
    }

    public static func make(
        _ window: CombinedWindow, members: [Account], display: DisplaySettings, now: Timestamp,
        formatter: DisplayFormatter
    ) -> CombinedLimitRow {
        guard isPooled(window) else {
            return .single(QuotaRowModel.make(quotaWindow(window), display: display, now: now, formatter: formatter))
        }
        return .pooled(
            CombinedRowModel.make(window, members: members, display: display, now: now, formatter: formatter))
    }

    public static func isPooled(_ window: CombinedWindow) -> Bool {
        window.segments.count > 1
    }

    static func quotaWindow(_ window: CombinedWindow) -> QuotaWindow {
        QuotaWindow(
            id: window.id, label: window.label, usedPercent: window.usedPercent,
            remainingPercent: window.remainingPercent, resetsAt: window.resetsAt, periodSeconds: nil,
            tone: window.tone, pace: window.pace, hidden: false)
    }
}
