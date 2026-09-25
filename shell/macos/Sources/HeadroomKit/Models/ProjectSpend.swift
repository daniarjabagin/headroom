public struct ProjectSpend: Decodable, Sendable, Hashable {
    public let project: String?
    public let costUSDMicros: Int64
    public let totalTokens: UInt64
    public let partial: Bool
    public let sharePermille: Int64
    public let costPerMTokUSDMicros: Int64?
    public let byProvider: [ProjectProviderSpend]

    enum CodingKeys: String, CodingKey {
        case project, partial
        case costUSDMicros = "cost_usd_micros"
        case totalTokens = "total_tokens"
        case sharePermille = "share_permille"
        case costPerMTokUSDMicros = "cost_per_mtok_usd_micros"
        case byProvider = "by_provider"
    }
}

public struct ProjectProviderSpend: Decodable, Sendable, Hashable {
    public let provider: String
    public let providerName: String
    public let costUSDMicros: Int64
    public let totalTokens: UInt64

    enum CodingKeys: String, CodingKey {
        case provider
        case providerName = "provider_name"
        case costUSDMicros = "cost_usd_micros"
        case totalTokens = "total_tokens"
    }
}

public struct OtherProjects: Decodable, Sendable, Hashable {
    public let count: UInt64
    public let costUSDMicros: Int64
    public let totalTokens: UInt64
    public let partial: Bool
    public let sharePermille: Int64
    public let costPerMTokUSDMicros: Int64?

    enum CodingKeys: String, CodingKey {
        case count, partial
        case costUSDMicros = "cost_usd_micros"
        case totalTokens = "total_tokens"
        case sharePermille = "share_permille"
        case costPerMTokUSDMicros = "cost_per_mtok_usd_micros"
    }
}
