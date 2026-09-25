public enum LogLevel: String, LenientStringEnum, CaseIterable {
    case error, warn, info, debug
    public static let fallback = LogLevel.info
}

public struct StatusPageSettings: Decodable, Sendable, Hashable {
    public var enabled = false

    enum CodingKeys: String, CodingKey {
        case enabled
    }

    public init() {}

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        enabled = try container.value(.enabled, default: enabled)
    }
}

public struct ShortcutSettings: Decodable, Sendable, Hashable {
    public var open = ""

    enum CodingKeys: String, CodingKey {
        case open
    }

    public init() {}

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        open = try container.value(.open, default: open)
    }
}

public struct LoggingSettings: Decodable, Sendable, Hashable {
    public var level = LogLevel.info

    enum CodingKeys: String, CodingKey {
        case level
    }

    public init() {}

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        level = try container.value(.level, default: level)
    }
}

public struct OnboardingSettings: Decodable, Sendable, Hashable {
    public var completed = false

    enum CodingKeys: String, CodingKey {
        case completed
    }

    public init() {}

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        completed = try container.value(.completed, default: completed)
    }
}
