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
    public let panelItems: [PanelItem]
    public let panelTone: Tone?
    public let providerStatus: [ProviderStatus]
    let reportsPanelItems: Bool

    public var features: DaemonFeatures { DaemonFeatures(state: self) }

    enum CodingKeys: String, CodingKey {
        case version, offline, display, headline, accounts, usage, spend, combined
        case appVersion = "app_version"
        case panelItems = "panel_items"
        case panelTone = "panel_tone"
        case providerStatus = "provider_status"
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
        combined = try container.value(.combined, default: [])
        let reported = try container.decodeIfPresent([PanelItem].self, forKey: .panelItems)
        reportsPanelItems = reported != nil
        panelItems = reported ?? PanelItem.fallback(headline: headline, valueMode: display.valueMode)
        panelTone = try container.decodeIfPresent(Tone.self, forKey: .panelTone)
        providerStatus = try container.value(.providerStatus, default: [])
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
    public let recovery: AccountRecovery?
    let reportsRecovery: Bool
    public let updatedAt: Timestamp?
    public let source: DataSource?
    public let windows: [QuotaWindow]
    public let balances: [Balance]
    public let notices: [Notice]
    public let usageHome: String
    public let collapsed: Bool
    public let refresh: AccountRefresh?

    public var displayName: String { label ?? email ?? providerName }

    enum CodingKeys: String, CodingKey {
        case id, provider, label, email, plan, hidden, owner, status, error, recovery, source, windows,
            balances, notices, collapsed, refresh
        case providerName = "provider_name"
        case updatedAt = "updated_at"
        case usageHome = "usage_home"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        provider = try container.decode(String.self, forKey: .provider)
        providerName = try container.decode(String.self, forKey: .providerName)
        label = try container.decodeIfPresent(String.self, forKey: .label)
        email = try container.decodeIfPresent(String.self, forKey: .email)
        plan = try container.decodeIfPresent(String.self, forKey: .plan)
        hidden = try container.decode(Bool.self, forKey: .hidden)
        owner = try container.decode(AccountOwner.self, forKey: .owner)
        status = try container.decode(AccountStatus.self, forKey: .status)
        error = try container.decodeIfPresent(AccountError.self, forKey: .error)
        recovery = try container.decodeIfPresent(AccountRecovery.self, forKey: .recovery)
        reportsRecovery = container.contains(.recovery)
        updatedAt = try container.decodeIfPresent(Timestamp.self, forKey: .updatedAt)
        source = try container.decodeIfPresent(DataSource.self, forKey: .source)
        windows = try container.decode([QuotaWindow].self, forKey: .windows)
        balances = try container.decode([Balance].self, forKey: .balances)
        notices = try container.decode([Notice].self, forKey: .notices)
        usageHome = try container.decode(String.self, forKey: .usageHome)
        collapsed = try container.value(.collapsed, default: false)
        refresh = try container.decodeIfPresent(AccountRefresh.self, forKey: .refresh)
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
    public let basis: PaceBasis?
    public let activeLeftSeconds: UInt64?

    enum CodingKeys: String, CodingKey {
        case severity, basis
        case evenPacePercent = "even_pace_percent"
        case projectedPercent = "projected_percent"
        case sparePercent = "spare_percent"
        case runsOutAt = "runs_out_at"
        case activeLeftSeconds = "active_left_seconds"
    }

    public var isPaused: Bool { basis == .paused }
}

public struct Balance: Decodable, Sendable, Hashable, Identifiable {
    public let id: String
    public let label: String
    public let kind: BalanceKind
    public let usdMicros: Int64?
    public let currency: String?
    public let micros: Int64?
    public let value: Int64?
    public let unit: String?

    enum CodingKeys: String, CodingKey {
        case id, label, kind, currency, micros, value, unit
        case usdMicros = "usd_micros"
    }
}

public struct Notice: Decodable, Sendable, Hashable {
    public let tone: Tone
    public let text: String
}
