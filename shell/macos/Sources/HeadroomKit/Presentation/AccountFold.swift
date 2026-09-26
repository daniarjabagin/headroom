public struct FoldSummary: Sendable, Hashable {
    public static let maxGlyphs = 3

    public let title: String
    public let names: String
    public let providers: [String]
    public let attention: FoldAttention
    public let attentionText: String?
    public let accessibilityLabel: String
}

public struct AccountFold: Sendable, Hashable {
    public let pinned: [AccountSectionModel]
    public let folded: [AccountSectionModel]
    public let summary: FoldSummary?

    public var isFolding: Bool { !folded.isEmpty }

    public static func make(
        _ sections: [AccountSectionModel], state: DaemonState, strings: UIStrings
    ) -> AccountFold {
        let folded = sections.filter { isCollapsed($0, state: state) }
        guard !folded.isEmpty else { return AccountFold(pinned: sections, folded: [], summary: nil) }
        let pinned = sections.filter { !isCollapsed($0, state: state) }
        let attention = FoldAttention.make(folded, state: state)
        return AccountFold(
            pinned: pinned, folded: folded, summary: summary(folded, attention: attention, strings: strings))
    }

    static func isCollapsed(_ section: AccountSectionModel, state: DaemonState) -> Bool {
        if case .combined = section.body {
            return state.combined.first { $0.provider == section.provider }?.collapsed ?? false
        }
        let members = state.accounts.filter { section.memberIDs.contains($0.id) }
        return !members.isEmpty && members.allSatisfy(\.collapsed)
    }

    static func summary(
        _ folded: [AccountSectionModel], attention: FoldAttention, strings: UIStrings
    ) -> FoldSummary {
        var seen: Set<String> = []
        let providers = folded.map(\.provider).filter { seen.insert($0).inserted }
        let title = strings.fill(PopupExtraText.moreCount, ["count": "\(folded.count)"])
        let names = "· " + folded.map(\.header.title).joined(separator: ", ")
        let note = attention.text(strings)
        return FoldSummary(
            title: title, names: names, providers: Array(providers.prefix(FoldSummary.maxGlyphs)),
            attention: attention, attentionText: note,
            accessibilityLabel: [title + " " + names, note].compactMap { $0 }.joined(separator: ". "))
    }
}
