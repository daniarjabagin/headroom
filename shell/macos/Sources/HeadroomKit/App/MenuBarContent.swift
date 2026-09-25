public enum MenuBarSlot: Sendable, Hashable {
    case mark(tone: Tone?)
    case item(MenuBarItem)

    public var tone: Tone? {
        switch self {
        case .mark(let tone): tone
        case .item(let item): item.tone
        }
    }

    public func pulses(reducedMotion: Bool) -> Bool {
        tone == .critical && !reducedMotion
    }
}

public enum MenuBarContent: Sendable, Hashable {
    case mark(tone: Tone?)
    case items([MenuBarItem])

    public static let glyph = MenuBarContent.mark(tone: nil)

    public var slots: [MenuBarSlot] {
        switch self {
        case .mark(let tone): [.mark(tone: tone)]
        case .items(let items): items.map(MenuBarSlot.item)
        }
    }

    public static func make(state: DaemonState?, formatter: DisplayFormatter) -> MenuBarContent {
        guard let state else { return .glyph }
        let display = state.display
        if display.panelMode == .icon || (display.panelIndicator == .none && display.panelLabel == .none) {
            return .mark(tone: state.panelTone)
        }
        let limit = display.panelMode == .several ? SettingsRules.maxPanelLimits : 1
        let items = state.panelItems.prefix(limit).map { MenuBarItem.make($0, state: state, formatter: formatter) }
        return items.isEmpty ? .glyph : .items(items)
    }

    public static func subject(_ item: PanelItem) -> String {
        guard item.combined else { return item.accountLabel ?? item.providerName }
        guard item.accountCount > 1 else { return item.providerName }
        return "\(item.providerName) ×\(item.accountCount)"
    }
}
