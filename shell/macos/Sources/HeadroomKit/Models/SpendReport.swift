public enum SpendGrouping: String, LenientStringEnum, CaseIterable {
    case model, project, provider, day
    public static let fallback = SpendGrouping.model
}

public enum SpendRange: Sendable, Hashable {
    case period(SpendPeriodPreference)
    case days(since: String, until: String?)
}

public struct SpendQuery: Encodable, Sendable, Hashable {
    public let range: SpendRange
    public let grouping: SpendGrouping
    public let provider: String?

    public init(range: SpendRange, grouping: SpendGrouping, provider: String? = nil) {
        self.range = range
        self.grouping = grouping
        self.provider = provider
    }

    enum CodingKeys: String, CodingKey {
        case period, since, until, by, provider
    }

    public func encode(to encoder: any Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch range {
        case .period(let period):
            try container.encode(period.rawValue, forKey: .period)
        case .days(let since, let until):
            try container.encode(since, forKey: .since)
            try container.encodeIfPresent(until, forKey: .until)
        }
        try container.encode(grouping.rawValue, forKey: .by)
        try container.encodeIfPresent(provider, forKey: .provider)
    }
}

public struct SpendReport: Decodable, Sendable, Hashable {
    public let since: String
    public let until: String
    public let by: SpendGrouping
    public let rows: [SpendRow]
    public let total: SpendRow
}

public struct SpendRow: Decodable, Sendable, Hashable {
    public let key: String?
    public let provider: String?
    public let tokens: TokenCounts
    public let costUSDMicros: Int64
    public let partial: Bool
    public let unpricedTokens: UInt64
    public let costPerMTokUSDMicros: Int64?
    public let sharePermille: Int64

    enum CodingKeys: String, CodingKey {
        case key, provider, tokens, partial
        case costUSDMicros = "cost_usd_micros"
        case unpricedTokens = "unpriced_tokens"
        case costPerMTokUSDMicros = "cost_per_mtok_usd_micros"
        case sharePermille = "share_permille"
    }
}
