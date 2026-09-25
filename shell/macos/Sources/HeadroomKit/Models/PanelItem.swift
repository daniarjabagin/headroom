public struct PanelItem: Decodable, Sendable, Hashable, Identifiable {
    public let accountID: String
    public let provider: String
    public let providerName: String
    public let accountLabel: String?
    public let window: String
    public let windowLabel: String
    public let usedPercent: Double
    public let remainingPercent: Double
    public let valuePercent: Double
    public let evenPacePercent: Double?
    public let tone: Tone
    public let combined: Bool
    public let accountCount: Int
    public let logo: String

    public var id: String { "\(combined ? "combined" : "account")/\(accountID)/\(window)" }

    enum CodingKeys: String, CodingKey {
        case provider, window, tone, combined, logo
        case accountID = "account_id"
        case providerName = "provider_name"
        case accountLabel = "account_label"
        case windowLabel = "window_label"
        case usedPercent = "used_percent"
        case remainingPercent = "remaining_percent"
        case valuePercent = "value_percent"
        case evenPacePercent = "even_pace_percent"
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
        valuePercent = try container.decode(Double.self, forKey: .valuePercent)
        evenPacePercent = try container.decodeIfPresent(Double.self, forKey: .evenPacePercent)
        tone = try container.decode(Tone.self, forKey: .tone)
        combined = try container.value(.combined, default: false)
        accountCount = try container.value(.accountCount, default: 1)
        logo = try container.value(.logo, default: provider)
    }

    static func fallback(headline: Headline?, valueMode: ValueMode) -> [PanelItem] {
        headline.map { [PanelItem(headline: $0, valueMode: valueMode)] } ?? []
    }

    init(headline: Headline, valueMode: ValueMode) {
        accountID = headline.accountID
        provider = headline.provider
        providerName = headline.providerName
        accountLabel = headline.accountLabel
        window = headline.window
        windowLabel = headline.windowLabel
        usedPercent = headline.usedPercent
        remainingPercent = headline.remainingPercent
        valuePercent = DisplayFormatter.readingPercent(headline, mode: valueMode)
        evenPacePercent = nil
        tone = headline.tone
        combined = headline.combined
        accountCount = headline.accountCount ?? 1
        logo = headline.provider
    }
}
