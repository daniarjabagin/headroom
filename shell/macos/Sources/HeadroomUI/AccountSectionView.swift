#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct PopupContext {
        let formatter: DisplayFormatter
        let display: DisplaySettings
        let actions: PopupActions

        var strings: UIStrings { formatter.strings }

        @MainActor
        func toggleValueMode() {
            actions.updateSettings(DisplayPatch.toggledValueMode(display))
        }

        @MainActor
        func toggleResetFormat() {
            actions.updateSettings(DisplayPatch.toggledResetFormat(display))
        }
    }

    struct AccountSectionView: View {
        let section: AccountSectionModel
        let context: PopupContext
        let now: Timestamp
        let expanded: Bool
        let reorder: ReorderHandle?
        let toggleExpanded: @MainActor () -> Void

        var body: some View {
            VStack(alignment: .leading, spacing: PopupMetrics.headerGap) {
                AccountHeaderView(header: section.header, context: context, now: now, reorder: reorder)
                if !isEmpty {
                    VStack(alignment: .leading, spacing: 0) { card }
                        .padding(.vertical, PopupMetrics.cardGutter)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .cardSurface()
                }
            }
        }

        private var isEmpty: Bool {
            if case .limits(let limits) = section.body { return limits.isEmpty }
            return false
        }

        @ViewBuilder
        private var card: some View {
            switch section.body {
            case .blocked(let notice):
                BlockingNoticeView(notice: notice, section: section, context: context)
            case .limits(let limits):
                LimitsView(
                    accountID: section.id, limits: limits, context: context, now: now, expanded: expanded,
                    toggleExpanded: toggleExpanded)
            }
        }
    }

    struct BlockingNoticeView: View {
        let notice: BlockingNotice
        let section: AccountSectionModel
        let context: PopupContext

        var body: some View {
            NoticePlate(kind: notice.kind, title: notice.title, detail: notice.detail, note: notice.note, actions: actions)
        }

        private var actions: [NoticeAction] {
            let strings = context.strings
            let actions = context.actions
            let provider = section.provider
            let accountID = section.id
            let retry = NoticeAction(
                id: "retry", title: strings.text(notice.retrying ? .retrying : .retry), primary: false,
                busy: notice.retrying, run: { actions.refreshAccount(accountID) })
            guard notice.offersSignIn else { return [retry] }
            let signIn = NoticeAction(
                id: "signin", title: strings.text(.signIn), primary: true, busy: false,
                run: { actions.signIn(provider) })
            return [signIn, retry]
        }
    }

    struct LimitsView: View {
        let accountID: String
        let limits: AccountLimits
        let context: PopupContext
        let now: Timestamp
        let expanded: Bool
        let toggleExpanded: @MainActor () -> Void

        var body: some View {
            ForEach(limits.notices) { notice in noticeView(notice) }
            if let rows = limits.skeletonRows {
                SkeletonRows(count: rows)
            }
            ForEach(limits.windows) { window in
                QuotaRowView(
                    row: QuotaRowModel.make(window, display: context.display, now: now, formatter: context.formatter),
                    toggleValueMode: { context.toggleValueMode() },
                    toggleResetFormat: { context.toggleResetFormat() })
            }
            if let bars = limits.trend {
                TrendRow(accountID: accountID, bars: bars, title: context.strings.text(.usageTrend))
            }
            extras
        }

        @ViewBuilder
        private var extras: some View {
            if limits.extrasCollapsible {
                Expander(expanded: expanded, toggle: toggleExpanded)
                if expanded {
                    extraRows.transition(.opacity.combined(with: .move(edge: .top)))
                }
            } else {
                extraRows
            }
        }

        private var extraRows: some View {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(limits.extras) { row in ValueRow(accountID: accountID, row: row) }
            }
        }

        @ViewBuilder
        private func noticeView(_ notice: NoticeModel) -> some View {
            if notice.kind == .info {
                NoticeLine(text: notice.title)
            } else {
                NoticePlate(
                    kind: notice.kind, title: notice.title, detail: notice.detail, note: nil,
                    actions: retryActions(notice))
            }
        }

        private func retryActions(_ notice: NoticeModel) -> [NoticeAction] {
            guard let accountID = notice.retryAccountID else { return [] }
            let actions = context.actions
            return [
                NoticeAction(
                    id: "retry", title: context.strings.text(.retry), primary: false, busy: false,
                    run: { actions.refreshAccount(accountID) })
            ]
        }
    }
#endif
