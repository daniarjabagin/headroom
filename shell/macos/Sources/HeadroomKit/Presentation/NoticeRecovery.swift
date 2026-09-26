public enum RecoveryPrimary: Sendable, Hashable {
    case signIn(provider: String)
    case cliSignIn(provider: String, command: String)
    case copyCommand(String)
}

public struct NoticeRecovery: Sendable, Hashable {
    public let accountID: String
    public let primary: RecoveryPrimary?
    public let retrying: Bool
}

enum RecoveryRules {
    static func effective(_ account: Account) -> AccountRecovery? {
        if account.reportsRecovery { return account.recovery }
        if AccountStatusRules.isSignedOut(account) { return .signIn(accountID: account.id) }
        if AccountStatusRules.lacksSubscription(account) || account.error == nil { return nil }
        return .retry
    }

    static func notice(_ account: Account) -> NoticeRecovery? {
        effective(account).map { recovery in
            NoticeRecovery(
                accountID: account.id, primary: primary(recovery, provider: account.provider),
                retrying: account.status == .refreshing)
        }
    }

    static func retryOnly(_ account: Account) -> NoticeRecovery {
        NoticeRecovery(accountID: account.id, primary: nil, retrying: account.status == .refreshing)
    }

    static func primary(_ recovery: AccountRecovery, provider: String) -> RecoveryPrimary? {
        switch recovery {
        case .retry: nil
        case .signIn: .signIn(provider: provider)
        case .cliLogin(let command, nil): .copyCommand(command)
        case .cliLogin(let command, .some): .cliSignIn(provider: provider, command: command)
        }
    }

    static func terminalHint(_ account: Account, strings: UIStrings) -> String? {
        guard case .cliLogin(let command, _) = effective(account) else { return nil }
        return strings.fill(.runInTerminal, ["command": command])
    }

    static func signedOutDetail(_ account: Account, strings: UIStrings) -> String {
        switch effective(account) {
        case .cliLogin(let command, _): strings.fill(.runInTerminal, ["command": command])
        case .retry: strings.text(.signInThenRetry)
        case .signIn, nil: strings.text(.signedOutDetail)
        }
    }
}
