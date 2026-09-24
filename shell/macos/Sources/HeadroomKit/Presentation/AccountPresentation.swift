public enum HeaderStatus: Sendable, Hashable {
    case refreshing
    case outdated(updatedAt: Timestamp?)
    case failed(message: String)
}

public struct AccountHeaderModel: Sendable, Hashable {
    public let provider: String
    public let title: String
    public let plan: String?
    public let status: HeaderStatus?
}

public enum NoticeKind: Sendable, Hashable {
    case info, warning, error, signIn
}

public struct NoticeModel: Sendable, Hashable, Identifiable {
    public let id: String
    public let kind: NoticeKind
    public let title: String
    public let detail: String?
    public let retryAccountID: String?
}

public struct BlockingNotice: Sendable, Hashable {
    public let kind: NoticeKind
    public let title: String
    public let detail: String
    public let note: String?
    public let offersSignIn: Bool
    public let retrying: Bool
}

public struct AccountLimits: Sendable, Hashable {
    public let notices: [NoticeModel]
    public let skeletonRows: Int?
    public let windows: [QuotaWindow]
    public let trend: [TrendBar]?
    public let extras: [ValueRowModel]
    public let extrasCollapsible: Bool

    public var isEmpty: Bool {
        notices.isEmpty && skeletonRows == nil && windows.isEmpty && trend == nil && extras.isEmpty
    }
}

public enum AccountBody: Sendable, Hashable {
    case blocked(BlockingNotice)
    case limits(AccountLimits)
}

public struct AccountSectionModel: Sendable, Hashable, Identifiable {
    static let minimumSkeletonRows = 2

    public let id: String
    public let provider: String
    public let header: AccountHeaderModel
    public let body: AccountBody

    public static func sections(_ state: DaemonState, formatter: DisplayFormatter) -> [AccountSectionModel] {
        let visible = state.accounts.filter { !$0.hidden }
        return visible.map { account in
            make(account, siblings: visible, state: state, formatter: formatter)
        }
    }

    static func make(
        _ account: Account, siblings: [Account], state: DaemonState, formatter: DisplayFormatter
    ) -> AccountSectionModel {
        let showsName = siblings.filter { $0.provider == account.provider }.count > 1
        let body: AccountBody =
            AccountStatusRules.isBlocked(account)
            ? .blocked(blockingNotice(account, strings: formatter.strings))
            : .limits(limits(account, state: state, formatter: formatter))
        return AccountSectionModel(
            id: account.id, provider: account.provider,
            header: header(account, showsName: showsName, offline: state.offline), body: body)
    }

    static func header(_ account: Account, showsName: Bool, offline: Bool) -> AccountHeaderModel {
        let who = account.label ?? account.email
        let title = showsName ? who.map { "\(account.providerName): \($0)" } ?? account.providerName : account.providerName
        let plan = AccountStatusRules.lacksSubscription(account) ? nil : account.plan
        return AccountHeaderModel(
            provider: account.provider, title: title, plan: plan, status: status(account, offline: offline))
    }

    static func status(_ account: Account, offline: Bool) -> HeaderStatus? {
        switch account.status {
        case .refreshing: return .refreshing
        case .stale: return .outdated(updatedAt: account.updatedAt)
        case .error:
            if AccountStatusRules.failedOffline(account, offline: offline) {
                return .outdated(updatedAt: account.updatedAt)
            }
            return .failed(message: account.error?.message ?? "")
        default: return nil
        }
    }

    static func blockingNotice(_ account: Account, strings: UIStrings) -> BlockingNotice {
        let retrying = account.status == .refreshing
        if AccountStatusRules.isSignedOut(account) {
            return BlockingNotice(
                kind: .signIn, title: strings.fill(.signedOutOf, ["provider": account.providerName]),
                detail: strings.text(.signedOutDetail), note: account.error?.message, offersSignIn: true,
                retrying: retrying)
        }
        return BlockingNotice(
            kind: .warning, title: strings.text(.noSubscription), detail: strings.text(.noSubscriptionDetail),
            note: AccountStatusRules.subscriptionNote(account.error), offersSignIn: false, retrying: retrying)
    }

    static func limits(_ account: Account, state: DaemonState, formatter: DisplayFormatter) -> AccountLimits {
        let notices = notices(account, offline: state.offline, strings: formatter.strings)
        if AccountStatusRules.awaitingFirstData(account) {
            return AccountLimits(
                notices: notices, skeletonRows: max(minimumSkeletonRows, account.windows.count), windows: [],
                trend: nil, extras: [], extrasCollapsible: false)
        }
        let windows = AccountStatusRules.shownWindows(account)
        let usage = UsageRows.usage(for: account, in: state.usage)
        let trend = usage.flatMap { state.display.showTrend ? UsageRows.trendBars($0.daily, formatter: formatter) : nil }
        let spend = usage.flatMap { usage in
            state.display.showAccountSpend
                ? UsageRows.spendRows(usage, providerName: account.providerName, formatter: formatter) : nil
        }
        let extras = (spend ?? []) + UsageRows.balanceRows(account.balances, formatter: formatter)
        return AccountLimits(
            notices: notices, skeletonRows: nil, windows: windows, trend: trend, extras: extras,
            extrasCollapsible: !extras.isEmpty && (!windows.isEmpty || trend != nil))
    }

    static func notices(_ account: Account, offline: Bool, strings: UIStrings) -> [NoticeModel] {
        let daemonNotices = account.notices.enumerated().map { index, notice in
            NoticeModel(
                id: "notice:\(index)", kind: kind(of: notice.tone),
                title: NoticeTranslation.translate(notice.text, language: strings.language), detail: nil,
                retryAccountID: nil)
        }
        guard AccountStatusRules.showsErrorNotice(account, offline: offline) else { return daemonNotices }
        let error = NoticeModel(
            id: "error", kind: .error, title: strings.fill(.couldNotRefresh, ["provider": account.providerName]),
            detail: account.error?.message, retryAccountID: account.id)
        return [error] + daemonNotices
    }

    static func kind(of tone: Tone) -> NoticeKind {
        switch tone {
        case .critical: .error
        case .warning: .warning
        case .good, .neutral: .info
        }
    }
}
