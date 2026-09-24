public enum MenuBarContent: Sendable, Hashable {
    case glyph
    case reading(text: String, fraction: Double)

    public static func make(state: DaemonState?, formatter: DisplayFormatter) -> MenuBarContent {
        guard let state, let headline = state.headline else { return .glyph }
        let percent = DisplayFormatter.readingPercent(headline, mode: state.display.valueMode)
        let text =
            switch state.display.panelLabel {
            case .percent: formatter.panelPercent(percent)
            case .window: windowText(headline, formatter: formatter)
            }
        return .reading(text: text, fraction: min(1, max(0, percent / 100)))
    }

    public static func subject(_ headline: Headline) -> String {
        guard headline.combined else { return headline.accountLabel ?? headline.providerName }
        guard let count = headline.accountCount, count > 1 else { return headline.providerName }
        return "\(headline.providerName) ×\(count)"
    }

    static func windowText(_ headline: Headline, formatter: DisplayFormatter) -> String {
        let window = formatter.windowLabel(id: headline.window, label: headline.windowLabel)
        return headline.combined ? "\(subject(headline)) · \(window)" : window
    }
}
