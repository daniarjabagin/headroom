public struct CombinedGroup: Decodable, Sendable, Hashable {
    public let provider: String
    public let providerName: String
    public let accountIDs: [String]
    public let accounts: [CombinedAccount]
    public let windows: [CombinedWindow]

    enum CodingKeys: String, CodingKey {
        case provider, accounts, windows
        case providerName = "provider_name"
        case accountIDs = "account_ids"
    }
}

public struct CombinedAccount: Decodable, Sendable, Hashable {
    public let accountID: String
    public let label: String?
    public let plan: String?

    enum CodingKeys: String, CodingKey {
        case label, plan
        case accountID = "account_id"
    }
}

public struct CombinedWindow: Decodable, Sendable, Hashable, Identifiable {
    public let id: String
    public let label: String
    public let capacityPercent: Double
    public let remainingPercent: Double
    public let usedPercent: Double
    public let resetsAt: Timestamp?
    public let tone: Tone
    public let pace: Pace
    public let segments: [CombinedSegment]

    enum CodingKeys: String, CodingKey {
        case id, label, tone, pace, segments
        case capacityPercent = "capacity_percent"
        case remainingPercent = "remaining_percent"
        case usedPercent = "used_percent"
        case resetsAt = "resets_at"
    }
}

public struct CombinedSegment: Decodable, Sendable, Hashable, Identifiable {
    public let accountID: String
    public let label: String?
    public let remainingPercent: Double
    public let usedPercent: Double
    public let resetsAt: Timestamp?
    public let tone: Tone

    public var id: String { accountID }

    enum CodingKeys: String, CodingKey {
        case label, tone
        case accountID = "account_id"
        case remainingPercent = "remaining_percent"
        case usedPercent = "used_percent"
        case resetsAt = "resets_at"
    }
}
