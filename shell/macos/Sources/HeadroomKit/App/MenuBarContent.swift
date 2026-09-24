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
        let capacity = capacity(of: headline, in: state.combined)
        return .reading(text: text, fraction: min(1, max(0, percent / capacity)))
    }

    public static func subject(_ headline: Headline) -> String {
        guard headline.combined else { return headline.accountLabel ?? headline.providerName }
        return headline.accountCount.map { "\(headline.providerName) ×\($0)" } ?? headline.providerName
    }

    static func windowText(_ headline: Headline, formatter: DisplayFormatter) -> String {
        let window = formatter.windowLabel(id: headline.window, label: headline.windowLabel)
        return headline.combined ? "\(subject(headline)) · \(window)" : window
    }

    static func capacity(of headline: Headline, in groups: [CombinedGroup]) -> Double {
        guard headline.combined else { return 100 }
        let group = groups.first { $0.provider == headline.provider }
        let capacity = group?.windows.first { $0.id == headline.window }?.capacityPercent
        guard let capacity, capacity.isFinite, capacity > 0 else { return 100 }
        return capacity
    }
}
