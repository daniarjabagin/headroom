public enum AccountRecovery: Decodable, Sendable, Hashable {
    case retry
    case signIn(accountID: String)
    case cliLogin(command: String)

    enum CodingKeys: String, CodingKey {
        case action, command
        case accountID = "account_id"
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(String.self, forKey: .action) {
        case "sign_in": self = .signIn(accountID: try container.decode(String.self, forKey: .accountID))
        case "cli_login": self = .cliLogin(command: try container.decode(String.self, forKey: .command))
        default: self = .retry
        }
    }
}
