public struct CombinedLimits: Sendable, Hashable {
    public let notices: [NoticeModel]
    public let windows: [CombinedWindow]
    public let members: [Account]

    public var isEmpty: Bool { notices.isEmpty && windows.isEmpty }
}

extension AccountSectionModel {
    static func group(of account: Account, in groups: [CombinedGroup]) -> CombinedGroup? {
        groups.first { $0.provider == account.provider && $0.accountIDs.contains(account.id) }
    }

    static func combined(
        _ group: CombinedGroup, members: [Account], state: DaemonState, formatter: DisplayFormatter
    ) -> AccountSectionModel {
        let strings = formatter.strings
        let header = AccountHeaderModel(
            provider: group.provider, title: group.providerName, plan: plans(group),
            status: groupStatus(members, offline: state.offline),
            accountCount: strings.fill(.accounts, count: UInt64(group.accountIDs.count)))
        let limits = CombinedLimits(
            notices: memberNotices(members, offline: state.offline, strings: strings), windows: group.windows,
            members: members)
        return AccountSectionModel(
            id: "combined:\(group.provider)", provider: group.provider, memberIDs: members.map(\.id), header: header,
            body: .combined(limits))
    }

    static func plans(_ group: CombinedGroup) -> String? {
        var seen: Set<String> = []
        let plans = group.accounts.compactMap(\.plan).filter { seen.insert($0).inserted }
        return plans.isEmpty ? nil : plans.joined(separator: " · ")
    }

    static func groupStatus(_ members: [Account], offline: Bool) -> HeaderStatus? {
        members.compactMap { status($0, offline: offline) }.min { rank($0) < rank($1) }
    }

    static func memberNotices(_ members: [Account], offline: Bool, strings: UIStrings) -> [NoticeModel] {
        var seen: Set<String> = []
        let collected = members.flatMap { member in
            Self.notices(member, offline: offline, strings: strings, name: title(member, showsName: true)).map {
                NoticeModel(
                    id: "\(member.id):\($0.id)", kind: $0.kind, title: $0.title, detail: $0.detail,
                    retryAccountID: $0.retryAccountID)
            }
        }
        return collected.filter { seen.insert($0.title).inserted }
    }

    private static func rank(_ status: HeaderStatus) -> Int {
        switch status {
        case .refreshing: 0
        case .failed: 1
        case .outdated: 2
        }
    }
}
