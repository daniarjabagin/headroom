public struct Settings: Decodable, Sendable, Hashable {
    public var refreshIntervalSecs: Int64
    public var notifications: NotificationSettings
    public var headline: HeadlineSetting
    public var reducedMotion: Bool
    public var display: DisplaySettings
    public var adaptiveRefresh = true
    public var statusPages = StatusPageSettings()
    public var shortcuts = ShortcutSettings()
    public var logging = LoggingSettings()
    public var onboarding = OnboardingSettings()
    public private(set) var features = DaemonFeatures.legacy

    enum CodingKeys: String, CodingKey {
        case notifications, headline, display, shortcuts, logging, onboarding
        case refreshIntervalSecs = "refresh_interval_secs"
        case reducedMotion = "reduced_motion"
        case adaptiveRefresh = "adaptive_refresh"
        case statusPages = "status_pages"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        refreshIntervalSecs = try container.decode(Int64.self, forKey: .refreshIntervalSecs)
        notifications = try container.decode(NotificationSettings.self, forKey: .notifications)
        headline = try container.decode(HeadlineSetting.self, forKey: .headline)
        reducedMotion = try container.decode(Bool.self, forKey: .reducedMotion)
        display = try container.decode(DisplaySettings.self, forKey: .display)
        adaptiveRefresh = try container.value(.adaptiveRefresh, default: adaptiveRefresh)
        statusPages = try container.value(.statusPages, default: statusPages)
        shortcuts = try container.value(.shortcuts, default: shortcuts)
        logging = try container.value(.logging, default: logging)
        onboarding = try container.value(.onboarding, default: onboarding)
        features = container.contains(.adaptiveRefresh) ? .current : .legacy
    }
}

public enum HeadlineSetting: Decodable, Sendable, Hashable {
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
}
