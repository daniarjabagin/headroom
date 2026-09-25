#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import Observation
    import SwiftUI

    public struct PopupActions {
        public var refreshNow: @MainActor () async -> Bool
        public var refreshAccount: @MainActor (String) async -> Bool
        public var openSettings: @MainActor () -> Void
        public var signIn: @MainActor (String) -> Void
        public var setAccountOrder: @MainActor ([String]) -> Void
        public var updateSettings: @MainActor (SettingsChange) -> Void
        public var reducedMotion: @MainActor () -> Bool
        public var providerLinks: @MainActor (String) -> ProviderLinks?
        public var openURL: @MainActor (URL) -> Void

        public init(
            refreshNow: @escaping @MainActor () async -> Bool,
            refreshAccount: @escaping @MainActor (String) async -> Bool,
            openSettings: @escaping @MainActor () -> Void,
            signIn: @escaping @MainActor (String) -> Void,
            setAccountOrder: @escaping @MainActor ([String]) -> Void,
            updateSettings: @escaping @MainActor (SettingsChange) -> Void,
            reducedMotion: @escaping @MainActor () -> Bool,
            providerLinks: @escaping @MainActor (String) -> ProviderLinks? = { _ in nil },
            openURL: @escaping @MainActor (URL) -> Void = { url in _ = NSWorkspace.shared.open(url) }
        ) {
            self.refreshNow = refreshNow
            self.refreshAccount = refreshAccount
            self.openSettings = openSettings
            self.signIn = signIn
            self.setAccountOrder = setAccountOrder
            self.updateSettings = updateSettings
            self.reducedMotion = reducedMotion
            self.providerLinks = providerLinks
            self.openURL = openURL
        }
    }

    @MainActor
    @Observable
    final class PopupUIState {
        var selection: SpendSelection?
        var expanded: Set<String> = []
        var showFolded = false
        var toast: PopupToast?
        private var toastCount = 0
        let tips = TipCenter()
        let refresh = RefreshControl()
        let retries = RetryControl()
        let icons = ProviderIconStore()

        func nextToastID() -> Int {
            toastCount += 1
            return toastCount
        }
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

        private var layout: PopupLayout {
            guard model.features.release06 else { return .normal }
            return PopupLayout.make(model.state?.display.density ?? .normal)
        }

        private var reducedMotion: Bool { systemReduceMotion || actions.reducedMotion() }

        public var body: some View {
            let screen = PopupScreen.make(
                phase: model.phase, state: model.state, lastError: model.lastError, serviceIssue: model.serviceIssue)
            let translucent = (model.state?.display.translucent ?? false) && !systemReduceTransparency
            VStack(spacing: 0) {
                scrollArea(screen)
                    .overlay(alignment: .bottom) { ToastHost(ui: ui) }
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
            .environment(\.headroomReducedMotion, reducedMotion)
            .environment(\.headroomTranslucent, translucent)
            .environment(\.popupLayout, layout)
            .environment(ui.tips)
            .environment(ui.icons)
            .onChange(of: model.state.map(PopupScreen.isRefreshing) ?? false, initial: true) { _, busy in
                ui.refresh.setDaemonBusy(busy)
            }
            .onChange(of: model.state.map(PopupScreen.refreshingIDs) ?? [], initial: true) { _, ids in
                ui.retries.observe(refreshing: ids)
            }
            .onChange(of: model.state.map { SpendSelection(display: $0.display) }, initial: true) { _, _ in
                syncSelection()
            }
            .onChange(of: presentation) { _, _ in ui.toast = nil }
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
                sectionActions: sectionActions, refresh: { pressRefresh() }
            )
            .padding(.horizontal, PopupMetrics.padding)
            .padding(.top, layout.cg.contentTop)
            .padding(.bottom, layout.cg.contentBottom)
            .background { HeightReader(height: $contentHeight) }
            if contentHeight > maxScrollHeight {
                ThinScrollView(height: maxScrollHeight, contentHeight: contentHeight, content: content)
                    .id(presentation)
            } else {
                content.fixedSize(horizontal: false, vertical: true)
            }
        }

        private var sectionActions: SectionActions {
            SectionActions(
                features: model.features, statuses: model.state?.providerStatus ?? [], links: actions.providerLinks,
                signInAgain: { [model] accountID, provider in
                    model.signInAgain(accountID: accountID, provider: provider)
                },
                run: { kind, section, url in perform(kind, section: section, url: url) })
        }

        private func perform(_ kind: HeaderMenuKind, section: AccountSectionModel, url: URL?) {
            switch kind {
            case .refresh:
                let refresh = actions.refreshAccount
                for id in section.memberIDs { Task { _ = await refresh(id) } }
            case .hide:
                for id in section.memberIDs { model.send(.setAccountHidden(accountID: id, hidden: true)) }
            case .star:
                guard let display = model.state?.display else { return }
                actions.updateSettings(.starredAccounts(HeaderMenuModel.starredAfterToggle(section, display)))
            case .link:
                if let url { actions.openURL(url) }
            case .share, .copyText:
                share(section, asImage: kind == .share)
            }
        }

        private func share(_ section: AccountSectionModel, asImage: Bool) {
            let formatter = model.formatter
            guard let state = model.state,
                let card = ShareCardModel.make(
                    section, state: state, now: Timestamp(date: Date()), formatter: formatter)
            else { return }
            let id = ui.nextToastID()
            guard asImage else {
                ShareExporter.copy(text: card.text)
                showToast(.textCopied(id: id, strings: formatter.strings))
                return
            }
            switch ShareExporter.export(card, icons: ui.icons) {
            case .copied(let saved): showToast(.imageShared(id: id, saved: saved, strings: formatter.strings))
            case .failed: showToast(.shareFailed(id: id, strings: formatter.strings))
            }
        }

        private func showToast(_ toast: PopupToast) {
            Motion.perform(Motion.standard, reduced: reducedMotion) { ui.toast = toast }
        }

        private func syncSelection() {
            guard let state = model.state else { return }
            let seeded = SpendSelection.seed(display: state.display, features: model.features)
            if ui.selection == nil || model.features.release06 { ui.selection = seeded }
        }

        private func pressRefresh() {
            ui.refresh.press(actions.refreshNow)
        }
    }

    struct SectionActions {
        let features: DaemonFeatures
        let statuses: [ProviderStatus]
        let links: @MainActor (String) -> ProviderLinks?
        let signInAgain: @MainActor (String, String) -> Void
        let run: @MainActor (HeaderMenuKind, AccountSectionModel, URL?) -> Void
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
        let sectionActions: SectionActions
        let refresh: @MainActor () -> Void

        var body: some View {
            switch screen {
            case .dashboard(let state):
                Dashboard(
                    state: state, accounts: accounts, formatter: formatter, actions: actions, ui: ui,
                    sectionActions: sectionActions, refresh: refresh)
            default:
                StatusScreen(screen: screen, strings: formatter.strings, retry: refresh)
            }
        }
    }
#endif
