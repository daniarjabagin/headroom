#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import Observation
    import SwiftUI

    public struct PopupActions {
        public var refreshNow: @MainActor () async -> Bool
        public var refreshAccount: @MainActor (String) -> Void
        public var openSettings: @MainActor () -> Void
        public var signIn: @MainActor (String) -> Void
        public var setAccountOrder: @MainActor ([String]) -> Void
        public var updateSettings: @MainActor (SettingsChange) -> Void
        public var reducedMotion: @MainActor () -> Bool

        public init(
            refreshNow: @escaping @MainActor () async -> Bool,
            refreshAccount: @escaping @MainActor (String) -> Void,
            openSettings: @escaping @MainActor () -> Void,
            signIn: @escaping @MainActor (String) -> Void,
            setAccountOrder: @escaping @MainActor ([String]) -> Void,
            updateSettings: @escaping @MainActor (SettingsChange) -> Void,
            reducedMotion: @escaping @MainActor () -> Bool
        ) {
            self.refreshNow = refreshNow
            self.refreshAccount = refreshAccount
            self.openSettings = openSettings
            self.signIn = signIn
            self.setAccountOrder = setAccountOrder
            self.updateSettings = updateSettings
            self.reducedMotion = reducedMotion
        }
    }

    @MainActor
    @Observable
    final class PopupUIState {
        var period = SpendPeriod.today
        var expanded: Set<String> = []
        let tips = TipCenter()
        let refresh = RefreshControl()
        let icons = ProviderIconStore()
    }

    public struct PopupView: View {
        let model: AppModel
        let actions: PopupActions
        let presentation: Int
        let maxHeight: CGFloat?
        let onResize: (@MainActor (CGSize) -> Void)?
        @State private var ui = PopupUIState()
        @State private var contentHeight: CGFloat = 0
        @State private var footerHeight: CGFloat = 0
        @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
        @Environment(\.accessibilityReduceTransparency) private var systemReduceTransparency

        public init(
            model: AppModel, actions: PopupActions, presentation: Int = 0, maxHeight: CGFloat? = nil,
            onResize: (@MainActor (CGSize) -> Void)? = nil
        ) {
            self.model = model
            self.actions = actions
            self.presentation = presentation
            self.maxHeight = maxHeight
            self.onResize = onResize
        }

        public var body: some View {
            let screen = PopupScreen.make(
                phase: model.phase, state: model.state, lastError: model.lastError, serviceIssue: model.serviceIssue)
            let translucent = (model.state?.display.translucent ?? false) && !systemReduceTransparency
            VStack(spacing: 0) {
                scrollArea(screen)
                FooterView(
                    screen: screen, formatter: model.formatter, refresh: { pressRefresh() },
                    openSettings: { actions.openSettings() }
                )
                .background { HeightReader(height: $footerHeight) }
            }
            .frame(width: PopupMetrics.width)
            .background { SizeReader(onChange: onResize) }
            .clipShape(RoundedRectangle(cornerRadius: PopupMetrics.cornerRadius, style: .continuous))
            .popupSurface(translucent: translucent)
            .overlay { TipOverlay(center: ui.tips) }
            .coordinateSpace(.named(PopupSpace.root))
            .environment(\.headroomReducedMotion, systemReduceMotion || actions.reducedMotion())
            .environment(\.headroomTranslucent, translucent)
            .environment(ui.tips)
            .environment(ui.icons)
            .onChange(of: model.state.map(PopupScreen.isRefreshing) ?? false, initial: true) { _, busy in
                ui.refresh.setDaemonBusy(busy)
            }
        }

        private var maxScrollHeight: CGFloat {
            let screen = NSScreen.main?.visibleFrame.height ?? 800
            let panel = maxHeight ?? PanelPlacement.maxHeight(visibleHeight: screen)
            return max(PopupMetrics.minScrollHeight, panel - footerHeight)
        }

        @ViewBuilder
        private func scrollArea(_ screen: PopupScreen) -> some View {
            let content = PopupContent(
                screen: screen, accounts: model.orderedAccounts, formatter: model.formatter, actions: actions, ui: ui,
                refresh: { pressRefresh() }
            )
            .padding(.horizontal, PopupMetrics.padding)
            .padding(.top, PopupMetrics.padding)
            .padding(.bottom, PopupMetrics.bottomPadding)
            .background { HeightReader(height: $contentHeight) }
            if contentHeight > maxScrollHeight {
                ThinScrollView(height: maxScrollHeight, contentHeight: contentHeight, content: content)
                    .id(presentation)
            } else {
                content.fixedSize(horizontal: false, vertical: true)
            }
        }

        private func pressRefresh() {
            ui.refresh.press(actions.refreshNow)
        }
    }

    struct HeightReader: View {
        @Binding var height: CGFloat

        var body: some View {
            GeometryReader { proxy in
                Color.clear.onChange(of: proxy.size.height, initial: true) { _, new in height = new }
            }
        }
    }

    struct SizeReader: View {
        let onChange: (@MainActor (CGSize) -> Void)?

        var body: some View {
            GeometryReader { proxy in
                Color.clear.onChange(of: proxy.size, initial: true) { _, new in onChange?(new) }
            }
        }
    }

    struct PopupContent: View {
        let screen: PopupScreen
        let accounts: [Account]
        let formatter: DisplayFormatter
        let actions: PopupActions
        @Bindable var ui: PopupUIState
        let refresh: @MainActor () -> Void

        var body: some View {
            switch screen {
            case .dashboard(let state):
                Dashboard(
                    state: state, accounts: accounts, formatter: formatter, actions: actions, ui: ui, refresh: refresh)
            default:
                StatusScreen(screen: screen, strings: formatter.strings, retry: refresh)
            }
        }
    }

    struct Dashboard: View {
        let state: DaemonState
        let accounts: [Account]
        let formatter: DisplayFormatter
        let actions: PopupActions
        @Bindable var ui: PopupUIState
        let refresh: @MainActor () -> Void
        @Environment(\.headroomReducedMotion) private var reducedMotion

        private var context: PopupContext {
            PopupContext(formatter: formatter, display: state.display, actions: actions)
        }

        var body: some View {
            VStack(alignment: .leading, spacing: PopupMetrics.sectionGap) {
                if SpendCardModel.shows(state) {
                    SpendSection(spend: state.spend, formatter: formatter, period: $ui.period) { refreshButton }
                } else {
                    HStack {
                        Spacer(minLength: 0)
                        refreshButton
                    }
                    .padding(.trailing, PopupMetrics.headerTrailing)
                }
                TimelineView(.periodic(from: .now, by: tickInterval)) { timeline in
                    AccountList(
                        sections: sections, context: context, now: Timestamp(date: timeline.date),
                        expanded: ui.expanded, toggleExpanded: { toggle($0) }, reorder: { reorder($0) })
                }
            }
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
