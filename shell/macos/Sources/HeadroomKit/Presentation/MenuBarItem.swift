public enum MenuBarIndicator: Sendable, Hashable {
    case none
    case ring(fraction: Double)
    case bar(fraction: Double, tick: Double?)
}

public struct MenuBarItem: Sendable, Hashable, Identifiable {
    public let id: String
    public let logo: String?
    public let windowLetter: String?
    public let indicator: MenuBarIndicator
    public let text: String?
    public let tone: Tone
    public let stale: Bool
    public let summary: String

    static func make(_ item: PanelItem, state: DaemonState, formatter: DisplayFormatter) -> MenuBarItem {
        let display = state.display
        let showsLogo = display.panelMode == .several || display.panelLabel == .window
        let windowName = formatter.windowLabel(id: item.window, label: item.windowLabel)
        let reading = formatter.percentReading(item.valuePercent, mode: display.valueMode)
        return MenuBarItem(
            id: item.id,
            logo: showsLogo ? item.logo : nil,
            windowLetter: showsLogo ? windowLetter(windowName) : nil,
            indicator: indicator(item, display: display),
            text: display.panelLabel == .none ? nil : formatter.panelPercent(item.valuePercent),
            tone: item.tone,
            stale: isStale(item, accounts: state.accounts),
            summary: "\(MenuBarContent.subject(item)) · \(windowName): \(reading)")
    }

    static func indicator(_ item: PanelItem, display: DisplaySettings) -> MenuBarIndicator {
        let fraction = clamp(item.valuePercent / 100)
        switch display.panelIndicator {
        case .none: return .none
        case .ring: return .ring(fraction: fraction)
        case .bar: return .bar(fraction: fraction, tick: tick(item.evenPacePercent, mode: display.valueMode))
        }
    }

    static func tick(_ evenPacePercent: Double?, mode: ValueMode) -> Double? {
        guard let even = evenPacePercent, even.isFinite else { return nil }
        return clamp(mode == .used ? even / 100 : 1 - even / 100)
    }

    static func windowLetter(_ name: String) -> String {
        let word = name.split(whereSeparator: \.isWhitespace).first.map(String.init) ?? name
        guard word.count > 2, let first = word.first else { return word }
        return String(first).uppercased()
    }

    static func isStale(_ item: PanelItem, accounts: [Account]) -> Bool {
        guard !item.combined else { return false }
        return accounts.first { $0.id == item.accountID }?.status == .stale
    }

    private static func clamp(_ value: Double) -> Double {
        guard value.isFinite else { return 0 }
        return min(1, max(0, value))
    }
}
