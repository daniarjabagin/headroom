import Foundation

public enum HourCycle: Sendable, Hashable {
    case twelveHour, twentyFourHour

    public static func resolve(_ format: TimeFormat, locale: Locale) -> HourCycle {
        switch format {
        case .twelveHour: .twelveHour
        case .twentyFourHour: .twentyFourHour
        case .auto: usesTwelveHour(locale) ? .twelveHour : .twentyFourHour
        }
    }

    static func usesTwelveHour(_ locale: Locale) -> Bool {
        let pattern = DateFormatter.dateFormat(fromTemplate: "j", options: 0, locale: locale) ?? "HH"
        return unquoted(pattern).contains { $0 == "h" || $0 == "K" }
    }

    private static func unquoted(_ pattern: String) -> String {
        var quoted = false
        return String(
            pattern.filter { character in
                if character == "'" { quoted.toggle() }
                return !quoted && character != "'"
            })
    }
}
