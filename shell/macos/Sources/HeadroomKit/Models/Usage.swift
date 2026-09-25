public struct Usage: Decodable, Sendable, Hashable {
    public let provider: String
    public let providerName: String
    public let usageHome: String
    public let today: UsageTotals
    public let yesterday: UsageTotals
    public let last30Days: UsageTotals
    public let daily: [DailyUsage]

    enum CodingKeys: String, CodingKey {
        case provider, today, yesterday, daily
        case providerName = "provider_name"
        case usageHome = "usage_home"
        case last30Days = "last_30_days"
    }
}

public struct TokenCounts: Decodable, Sendable, Hashable {
    public let input: UInt64
    public let cacheRead: UInt64
    public let cacheWrite: UInt64
    public let output: UInt64
    public let reasoning: UInt64
    public let total: UInt64

    enum CodingKeys: String, CodingKey {
        case input, output, reasoning, total
        case cacheRead = "cache_read"
        case cacheWrite = "cache_write"
    }
}

public struct UsageTotals: Decodable, Sendable, Hashable {
    public let tokens: TokenCounts
    public let costUSDMicros: Int64
    public let partial: Bool
    public let unpricedTokens: UInt64
    public let unpricedModels: [String]
    public let models: [ModelUsage]
    public let modelsOther: OtherModels?

    enum CodingKeys: String, CodingKey {
        case tokens, partial, models
        case costUSDMicros = "cost_usd_micros"
        case unpricedTokens = "unpriced_tokens"
        case unpricedModels = "unpriced_models"
        case modelsOther = "models_other"
    }
}

public struct DailyUsage: Decodable, Sendable, Hashable {
    public let date: String
    public let totalTokens: UInt64
    public let costUSDMicros: Int64
    public let partial: Bool

    enum CodingKeys: String, CodingKey {
        case date, partial
        case totalTokens = "total_tokens"
        case costUSDMicros = "cost_usd_micros"
    }
}

public struct ModelUsage: Decodable, Sendable, Hashable {
    public let model: String
    public let totalTokens: UInt64
    public let costUSDMicros: Int64
    public let partial: Bool
    public let costPerMTokUSDMicros: Int64?

    enum CodingKeys: String, CodingKey {
        case model, partial
        case totalTokens = "total_tokens"
        case costUSDMicros = "cost_usd_micros"
        case costPerMTokUSDMicros = "cost_per_mtok_usd_micros"
    }
}

public struct OtherModels: Decodable, Sendable, Hashable {
    public let count: UInt64
    public let totalTokens: UInt64
    public let costUSDMicros: Int64
    public let partial: Bool

    enum CodingKeys: String, CodingKey {
        case count, partial
        case totalTokens = "total_tokens"
        case costUSDMicros = "cost_usd_micros"
    }
}

public struct Spend: Decodable, Sendable, Hashable {
    public let today: PeriodSpend
    public let yesterday: PeriodSpend
    public let last7Days: PeriodSpend?
    public let last30Days: PeriodSpend

    enum CodingKeys: String, CodingKey {
        case today, yesterday
        case last7Days = "last_7_days"
        case last30Days = "last_30_days"
    }

    public func period(_ period: SpendPeriodPreference) -> PeriodSpend? {
        switch period {
        case .today: today
        case .yesterday: yesterday
        case .last7Days: last7Days
        case .last30Days: last30Days
        }
    }
}

public struct PeriodSpend: Decodable, Sendable, Hashable {
    public let costUSDMicros: Int64
    public let totalTokens: UInt64
    public let partial: Bool
    public let byProvider: [ProviderSpend]
    public let costPerMTokUSDMicros: Int64?
    public let projects: [ProjectSpend]?
    public let projectsOther: OtherProjects?

    enum CodingKeys: String, CodingKey {
        case partial, projects
        case costUSDMicros = "cost_usd_micros"
        case totalTokens = "total_tokens"
        case byProvider = "by_provider"
        case costPerMTokUSDMicros = "cost_per_mtok_usd_micros"
        case projectsOther = "projects_other"
    }
}

public struct ProviderSpend: Decodable, Sendable, Hashable {
    public let provider: String
    public let providerName: String
    public let costUSDMicros: Int64
    public let totalTokens: UInt64
    public let partial: Bool
    public let models: [ModelUsage]
    public let modelsOther: OtherModels?
    public let costPerMTokUSDMicros: Int64?

    enum CodingKeys: String, CodingKey {
        case provider, partial, models
        case providerName = "provider_name"
        case costUSDMicros = "cost_usd_micros"
        case totalTokens = "total_tokens"
        case modelsOther = "models_other"
        case costPerMTokUSDMicros = "cost_per_mtok_usd_micros"
    }
}
