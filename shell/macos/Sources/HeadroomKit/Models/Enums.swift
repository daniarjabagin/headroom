public protocol LenientStringEnum: RawRepresentable, Codable, Sendable, Hashable
where RawValue == String {
    static var fallback: Self { get }
}

extension LenientStringEnum {
    public init(from decoder: any Decoder) throws {
        let raw = try decoder.singleValueContainer().decode(String.self)
        self = Self(rawValue: raw) ?? Self.fallback
    }

    public func encode(to encoder: any Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(rawValue)
    }
}

public enum Tone: String, LenientStringEnum {
    case neutral, good, warning, critical
    public static let fallback = Tone.neutral
}

public enum PaceSeverity: String, LenientStringEnum {
    case untracked, healthy, close
    case runningOut = "running_out"
    case spent
    public static let fallback = PaceSeverity.untracked
}

public enum AccountStatus: String, LenientStringEnum {
    case refreshing
    case noSubscription = "no_subscription"
    case signedOut = "signed_out"
    case error, fresh, stale, unknown
    public static let fallback = AccountStatus.unknown
}

public enum DataSource: String, LenientStringEnum {
    case live
    case localLog = "local_log"
    case cache, unknown
    public static let fallback = DataSource.unknown
}

public enum AccountOwner: String, LenientStringEnum {
    case cli, headroom, unknown
    public static let fallback = AccountOwner.unknown
}

public enum BalanceKind: String, LenientStringEnum {
    case usd, money, count, unknown
    public static let fallback = BalanceKind.unknown
}

public enum AlertUrgency: String, LenientStringEnum {
    case low, normal, critical
    public static let fallback = AlertUrgency.normal
}
