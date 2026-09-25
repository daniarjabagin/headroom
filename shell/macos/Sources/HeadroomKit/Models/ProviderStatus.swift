public enum StatusIndicator: String, LenientStringEnum {
    case none, minor, major, critical, maintenance, unknown
    public static let fallback = StatusIndicator.unknown
}

public struct ProviderStatus: Decodable, Sendable, Hashable, Identifiable {
    public let provider: String
    public let indicator: StatusIndicator
    public let tone: Tone
    public let title: String?
    public let stage: String?
    public let startedAt: Timestamp?
    public let url: String

    public var id: String { provider }
    public var isClear: Bool { indicator == .none }

    enum CodingKeys: String, CodingKey {
        case provider, indicator, tone, title, stage, url
        case startedAt = "started_at"
    }
}

public enum PollingMode: String, LenientStringEnum {
    case live, idle
    public static let fallback = PollingMode.idle
}

public enum PollingReason: String, LenientStringEnum {
    case hold, backoff, activity, schedule, unknown
    public static let fallback = PollingReason.unknown
}

public struct AccountRefresh: Decodable, Sendable, Hashable {
    public let mode: PollingMode
    public let intervalSecs: Int64
    public let nextAt: Timestamp?
    public let reason: PollingReason

    enum CodingKeys: String, CodingKey {
        case mode, reason
        case intervalSecs = "interval_secs"
        case nextAt = "next_at"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        mode = try container.decode(PollingMode.self, forKey: .mode)
        intervalSecs = try container.decode(Int64.self, forKey: .intervalSecs)
        nextAt = try container.decodeIfPresent(Timestamp.self, forKey: .nextAt)
        reason = try container.value(.reason, default: PollingReason.unknown)
    }
}
