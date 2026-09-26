public enum AccountRecovery: Decodable, Sendable, Hashable {
    case retry
    case signIn(accountID: String)
    case cliLogin(command: String, accountID: String?)

    enum CodingKeys: String, CodingKey {
        case action, command
        case accountID = "account_id"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(String.self, forKey: .action) {
        case "sign_in": self = .signIn(accountID: try container.decode(String.self, forKey: .accountID))
        case "cli_login":
            self = .cliLogin(
                command: try container.decode(String.self, forKey: .command),
                accountID: try container.decodeIfPresent(String.self, forKey: .accountID))
        default: self = .retry
        }
    }

    public var signInAccountID: String? {
        switch self {
        case .signIn(let accountID): accountID
        case .cliLogin(_, let accountID): accountID
        case .retry: nil
        }
    }
}
