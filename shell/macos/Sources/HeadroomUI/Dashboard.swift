#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct Dashboard: View {
        let state: DaemonState
        let accounts: [Account]
        let formatter: DisplayFormatter
        let actions: PopupActions
        @Bindable var ui: PopupUIState
        let sectionActions: SectionActions
        let refresh: @MainActor () -> Void
        @Environment(\.headroomReducedMotion) private var reducedMotion
        @Environment(\.popupLayout) private var layout

        private var context: PopupContext {
            PopupContext(
                formatter: formatter, display: state.display, actions: actions, retries: ui.retries,
                sections: sectionActions, accounts: state.accounts)
        }

        var body: some View {
            VStack(alignment: .leading, spacing: layout.cg.sectionGap) {
                if SpendCardModel.shows(state) {
                    SpendSection(
                        spend: state.spend, formatter: formatter, units: SpendCardModel.units(state.features),
                        selection: selection
                    ) { refreshButton }
                } else {
                    HStack {
                        Spacer(minLength: 0)
                        refreshButton
                    }
                    .padding(.trailing, PopupMetrics.headerTrailing)
                }
                TimelineView(.periodic(from: .now, by: tickInterval)) { timeline in
                    accountLists(now: Timestamp(date: timeline.date))
                }
            }
        }

        private var selection: Binding<SpendSelection> {
            Binding(
                get: { ui.selection ?? SpendSelection.seed(display: state.display, features: state.features) },
                set: { select($0) })
        }

        private func select(_ new: SpendSelection) {
            let old = selection.wrappedValue
            Motion.perform(Motion.standard, reduced: reducedMotion) { ui.selection = new }
            guard state.features.release06 else { return }
            for change in SpendSelection.change(from: old, to: new) { actions.updateSettings(change) }
        }

        @ViewBuilder
        private func accountLists(now: Timestamp) -> some View {
            let fold = AccountFold.make(sections, state: state, strings: formatter.strings)
            VStack(alignment: .leading, spacing: layout.cg.sectionGap) {
                if !fold.pinned.isEmpty { list(fold.pinned, now: now) }
                if let summary = fold.summary {
                    if ui.showFolded {
                        NotPinnedDivider(strings: formatter.strings, collapse: { toggleFold() })
                        list(fold.folded, now: now)
                            .transition(.opacity.combined(with: .offset(y: 4)))
                    } else {
                        MoreRow(summary: summary, expand: { toggleFold() })
                            .transition(.opacity)
                    }
                }
            }
        }

        private func list(_ sections: [AccountSectionModel], now: Timestamp) -> some View {
            AccountList(
                sections: sections, context: context, now: now, expanded: ui.expanded,
                toggleExpanded: { toggle($0) }, reorder: { reorder($0) })
        }

        private var refreshButton: some View {
            RefreshButton(control: ui.refresh, strings: formatter.strings, press: refresh)
        }

        private var sections: [AccountSectionModel] {
            AccountSectionModel.sections(state, ordered: accounts, formatter: formatter)
        }

        private var tickInterval: TimeInterval {
            let now = Timestamp(date: Date())
            let live = accounts.contains { account in
                !account.hidden
                    && account.windows.contains { QuotaRowModel.needsSecondTicks($0, display: state.display, now: now) }
            }
            return live ? 1 : 30
        }

        private func toggleFold() {
            Motion.perform(Motion.standard, reduced: reducedMotion) { ui.showFolded.toggle() }
        }

        private func toggle(_ accountID: String) {
            Motion.perform(Motion.standard, reduced: reducedMotion) {
                if ui.expanded.contains(accountID) {
                    ui.expanded.remove(accountID)
                } else {
                    ui.expanded.insert(accountID)
                }
            }
        }

        private func reorder(_ visibleOrder: [String]) {
            let order = AccountOrder.mergeOrder(all: accounts.map(\.id), visible: visibleOrder)
            Motion.perform(Motion.standard, reduced: reducedMotion) { actions.setAccountOrder(order) }
        }
    }
#endif
