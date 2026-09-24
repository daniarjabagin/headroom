#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import Observation

    @MainActor
    public final class SettingsWindowController: NSObject, NSWindowDelegate {
        private let context: SettingsContext
        private var window: NSWindow?
        private var tabs: SettingsTabController?

        public init(context: SettingsContext) {
            self.context = context
            super.init()
        }

        public func show(_ route: SettingsRoute) {
            context.navigation.open(route)
            let window = self.window ?? makeWindow()
            self.window = window
            tabs?.fitWindow(animated: false)
            NSApp.activate()
            window.makeKeyAndOrderFront(nil)
            window.orderFrontRegardless()
            context.loginItem.refresh()
            Task { await context.store.reload() }
        }

        private func makeWindow() -> NSWindow {
            let tabs = SettingsTabController(context: context) { [context] in
                context.store.settings?.reducedMotion ?? false
                    || NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
            }
            self.tabs = tabs
            let window = NSWindow(contentViewController: tabs)
            window.styleMask = [.titled, .closable, .miniaturizable]
            window.toolbarStyle = .preference
            window.isReleasedWhenClosed = false
            window.collectionBehavior = [.moveToActiveSpace]
            window.delegate = self
            window.setContentSize(tabs.selectedTab.contentSize)
            window.center()
            window.setFrameAutosaveName("HeadroomSettings")
            window.title = tabs.selectedTab.title(context.strings)
            followTheme(of: window)
            return window
        }

        private func followTheme(of window: NSWindow) {
            let theme = withObservationTracking {
                context.model.state?.display.theme
            } onChange: { [weak self, weak window] in
                Task { @MainActor in
                    guard let self, let window else { return }
                    self.followTheme(of: window)
                }
            }
            window.appearance = (theme ?? .system).appearance
        }
    }
#endif
