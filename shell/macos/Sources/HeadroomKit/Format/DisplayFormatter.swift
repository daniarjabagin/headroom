import Foundation

public struct DisplayFormatter: Sendable {
    public static let dash = "—"

    public let strings: UIStrings
    public let timeZone: TimeZone

    public init(language: UILanguage, timeZone: TimeZone = .current) {
        strings = UIStrings(language: language)
        self.timeZone = timeZone
    }

    public var language: UILanguage { strings.language }

    public static func readingPercent(_ window: QuotaWindow, mode: ValueMode) -> Double {
        mode == .used ? window.usedPercent : window.remainingPercent
    }

    public static func readingPercent(_ headline: Headline, mode: ValueMode) -> Double {
        mode == .used ? headline.usedPercent : headline.remainingPercent
    }

    public func panelPercent(_ percent: Double) -> String {
        guard let whole = Self.roundedPercent(percent) else { return Self.dash }
        return "\(whole)%"
    }

    public func percentReading(_ percent: Double, mode: ValueMode) -> String {
        guard let whole = Self.roundedPercent(percent) else { return Self.dash }
        return strings.fill(mode == .used ? .percentUsed : .percentLeft, ["percent": "\(whole)"])
    }

    public func windowLabel(id: String, label: String) -> String {
        switch id {
        case "session": strings.text(.session)
        case "weekly": strings.text(.weekly)
        default: LabelTranslation.translate(label, language: strings.language)
        }
    }

    static func roundedPercent(_ percent: Double) -> Int64? {
        guard percent.isFinite, abs(percent) < 1e15 else { return nil }
        return max(0, Int64(percent.rounded(.toNearestOrAwayFromZero)))
    }
}
