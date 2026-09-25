#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct PopupContext {
        let formatter: DisplayFormatter
        let display: DisplaySettings
        let actions: PopupActions
        let retries: RetryControl

        var strings: UIStrings { formatter.strings }

        @MainActor
        func toggleValueMode() {
            actions.updateSettings(DisplayToggle.valueMode(display))
        }

        @MainActor
        func toggleResetFormat() {
            actions.updateSettings(DisplayToggle.resetFormat(display))
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
            switch section.body {
            case .limits(let limits): limits.isEmpty
            case .combined(let limits): limits.isEmpty
            case .blocked: false
            }
        }

        @ViewBuilder
        private var card: some View {
            switch section.body {
            case .blocked(let notice):
                RecoveryPlate(
                    kind: notice.kind, title: notice.title, detail: notice.detail, note: notice.note,
                    recovery: notice.recovery, context: context)
            case .limits(let limits):
                LimitsView(
                    accountID: section.id, limits: limits, context: context, now: now, expanded: expanded,
                    toggleExpanded: toggleExpanded)
            case .combined(let limits):
                CombinedLimitsView(sectionID: section.id, limits: limits, context: context, now: now)
            }
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
            ForEach(limits.notices) { notice in NoticeRow(notice: notice, context: context) }
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
