public enum MenuBarContent: Sendable, Hashable {
    case glyph
    case reading(text: String, fraction: Double)

    public static func make(state: DaemonState?, formatter: DisplayFormatter) -> MenuBarContent {
        guard let state, let headline = state.headline else { return .glyph }
        let percent = DisplayFormatter.readingPercent(headline, mode: state.display.valueMode)
        let text =
            switch state.display.panelLabel {
            case .percent: formatter.panelPercent(percent)
            case .window: formatter.windowLabel(id: headline.window, label: headline.windowLabel)
            }
        return .reading(text: text, fraction: min(1, max(0, percent / 100)))
    }
}
