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
    public let accountCount: String?

    init(provider: String, title: String, plan: String?, status: HeaderStatus?, accountCount: String? = nil) {
        self.provider = provider
        self.title = title
        self.plan = plan
        self.status = status
        self.accountCount = accountCount
    }
}

public enum NoticeKind: Sendable, Hashable {
    case info, warning, error, signIn
}

public struct NoticeModel: Sendable, Hashable, Identifiable {
    public let id: String
    public let kind: NoticeKind
    public let title: String
    public let detail: String?
    public let note: String?
    public let recovery: NoticeRecovery?
}

public struct BlockingNotice: Sendable, Hashable {
    public let kind: NoticeKind
    public let title: String
    public let detail: String
    public let note: String?
    public let recovery: NoticeRecovery
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
    case combined(CombinedLimits)
}

public struct AccountSectionModel: Sendable, Hashable, Identifiable {
    static let minimumSkeletonRows = 2

    public let id: String
    public let provider: String
    public let memberIDs: [String]
    public let header: AccountHeaderModel
    public let body: AccountBody

    public static func sections(
        _ state: DaemonState, ordered accounts: [Account]? = nil, formatter: DisplayFormatter
    ) -> [AccountSectionModel] {
        let visible = (accounts ?? state.accounts).filter { !$0.hidden }
        return visible.compactMap { account in
            guard let group = group(of: account, in: state.combined) else {
                return make(account, siblings: visible, state: state, formatter: formatter)
            }
            let members = visible.filter { group.accountIDs.contains($0.id) }
            guard members.first?.id == account.id else { return nil }
            return combined(group, members: members, state: state, formatter: formatter)
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
            id: account.id, provider: account.provider, memberIDs: [account.id],
            header: header(account, showsName: showsName, offline: state.offline), body: body)
    }

    static func header(_ account: Account, showsName: Bool, offline: Bool) -> AccountHeaderModel {
        let plan = AccountStatusRules.lacksSubscription(account) ? nil : account.plan
        return AccountHeaderModel(
            provider: account.provider, title: title(account, showsName: showsName), plan: plan,
            status: status(account, offline: offline))
    }

    static func title(_ account: Account, showsName: Bool) -> String {
        let who = account.label ?? account.email
        return showsName ? who.map { "\(account.providerName): \($0)" } ?? account.providerName : account.providerName
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
        let recovery = RecoveryRules.notice(account) ?? RecoveryRules.retryOnly(account)
        if AccountStatusRules.isSignedOut(account) {
            return BlockingNotice(
                kind: .signIn, title: strings.fill(.signedOutOf, ["provider": account.providerName]),
                detail: RecoveryRules.signedOutDetail(account, strings: strings), note: account.error?.message,
                recovery: recovery)
        }
        return BlockingNotice(
            kind: .warning, title: strings.text(.noSubscription), detail: strings.text(.noSubscriptionDetail),
            note: AccountStatusRules.subscriptionNote(account.error), recovery: recovery)
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
        let trend = usage.flatMap {
            state.display.showTrend ? UsageRows.trendBars($0.daily, formatter: formatter) : nil
        }
        let spend = usage.flatMap { usage in
            state.display.showAccountSpend
                ? UsageRows.spendRows(usage, providerName: account.providerName, formatter: formatter) : nil
        }
        let extras = (spend ?? []) + UsageRows.balanceRows(account.balances, formatter: formatter)
        return AccountLimits(
            notices: notices, skeletonRows: nil, windows: windows, trend: trend, extras: extras,
            extrasCollapsible: !extras.isEmpty && (!windows.isEmpty || trend != nil))
    }

    static func notices(_ account: Account, offline: Bool, strings: UIStrings, name: String? = nil) -> [NoticeModel] {
        let daemonNotices = account.notices.enumerated().map { index, notice in
            NoticeModel(
                id: "notice:\(index)", kind: kind(of: notice.tone),
                title: NoticeTranslation.translate(notice.text, language: strings.language), detail: nil, note: nil,
                recovery: nil)
        }
        guard AccountStatusRules.showsErrorNotice(account, offline: offline) else { return daemonNotices }
        return [errorNotice(account, strings: strings, name: name ?? account.providerName)] + daemonNotices
    }

    static func errorNotice(_ account: Account, strings: UIStrings, name: String) -> NoticeModel {
        let changed = AccountStatusRules.accountChanged(account)
        return NoticeModel(
            id: "error", kind: .error,
            title: strings.fill(changed ? .accountChanged : .couldNotRefresh, ["provider": name]),
            detail: changed ? strings.text(.accountChangedDetail) : account.error?.message,
            note: RecoveryRules.terminalHint(account, strings: strings), recovery: RecoveryRules.notice(account))
    }

    static func kind(of tone: Tone) -> NoticeKind {
        switch tone {
        case .critical: .error
        case .warning: .warning
        case .good, .neutral: .info
        }
    }
}
