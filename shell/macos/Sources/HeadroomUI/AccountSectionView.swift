#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct PopupContext {
        let formatter: DisplayFormatter
        let display: DisplaySettings
        let actions: PopupActions
        let retries: RetryControl
        let sections: SectionActions
        let accounts: [Account]

        var strings: UIStrings { formatter.strings }

        @MainActor
        func toggleValueMode() {
            actions.updateSettings(DisplayToggle.valueMode(display))
        }

        @MainActor
        func toggleResetFormat() {
            actions.updateSettings(DisplayToggle.resetFormat(display))
        }

        var valueTip: String { ReadingTips.value(display, strings: strings) }
        var resetTip: String { ReadingTips.reset(display, strings: strings) }

        func status(for section: AccountSectionModel, now: Timestamp) -> StatusNoticeModel? {
            StatusNoticeModel.make(
                provider: section.provider, statuses: sections.statuses, now: now, formatter: formatter)
        }

        @MainActor
        func menu(for section: AccountSectionModel) -> HeaderMenuModel {
            let name = accounts.first { section.memberIDs.contains($0.id) }?.providerName ?? section.header.title
            return HeaderMenuModel.make(
                section, providerName: name, links: sections.links(section.provider), display: display,
                features: sections.features, strings: strings)
        }
    }

    struct AccountSectionView: View {
        let section: AccountSectionModel
        let context: PopupContext
        let now: Timestamp
        let expanded: Bool
        let reorder: ReorderHandle?
        let toggleExpanded: @MainActor () -> Void
        @Environment(\.popupLayout) private var layout

        var body: some View {
            let status = context.status(for: section, now: now)
            VStack(alignment: .leading, spacing: layout.cg.headerGap) {
                AccountHeaderView(
                    header: section.header, context: context, now: now, reorder: reorder, status: status,
                    menu: context.menu(for: section), run: { kind, url in context.sections.run(kind, section, url) })
                if !isEmpty || status != nil {
                    VStack(alignment: .leading, spacing: 0) { card(status) }
                        .padding(.vertical, layout.cg.cardGutter)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .cardSurface()
                }
            }
        }

        private var isEmpty: Bool {
            switch section.body {
            case .limits(let limits): limits.isEmpty
            case .combined(let limits): limits.isEmpty
            case .blocked: false
            }
        }

        @ViewBuilder
        private func card(_ status: StatusNoticeModel?) -> some View {
            switch section.body {
            case .blocked(let notice):
                RecoveryPlate(
                    kind: notice.kind, title: notice.title, detail: notice.detail, note: notice.note,
                    recovery: notice.recovery, context: context)
                statusView(status)
            case .limits(let limits):
                LimitsView(
                    accountID: section.id, limits: limits, context: context, now: now, expanded: expanded,
                    status: status, toggleExpanded: toggleExpanded)
            case .combined(let limits):
                CombinedLimitsView(sectionID: section.id, limits: limits, context: context, now: now, status: status)
            }
        }

        @ViewBuilder
        private func statusView(_ status: StatusNoticeModel?) -> some View {
            if let status { StatusNoticeView(notice: status, open: { context.actions.openURL($0) }) }
        }
    }

    struct LimitsView: View {
        let accountID: String
        let limits: AccountLimits
        let context: PopupContext
        let now: Timestamp
        let expanded: Bool
        let status: StatusNoticeModel?
        let toggleExpanded: @MainActor () -> Void

        var body: some View {
            ForEach(limits.notices) { notice in NoticeRow(notice: notice, context: context) }
            if let status { StatusNoticeView(notice: status, open: { context.actions.openURL($0) }) }
            if let rows = limits.skeletonRows {
                SkeletonRows(count: rows)
            }
            ForEach(limits.windows) { window in
                QuotaRowView(
                    scope: accountID,
                    row: QuotaRowModel.make(window, display: context.display, now: now, formatter: context.formatter),
                    resetsAt: window.resetsAt, context: context, now: now)
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
    }

    struct NoticeRow: View {
        let notice: NoticeModel
        let context: PopupContext

        var body: some View {
            if notice.kind == .info {
                NoticeLine(text: notice.title)
            } else {
                RecoveryPlate(
                    kind: notice.kind, title: notice.title, detail: notice.detail, note: notice.note,
                    recovery: notice.recovery, context: context)
            }
        }
    }
#endif
