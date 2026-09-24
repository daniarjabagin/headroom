public struct Settings: Codable, Sendable, Hashable {
    public var refreshIntervalSecs: Int64
    public var notifications: NotificationSettings
    public var headline: HeadlineSetting
    public var reducedMotion: Bool
    public var display: DisplaySettings

    enum CodingKeys: String, CodingKey {
        case notifications, headline, display
        case refreshIntervalSecs = "refresh_interval_secs"
        case reducedMotion = "reduced_motion"
    }
}

public struct NotificationSettings: Codable, Sendable, Hashable {
    public var almostOut: Bool
    public var cuttingItClose: Bool
    public var willRunOut: Bool
    public var reset: Bool

    enum CodingKeys: String, CodingKey {
        case reset
        case almostOut = "almost_out"
        case cuttingItClose = "cutting_it_close"
        case willRunOut = "will_run_out"
    }
}

public enum HeadlineSetting: Codable, Sendable, Hashable {
    case auto
    case pinned(accountID: String, window: String)

    enum CodingKeys: String, CodingKey {
        case mode, window
        case accountID = "account_id"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        guard try container.decode(String.self, forKey: .mode) == "pinned" else {
            self = .auto
            return
        }
        self = .pinned(
            accountID: try container.decode(String.self, forKey: .accountID),
            window: try container.decode(String.self, forKey: .window))
    }

    public func encode(to encoder: any Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .auto:
            try container.encode("auto", forKey: .mode)
        case .pinned(let accountID, let window):
            try container.encode("pinned", forKey: .mode)
            try container.encode(accountID, forKey: .accountID)
            try container.encode(window, forKey: .window)
        }
    }
}
