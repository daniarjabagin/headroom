public struct ProvidersPayload: Decodable, Sendable, Hashable {
    public let version: Int
    public let providers: [ProviderInfo]
}

public struct ProviderInfo: Decodable, Sendable, Hashable, Identifiable {
    public let id: String
    public let displayName: String
    public let addAccount: [AddAccountMethod]
    public let multiAccount: Bool
    public let localUsage: Bool

    enum CodingKeys: String, CodingKey {
        case id
        case displayName = "display_name"
        case addAccount = "add_account"
        case multiAccount = "multi_account"
        case localUsage = "local_usage"
    }
}

public enum AddAccountMethod: Decodable, Sendable, Hashable {
    case cliLogin(program: String)
    case apiKey(label: String, consoleURL: String, hint: String)
    case autoDetect(reason: String)
    case unsupported(kind: String)

    enum CodingKeys: String, CodingKey {
        case kind, program, label, hint, reason
        case consoleURL = "console_url"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let kind = try container.decode(String.self, forKey: .kind)
        switch kind {
        case "cli_login":
            self = .cliLogin(program: try container.decode(String.self, forKey: .program))
        case "api_key":
            self = .apiKey(
                label: try container.decode(String.self, forKey: .label),
                consoleURL: try container.decode(String.self, forKey: .consoleURL),
                hint: try container.decode(String.self, forKey: .hint))
        case "auto_detect":
            self = .autoDetect(reason: try container.decode(String.self, forKey: .reason))
        default:
            self = .unsupported(kind: kind)
        }
    }
}
