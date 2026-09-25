import Foundation

public enum AccountCommand: Sendable, Hashable {
    case add(provider: String, label: String, apiKeyOnStdin: Bool)
    case login(accountID: String, apiKeyOnStdin: Bool)
    case remove(accountID: String)

    public var arguments: [String] {
        switch self {
        case .add(let provider, let label, let apiKeyOnStdin):
            let trimmed = label.trimmingCharacters(in: .whitespacesAndNewlines)
            let keyFlag = apiKeyOnStdin ? ["--api-key-stdin"] : []
            let labelFlag = trimmed.isEmpty ? [] : ["--label=\(trimmed)"]
            return ["accounts", "add", provider] + keyFlag + labelFlag + Self.progress
        case .login(let accountID, let apiKeyOnStdin):
            return ["accounts", "login", accountID] + (apiKeyOnStdin ? ["--api-key-stdin"] : []) + Self.progress
        case .remove(let accountID):
            return ["accounts", "remove", accountID, "--yes"] + Self.progress
        }
    }

    private static let progress = ["--progress", "json"]
}

public enum AccountProgressEvent: Sendable, Hashable {
    case started
    case url(String)
    case output(String)
    case done(accountID: String?)
    case error(String)

    public static func parse(_ line: String) -> AccountProgressEvent? {
        guard !line.trimmingCharacters(in: .whitespaces).isEmpty else { return nil }
        guard let raw = try? JSONDecoder().decode(RawProgressEvent.self, from: Data(line.utf8)) else {
            return .output(line)
        }
        return raw.event ?? .output(line)
    }
}

private struct RawProgressEvent: Decodable {
    let kind: String
    let url: String?
    let line: String?
    let accountID: String?
    let message: String?

    enum CodingKeys: String, CodingKey {
        case url, line, message
        case kind = "event"
        case accountID = "account_id"
    }

    var event: AccountProgressEvent? {
        switch kind {
        case "started": .started
        case "url": url.flatMap { $0.isEmpty ? nil : AccountProgressEvent.url($0) }
        case "output": line.map(AccountProgressEvent.output)
        case "done": .done(accountID: accountID.flatMap { $0.isEmpty ? nil : $0 })
        case "error": .error(message.flatMap { $0.isEmpty ? nil : $0 } ?? "unknown error")
        default: nil
        }
    }
}

public enum DeviceCode {
    public static func find(in line: String) -> String? {
        guard line.range(of: "code", options: .caseInsensitive) != nil else { return nil }
        let words = line.split(whereSeparator: { !($0.isLetter || $0.isNumber || $0 == "-") })
        return words.map(String.init).first(where: isCode)
    }

    static func isCode(_ word: String) -> Bool {
        let groups = word.split(separator: "-", omittingEmptySubsequences: false)
        guard groups.count == 2 else { return false }
        return groups.allSatisfy { group in
            group.count >= 4 && group.count <= 8
                && group.allSatisfy { $0.isASCII && ($0.isUppercase || $0.isNumber) }
        }
    }
}

public enum HelperFailure: Error, Sendable, Hashable {
    case message(String)
    case exitStatus(Int32)
    case missingHelper
    case launchFailed(String)

    public func text(_ strings: UIStrings) -> String {
        switch self {
        case .message(let message), .launchFailed(let message): message
        case .exitStatus(let status): strings.fill(AddAccountText.exitStatus, ["status": "\(status)"])
        case .missingHelper: strings.text(AddAccountText.missingHelper)
        }
    }
}
