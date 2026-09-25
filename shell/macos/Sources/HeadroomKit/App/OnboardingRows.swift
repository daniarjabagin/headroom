public enum OnboardingRowKind: Sendable, Hashable {
    case tracked(accountID: String, shown: Bool)
    case signIn(provider: String)
    case notInstalled
}

public struct OnboardingRow: Sendable, Hashable, Identifiable {
    public let id: String
    public let providerID: String
    public let title: String
    public let subtitle: String
    public let kind: OnboardingRowKind
}

public enum OnboardingRows {
    public static func rows(accounts: [Account], providers: [ProviderInfo], strings: UIStrings) -> [OnboardingRow] {
        let found = accounts.map { row(for: $0, among: accounts, strings: strings) }
        let tracked = found.filter { if case .tracked = $0.kind { true } else { false } }
        let signIn = found.filter { if case .signIn = $0.kind { true } else { false } }
        return tracked + signIn + missing(providers, accounts: accounts, strings: strings)
    }

    static func row(for account: Account, among accounts: [Account], strings: UIStrings) -> OnboardingRow {
        let title = SettingsOptions.accountTitle(account, among: accounts)
        guard !AccountStatusRules.isSignedOut(account) else {
            return OnboardingRow(
                id: account.id, providerID: account.provider, title: title,
                subtitle: strings.text(OnboardingText.foundNotSignedIn), kind: .signIn(provider: account.provider))
        }
        let details = [account.plan, account.email].compactMap { $0 }
        return OnboardingRow(
            id: account.id, providerID: account.provider, title: title,
            subtitle: ([strings.text(OnboardingText.signedIn)] + details).joined(separator: " · "),
            kind: .tracked(accountID: account.id, shown: !account.hidden))
    }

    static func missing(_ providers: [ProviderInfo], accounts: [Account], strings: UIStrings) -> [OnboardingRow] {
        let known = Set(accounts.map(\.provider))
        return providers.filter { !known.contains($0.id) && isInstallable($0) }.map { provider in
            OnboardingRow(
                id: "provider:\(provider.id)", providerID: provider.id, title: provider.displayName,
                subtitle: strings.text(OnboardingText.notInstalled), kind: .notInstalled)
        }
    }

    static func isInstallable(_ provider: ProviderInfo) -> Bool {
        provider.supportedMethods.contains { method in
            if case .apiKey = method { return false }
            return true
        }
    }
}
