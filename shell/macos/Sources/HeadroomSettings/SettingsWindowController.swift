#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import Observation
    import SwiftUI

    @MainActor
    public final class SettingsWindowController: NSObject, NSWindowDelegate {
        private let context: SettingsContext
        private var window: NSWindow?

        public init(context: SettingsContext) {
            self.context = context
            super.init()
        }

        public func show(_ route: SettingsRoute) {
            context.navigation.open(route)
            let window = self.window ?? makeWindow()
            self.window = window
            NSApp.activate()
            window.makeKeyAndOrderFront(nil)
            window.orderFrontRegardless()
            context.loginItem.refresh()
            Task { await context.store.reload() }
        }

        private func makeWindow() -> NSWindow {
            let hosting = NSHostingController(rootView: SettingsView(context: context))
            hosting.sizingOptions = [.preferredContentSize]
            let window = NSWindow(contentViewController: hosting)
            window.styleMask = [.titled, .closable, .miniaturizable]
            window.isReleasedWhenClosed = false
            window.collectionBehavior = [.moveToActiveSpace]
            window.delegate = self
            window.center()
            window.setFrameAutosaveName("HeadroomSettings")
            observeTitle(of: window)
            return window
        }

        private func observeTitle(of window: NSWindow) {
            let title = withObservationTracking {
                context.strings.text(SettingsText.windowTitle)
            } onChange: { [weak self, weak window] in
                Task { @MainActor in
                    guard let self, let window else { return }
                    self.observeTitle(of: window)
                }
            }
            window.title = title
        }
    }
#endif
