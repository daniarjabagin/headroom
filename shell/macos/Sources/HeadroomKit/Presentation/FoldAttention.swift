public struct FoldAttention: Sendable, Hashable {
    public enum Kind: Sendable, Hashable {
        case none, tone, notice
    }

    public static let calm = FoldAttention(kind: .none, tone: .neutral, count: 0)

    public let kind: Kind
    public let tone: Tone
    public let count: Int

    public static func make(_ folded: [AccountSectionModel], state: DaemonState) -> FoldAttention {
        let entries = folded.map { entry($0, state: state) }
        let count = entries.filter { !$0.notices.isEmpty || isAlarming($0.tone) }.count
        let notices = entries.flatMap(\.notices)
        if !notices.isEmpty {
            return FoldAttention(kind: .notice, tone: notices.contains(.error) ? .critical : .warning, count: count)
        }
        let tone = worst(entries.map(\.tone))
        return isAlarming(tone) ? FoldAttention(kind: .tone, tone: tone, count: count) : calm
    }

    public func text(_ strings: UIStrings) -> String? {
        kind == .none ? nil : strings.fill(PopupPlural.needAttention, count: UInt64(count))
    }

    enum Notice: Sendable, Hashable {
        case blocked, error
    }

    struct Entry {
        let notices: [Notice]
        let tone: Tone
    }

    static func entry(_ section: AccountSectionModel, state: DaemonState) -> Entry {
        let members = state.accounts.filter { section.memberIDs.contains($0.id) }
        let notices = members.compactMap { notice($0, offline: state.offline) }
        return Entry(notices: notices, tone: worst(tones(section, members: members, state: state)))
    }

    static func tones(_ section: AccountSectionModel, members: [Account], state: DaemonState) -> [Tone] {
        if case .combined = section.body {
            let group = state.combined.first { $0.provider == section.provider }
            return group?.windows.map(\.tone) ?? []
        }
        return members.flatMap(AccountStatusRules.shownWindows).map(\.tone)
    }

    static func notice(_ account: Account, offline: Bool) -> Notice? {
        if AccountStatusRules.isBlocked(account) { return .blocked }
        return AccountStatusRules.showsErrorNotice(account, offline: offline) ? .error : nil
    }

    static func rank(_ tone: Tone) -> Int {
        switch tone {
        case .critical: 2
        case .warning: 1
        case .neutral, .good: 0
        }
    }

    static func isAlarming(_ tone: Tone) -> Bool {
        rank(tone) > 0
    }

    static func worst(_ tones: [Tone]) -> Tone {
        tones.max { rank($0) < rank($1) } ?? .neutral
    }
}
