public struct DaemonState: Decodable, Sendable, Hashable {
    public static let schemaVersion = 1

    public let version: Int
    public let generatedAt: Timestamp
    public let nextRefreshAt: Timestamp?
    public let lastSuccessAt: Timestamp?
    public let offline: Bool
    public let display: DisplaySettings
    public let headline: Headline?
    public let accounts: [Account]
    public let usage: [Usage]
    public let spend: Spend
    public let appVersion: String?
    public let combined: [CombinedGroup]

    enum CodingKeys: String, CodingKey {
        case version, offline, display, headline, accounts, usage, spend, combined
        case appVersion = "app_version"
        case generatedAt = "generated_at"
        case nextRefreshAt = "next_refresh_at"
        case lastSuccessAt = "last_success_at"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        version = try container.decode(Int.self, forKey: .version)
        generatedAt = try container.decode(Timestamp.self, forKey: .generatedAt)
        nextRefreshAt = try container.decodeIfPresent(Timestamp.self, forKey: .nextRefreshAt)
        lastSuccessAt = try container.decodeIfPresent(Timestamp.self, forKey: .lastSuccessAt)
        offline = try container.decode(Bool.self, forKey: .offline)
        display = try container.decode(DisplaySettings.self, forKey: .display)
        headline = try container.decodeIfPresent(Headline.self, forKey: .headline)
        accounts = try container.decode([Account].self, forKey: .accounts)
        usage = try container.decode([Usage].self, forKey: .usage)
        spend = try container.decode(Spend.self, forKey: .spend)
        appVersion = try container.decodeIfPresent(String.self, forKey: .appVersion)
        combined = try container.decodeIfPresent([CombinedGroup].self, forKey: .combined) ?? []
    }
}

struct SchemaHeader: Decodable {
    let version: Int
}

public struct Headline: Decodable, Sendable, Hashable {
    public let accountID: String
    public let provider: String
    public let providerName: String
    public let accountLabel: String?
    public let window: String
    public let windowLabel: String
    public let usedPercent: Double
    public let remainingPercent: Double
    public let tone: Tone
    public let combined: Bool
    public let accountCount: Int?

    enum CodingKeys: String, CodingKey {
        case provider, window, tone, combined
        case accountID = "account_id"
        case providerName = "provider_name"
        case accountLabel = "account_label"
        case windowLabel = "window_label"
        case usedPercent = "used_percent"
        case remainingPercent = "remaining_percent"
        case accountCount = "account_count"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        accountID = try container.decode(String.self, forKey: .accountID)
        provider = try container.decode(String.self, forKey: .provider)
        providerName = try container.decode(String.self, forKey: .providerName)
        accountLabel = try container.decodeIfPresent(String.self, forKey: .accountLabel)
        window = try container.decode(String.self, forKey: .window)
        windowLabel = try container.decode(String.self, forKey: .windowLabel)
        usedPercent = try container.decode(Double.self, forKey: .usedPercent)
        remainingPercent = try container.decode(Double.self, forKey: .remainingPercent)
        tone = try container.decode(Tone.self, forKey: .tone)
        combined = try container.decodeIfPresent(Bool.self, forKey: .combined) ?? false
        accountCount = try container.decodeIfPresent(Int.self, forKey: .accountCount)
    }
}

public struct Account: Decodable, Sendable, Hashable, Identifiable {
    public let id: String
    public let provider: String
    public let providerName: String
    public let label: String?
    public let email: String?
    public let plan: String?
    public let hidden: Bool
    public let owner: AccountOwner
    public let status: AccountStatus
    public let error: AccountError?
    public let updatedAt: Timestamp?
    public let source: DataSource?
    public let windows: [QuotaWindow]
    public let balances: [Balance]
    public let notices: [Notice]
    public let usageHome: String

    public var displayName: String { label ?? email ?? providerName }

    enum CodingKeys: String, CodingKey {
        case id, provider, label, email, plan, hidden, owner, status, error, source, windows,
            balances, notices
        case providerName = "provider_name"
        case updatedAt = "updated_at"
        case usageHome = "usage_home"
    }
}

public struct AccountError: Decodable, Sendable, Hashable {
    public let kind: String
    public let message: String
}

public struct QuotaWindow: Decodable, Sendable, Hashable, Identifiable {
    public let id: String
    public let label: String
    public let usedPercent: Double
    public let remainingPercent: Double
    public let resetsAt: Timestamp?
    public let periodSeconds: Int64?
    public let tone: Tone
    public let pace: Pace
    public let hidden: Bool

    enum CodingKeys: String, CodingKey {
        case id, label, tone, pace, hidden
        case usedPercent = "used_percent"
        case remainingPercent = "remaining_percent"
        case resetsAt = "resets_at"
        case periodSeconds = "period_seconds"
    }
}

public struct Pace: Decodable, Sendable, Hashable {
    public let severity: PaceSeverity
    public let evenPacePercent: Double?
    public let projectedPercent: Double?
    public let sparePercent: Double?
    public let runsOutAt: Timestamp?

    enum CodingKeys: String, CodingKey {
        case severity
        case evenPacePercent = "even_pace_percent"
        case projectedPercent = "projected_percent"
        case sparePercent = "spare_percent"
        case runsOutAt = "runs_out_at"
    }
}

public struct Balance: Decodable, Sendable, Hashable, Identifiable {
    public let id: String
    public let label: String
    public let kind: BalanceKind
    public let usdMicros: Int64?
    public let value: Int64?
    public let unit: String?

    enum CodingKeys: String, CodingKey {
        case id, label, kind, value, unit
        case usdMicros = "usd_micros"
    }
}

public struct Notice: Decodable, Sendable, Hashable {
    public let tone: Tone
    public let text: String
}
