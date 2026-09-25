public enum Density: String, LenientStringEnum, CaseIterable {
    case normal, compact
    public static let fallback = Density.normal
}

public enum TimeFormat: String, LenientStringEnum, CaseIterable {
    case auto
    case twelveHour = "12h"
    case twentyFourHour = "24h"
    public static let fallback = TimeFormat.auto
}

public enum PanelMode: String, LenientStringEnum, CaseIterable {
    case headline, several, icon
    public static let fallback = PanelMode.headline
}

public enum PanelIndicator: String, LenientStringEnum, CaseIterable {
    case ring, bar, none
    public static let fallback = PanelIndicator.ring
}

public enum PanelBox: String, LenientStringEnum, CaseIterable {
    case left, center, right
    public static let fallback = PanelBox.right
}

public enum SpendPeriodPreference: String, LenientStringEnum, CaseIterable {
    case today, yesterday
    case last7Days = "7d"
    case last30Days = "30d"
    public static let fallback = SpendPeriodPreference.last30Days
}

public enum SpendUnit: String, LenientStringEnum, CaseIterable {
    case cost, tokens
    case costPerMTok = "cost_per_mtok"
    public static let fallback = SpendUnit.cost
}

public enum SpendBreakdown: String, LenientStringEnum, CaseIterable {
    case models, projects
    public static let fallback = SpendBreakdown.models
}

public struct PanelLimit: Decodable, Sendable, Hashable {
    public let accountID: String
    public let window: String

    public init(accountID: String, window: String) {
        self.accountID = accountID
        self.window = window
    }

    enum CodingKeys: String, CodingKey {
        case window
        case accountID = "account_id"
    }
}

public struct PanelPosition: Decodable, Sendable, Hashable {
    public static let standard = PanelPosition(box: .right, index: 0)

    public let box: PanelBox
    public let index: UInt32

    public init(box: PanelBox, index: UInt32) {
        self.box = box
        self.index = index
    }
}

extension KeyedDecodingContainer {
    func value<Value: Decodable>(_ key: Key, default fallback: Value) throws -> Value {
        try decodeIfPresent(Value.self, forKey: key) ?? fallback
    }
}
