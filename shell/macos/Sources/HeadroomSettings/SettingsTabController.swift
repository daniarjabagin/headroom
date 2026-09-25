#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import Observation
    import SwiftUI

    extension SettingsTab {
        static let width: CGFloat = 600

        var symbol: String {
            switch self {
            case .general: "gearshape"
            case .accounts: "person.2"
            case .notifications: "bell.badge"
            case .service: "server.rack"
            case .advanced: "slider.horizontal.3"
            }
        }

        var contentSize: NSSize {
            switch self {
            case .general: NSSize(width: Self.width, height: 620)
            case .accounts: NSSize(width: Self.width, height: 440)
            case .notifications: NSSize(width: Self.width, height: 560)
            case .service: NSSize(width: Self.width, height: 420)
            case .advanced: NSSize(width: Self.width, height: 460)
            }
        }

        func title(_ strings: UIStrings) -> String {
            switch self {
            case .general: strings.text(SettingsText.general)
            case .accounts: strings.text(AccountsText.accounts)
            case .notifications: strings.text(SettingsText.notifications)
            case .service: strings.text(SettingsText.service)
            case .advanced: strings.text(AdvancedText.advanced)
            }
        }
    }

    @MainActor
    final class SettingsTabController: NSTabViewController {
        private let context: SettingsContext
        private let reducedMotion: @MainActor () -> Bool

        init(context: SettingsContext, reducedMotion: @escaping @MainActor () -> Bool) {
            self.context = context
            self.reducedMotion = reducedMotion
            super.init(nibName: nil, bundle: nil)
            tabStyle = .toolbar
            for tab in SettingsTab.allCases {
                addTabViewItem(makeItem(tab))
            }
        }

        @available(*, unavailable)
        required init?(coder: NSCoder) {
            nil
        }

        override func viewDidLoad() {
            super.viewDidLoad()
            followTitles()
            followNavigation()
        }

        var selectedTab: SettingsTab {
            let index = max(0, selectedTabViewItemIndex)
            return SettingsTab.allCases.indices.contains(index) ? SettingsTab.allCases[index] : .general
        }

        override func tabView(_ tabView: NSTabView, didSelect tabViewItem: NSTabViewItem?) {
            super.tabView(tabView, didSelect: tabViewItem)
            guard let window = viewIfLoaded?.window else { return }
            let tab = selectedTab
            if context.navigation.tab != tab { context.navigation.tab = tab }
            window.title = tab.title(context.strings)
            fitWindow(animated: !reducedMotion())
        }

        func fitWindow(animated: Bool) {
            guard let window = viewIfLoaded?.window, let current = window.contentView?.frame.size else { return }
            let target = selectedTab.contentSize
            guard target != current else { return }
            var frame = window.frame
            frame.origin.y -= target.height - current.height
            frame.size.height += target.height - current.height
            frame.size.width += target.width - current.width
            window.setFrame(frame, display: true, animate: animated)
        }

        private func makeItem(_ tab: SettingsTab) -> NSTabViewItem {
            let hosting = NSHostingController(rootView: SettingsPage(context: context, tab: tab))
            hosting.sizingOptions = []
            hosting.preferredContentSize = tab.contentSize
            let item = NSTabViewItem(viewController: hosting)
            item.image = NSImage(systemSymbolName: tab.symbol, accessibilityDescription: nil)
            return item
        }

        private func followTitles() {
            let titles = withObservationTracking {
                SettingsTab.allCases.map { $0.title(context.strings) }
            } onChange: { [weak self] in
                Task { @MainActor in self?.followTitles() }
            }
            for (item, title) in zip(tabViewItems, titles) {
                item.label = title
                item.viewController?.title = title
            }
            viewIfLoaded?.window?.title = selectedTab.title(context.strings)
        }

        private func followNavigation() {
            let tab = withObservationTracking {
                context.navigation.tab
            } onChange: { [weak self] in
                Task { @MainActor in self?.followNavigation() }
            }
            guard let index = SettingsTab.allCases.firstIndex(of: tab), index != selectedTabViewItemIndex else {
                return
            }
            selectedTabViewItemIndex = index
        }
    }

    struct SettingsPage: View {
        let context: SettingsContext
        let tab: SettingsTab

        var body: some View {
            page.frame(maxWidth: .infinity, maxHeight: .infinity)
        }

        @ViewBuilder
        private var page: some View {
            switch tab {
            case .general: GeneralSettingsView(context: context)
            case .accounts: accounts
            case .notifications: NotificationsSettingsView(context: context)
            case .service: ServiceSettingsView(context: context)
            case .advanced: AdvancedSettingsView(context: context)
            }
        }

        private var accounts: some View {
            @Bindable var navigation = context.navigation
            return AccountsSettingsView(context: context).sheet(item: $navigation.addAccount) { request in
                AddAccountSheet(context: context, initialProvider: request.provider, accountID: request.accountID)
            }
        }
    }
#endif
