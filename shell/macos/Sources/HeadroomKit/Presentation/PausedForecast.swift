public enum PausedText: LocalizedText {
    case paused, pausedLasts, aboutHours, aboutMinutes

    public var translations: (english: String, russian: String) {
        switch self {
        case .paused: ("Paused", "Пауза")
        case .pausedLasts: ("Paused · lasts {duration} of work", "Пауза · хватит {duration} работы")
        case .aboutHours: ("≈{hours} h", "≈{hours} ч")
        case .aboutMinutes: ("≈{minutes} min", "≈{minutes} мин")
        }
    }
}

enum PausedForecast {
    static let minute: UInt64 = 60
    static let hour: UInt64 = 3600

    static func text(_ pace: Pace, strings: UIStrings) -> String {
        guard let seconds = pace.activeLeftSeconds else { return strings.text(PausedText.paused) }
        return strings.fill(PausedText.pausedLasts, ["duration": approximate(seconds, strings: strings)])
    }

    static func approximate(_ seconds: UInt64, strings: UIStrings) -> String {
        let minutes = rounded(seconds, to: minute)
        guard minutes >= hour / minute else {
            return strings.fill(PausedText.aboutMinutes, ["minutes": "\(max(1, minutes))"])
        }
        return strings.fill(PausedText.aboutHours, ["hours": "\(rounded(seconds, to: hour))"])
    }

    private static func rounded(_ seconds: UInt64, to unit: UInt64) -> UInt64 {
        seconds / unit + (seconds % unit >= unit / 2 ? 1 : 0)
    }
}
