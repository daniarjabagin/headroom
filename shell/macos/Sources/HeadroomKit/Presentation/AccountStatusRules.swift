public enum AccountStatusRules {
    static let signInErrors: Set<String> = ["not_signed_in", "sign_in_expired"]
    static let noSubscriptionKind = "no_subscription"
    static let networkKind = "network"
    static let noSubscriptionTitle = "no active subscription"

    public static func isSignedOut(_ account: Account) -> Bool {
        if account.status == .signedOut { return true }
        return account.status == .refreshing && signInErrors.contains(account.error?.kind ?? "")
    }

    public static func lacksSubscription(_ account: Account) -> Bool {
        if account.status == .noSubscription { return true }
        return account.status == .refreshing && account.error?.kind == noSubscriptionKind
    }

    public static func isBlocked(_ account: Account) -> Bool {
        isSignedOut(account) || lacksSubscription(account)
    }

    public static func failedOffline(_ account: Account, offline: Bool) -> Bool {
        offline && account.error?.kind == networkKind
    }

    public static func showsErrorNotice(_ account: Account, offline: Bool) -> Bool {
        account.status == .error && !failedOffline(account, offline: offline)
    }

    public static func awaitingFirstData(_ account: Account) -> Bool {
        account.status == .refreshing && account.updatedAt == nil && shownWindows(account).isEmpty
    }

    public static func shownWindows(_ account: Account) -> [QuotaWindow] {
        account.windows.filter { !$0.hidden }
    }

    public static func subscriptionNote(_ error: AccountError?) -> String? {
        guard let error, error.message != error.kind else { return nil }
        let text = normalized(error.message)
        guard !text.isEmpty, text != noSubscriptionTitle, text != normalized(noSubscriptionKind) else { return nil }
        return error.message
    }

    static func normalized(_ text: String) -> String {
        var result = ""
        var pendingSpace = false
        for character in text.lowercased() {
            if character.isLetter || character.isNumber {
                if pendingSpace && !result.isEmpty { result.append(" ") }
                pendingSpace = false
                result.append(character)
            } else {
                pendingSpace = true
            }
        }
        return result
    }
}
