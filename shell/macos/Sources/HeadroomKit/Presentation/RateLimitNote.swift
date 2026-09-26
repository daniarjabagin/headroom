public enum RateLimitNote {
    static let noticeID = "rate_limit"

    public static func text(_ account: Account, formatter: DisplayFormatter) -> String? {
        guard AccountStatusRules.isQuietlyLimited(account) else { return nil }
        guard let next = account.refresh?.nextAt else { return formatter.strings.text(.providerLimiting) }
        return formatter.strings.fill(.providerLimitingNextTry, ["time": formatter.clockTime(next.date)])
    }

    static func notices(_ account: Account, formatter: DisplayFormatter) -> [NoticeModel] {
        guard let text = text(account, formatter: formatter) else { return [] }
        return [NoticeModel(id: noticeID, kind: .info, title: text, detail: nil, note: nil, recovery: nil)]
    }
}
