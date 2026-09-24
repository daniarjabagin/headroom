#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import HeadroomSettings
    import HeadroomUI
    import SwiftUI

    @MainActor
    final class WelcomeWindowController: NSObject, NSWindowDelegate {
        private let flow: WelcomeFlow
        private let model: AppModel
        private let reducedMotion: Bool
        private var window: NSWindow?

        private init(flow: WelcomeFlow, model: AppModel, reducedMotion: Bool) {
            self.flow = flow
            self.model = model
            self.reducedMotion = reducedMotion
            super.init()
        }

        static func makeIfNeeded(
            model: AppModel, loginItem: LoginItem, reducedMotion: Bool, flag: FirstRunFlag = FirstRunFlag()
        ) -> WelcomeWindowController? {
            guard !flag.isCompleted else { return nil }
            let flow = WelcomeFlow(flag: flag) { enable(loginItem) }
            return WelcomeWindowController(flow: flow, model: model, reducedMotion: reducedMotion)
        }

        func show(onExit: @escaping @MainActor (WelcomeExit) -> Void) {
            flow.onFinish = { [weak self] exit in
                self?.close()
                onExit(exit)
            }
            let window = self.window ?? makeWindow()
            self.window = window
            NSApp.activate()
            window.makeKeyAndOrderFront(nil)
            window.orderFrontRegardless()
        }

        func windowWillClose(_ notification: Notification) {
            flow.choose(.dismissed)
        }

        private static func enable(_ loginItem: LoginItem) -> String? {
            loginItem.refresh()
            guard !loginItem.isEnabled else { return nil }
            loginItem.setEnabled(true)
            return loginItem.failure
        }

        private func close() {
            window?.delegate = nil
            window?.close()
            window = nil
        }

        private func makeWindow() -> NSWindow {
            let view = WelcomeView(flow: flow, model: model, reducedMotion: reducedMotion)
            let hosting = NSHostingController(rootView: view)
            hosting.sizingOptions = .preferredContentSize
            let window = NSWindow(contentViewController: hosting)
            window.styleMask = [.titled, .closable, .fullSizeContentView]
            window.titlebarAppearsTransparent = true
            window.titleVisibility = .hidden
            window.title = model.formatter.strings.text(WelcomeText.windowTitle)
            window.isMovableByWindowBackground = true
            window.isReleasedWhenClosed = false
            window.collectionBehavior = [.moveToActiveSpace]
            window.animationBehavior = reducedMotion ? .none : .documentWindow
            window.delegate = self
            window.center()
            return window
        }
    }
#endif
