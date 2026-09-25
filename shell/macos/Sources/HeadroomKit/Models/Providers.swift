import Foundation

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
    public let links: ProviderLinks?

    enum CodingKeys: String, CodingKey {
        case id, links
        case displayName = "display_name"
        case addAccount = "add_account"
        case multiAccount = "multi_account"
        case localUsage = "local_usage"
    }
}

public enum ProviderLinkKind: String, Sendable, Hashable, CaseIterable {
    case status, dashboard, usage
}

public struct ProviderLinks: Decodable, Sendable, Hashable {
    public let status: URL?
    public let dashboard: URL?
    public let usage: URL?

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: ProviderLinkKind.self)
        status = try Self.secureURL(container, .status)
        dashboard = try Self.secureURL(container, .dashboard)
        usage = try Self.secureURL(container, .usage)
    }

    public func url(_ kind: ProviderLinkKind) -> URL? {
        switch kind {
        case .status: status
        case .dashboard: dashboard
        case .usage: usage
        }
    }

    typealias Container = KeyedDecodingContainer<ProviderLinkKind>

    static func secureURL(_ container: Container, _ kind: ProviderLinkKind) throws -> URL? {
        guard let text = try container.decodeIfPresent(String.self, forKey: kind), text.hasPrefix("https://"),
            let url = URL(string: text), url.host?.isEmpty == false
        else { return nil }
        return url
    }
}

extension ProviderLinkKind: CodingKey {}

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
