public struct NotificationSettings: Decodable, Sendable, Hashable {
    public var almostOut: Bool
    public var cuttingItClose: Bool
    public var willRunOut: Bool
    public var reset: Bool
    public var thresholdPercent = SettingsRules.defaultThresholdPercent
    public var providerThresholds: [String: Int] = [:]
    public var quietHours = QuietHours.standard

    enum CodingKeys: String, CodingKey {
        case reset
        case almostOut = "almost_out"
        case cuttingItClose = "cutting_it_close"
        case willRunOut = "will_run_out"
        case thresholdPercent = "threshold_percent"
        case providerThresholds = "provider_thresholds"
        case quietHours = "quiet_hours"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        almostOut = try container.decode(Bool.self, forKey: .almostOut)
        cuttingItClose = try container.decode(Bool.self, forKey: .cuttingItClose)
        willRunOut = try container.decode(Bool.self, forKey: .willRunOut)
        reset = try container.decode(Bool.self, forKey: .reset)
        thresholdPercent = try container.value(.thresholdPercent, default: thresholdPercent)
        providerThresholds = try container.value(.providerThresholds, default: providerThresholds)
        quietHours = try container.value(.quietHours, default: quietHours)
    }

    public func threshold(provider: String) -> Int {
        providerThresholds[provider] ?? thresholdPercent
    }
}

public struct QuietHours: Decodable, Sendable, Hashable {
    public static let standard = QuietHours(
        enabled: false, from: TimeOfDay.at(22, 0), to: TimeOfDay.at(8, 0), allowCritical: true)

    public var enabled: Bool
    public var from: TimeOfDay
    public var to: TimeOfDay
    public var allowCritical: Bool

    public init(enabled: Bool, from: TimeOfDay, to: TimeOfDay, allowCritical: Bool) {
        self.enabled = enabled
        self.from = from
        self.to = to
        self.allowCritical = allowCritical
    }

    enum CodingKeys: String, CodingKey {
        case enabled, from, to
        case allowCritical = "allow_critical"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        enabled = try container.value(.enabled, default: Self.standard.enabled)
        from = try container.value(.from, default: Self.standard.from)
        to = try container.value(.to, default: Self.standard.to)
        allowCritical = try container.value(.allowCritical, default: Self.standard.allowCritical)
    }
}

public struct TimeOfDay: Decodable, Sendable, Hashable, Comparable {
    public let hour: Int
    public let minute: Int

    public init?(hour: Int, minute: Int) {
        guard (0..<24).contains(hour), (0..<60).contains(minute) else { return nil }
        self.hour = hour
        self.minute = minute
    }

    public init?(_ text: String) {
        let parts = text.split(separator: ":", omittingEmptySubsequences: false)
        guard parts.count == 2, parts.allSatisfy(Self.isTwoDigits),
            let hour = Int(parts[0]), let minute = Int(parts[1])
        else { return nil }
        self.init(hour: hour, minute: minute)
    }

    private init(checkedHour hour: Int, minute: Int) {
        self.hour = hour
        self.minute = minute
    }

    static func at(_ hour: Int, _ minute: Int) -> TimeOfDay {
        TimeOfDay(checkedHour: min(max(hour, 0), 23), minute: min(max(minute, 0), 59))
    }

    public var text: String { "\(Self.padded(hour)):\(Self.padded(minute))" }

    public static func < (lhs: TimeOfDay, rhs: TimeOfDay) -> Bool {
        (lhs.hour, lhs.minute) < (rhs.hour, rhs.minute)
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.singleValueContainer()
        let text = try container.decode(String.self)
        guard let parsed = TimeOfDay(text) else {
            throw DecodingError.dataCorruptedError(in: container, debugDescription: "not an HH:MM time: \(text)")
        }
        self = parsed
    }

    private static func isTwoDigits(_ part: Substring) -> Bool {
        part.count == 2 && part.allSatisfy { $0.isASCII && $0.isNumber }
    }

    private static func padded(_ value: Int) -> String {
        value < 10 ? "0\(value)" : "\(value)"
    }
}
