#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import HeadroomSettings
    import HeadroomUI
    import Observation
    import SwiftUI

    @MainActor
    final class WelcomeWindowController: NSObject, NSWindowDelegate {
        static let settingsTimeout: Duration = .seconds(5)

        private let model: AppModel
        private let store: SettingsStore
        private let loginItem: LoginItem
        private let flag: FirstRunFlag
        private var flow: WelcomeFlow?
        private var window: NSWindow?
        private var timedOut = false
        private var timeout: Task<Void, Never>?
        private var onExit: (@MainActor (WelcomeExit) -> Void)?

        init(model: AppModel, store: SettingsStore, loginItem: LoginItem, flag: FirstRunFlag = FirstRunFlag()) {
            self.model = model
            self.store = store
            self.loginItem = loginItem
            self.flag = flag
            super.init()
        }

        func showWhenNeeded(onExit: @escaping @MainActor (WelcomeExit) -> Void) {
            self.onExit = onExit
            timeout = Task { [weak self] in
                try? await Task.sleep(for: Self.settingsTimeout)
                guard !Task.isCancelled else { return }
                self?.timedOut = true
                self?.decide()
            }
            decide()
        }

        func windowWillClose(_ notification: Notification) {
            flow?.choose(.dismissed)
        }

        private func decide() {
            guard flow == nil else { return }
            let settings = withObservationTracking {
                store.settings
            } onChange: { [weak self] in
                Task { @MainActor in self?.decide() }
            }
            switch WelcomePlan.decide(firstRunCompleted: flag.isCompleted, settings: settings, timedOut: timedOut) {
            case .wait: return
            case .skip: finish(.dismissed)
            case .show(let steps): present(steps)
            }
        }

        private func present(_ steps: [WelcomeStep]) {
            timeout?.cancel()
            let store = store
            let loginItem = loginItem
            let flow = WelcomeFlow(
                flag: flag, steps: steps, completeOnboarding: { store.change(.onboardingCompleted(true)) },
                enableLoginItem: { Self.enable(loginItem) })
            flow.onFinish = { [weak self] exit in self?.finish(exit) }
            self.flow = flow
            let window = makeWindow(flow)
            self.window = window
            NSApp.activate()
            window.makeKeyAndOrderFront(nil)
            window.orderFrontRegardless()
        }

        private func finish(_ exit: WelcomeExit) {
            timeout?.cancel()
            close()
            let onExit = onExit
            self.onExit = nil
            onExit?(exit)
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

        private func makeWindow(_ flow: WelcomeFlow) -> NSWindow {
            let reducedMotion = store.settings?.reducedMotion ?? false
            let view = WelcomeView(flow: flow, model: model, store: store, reducedMotion: reducedMotion)
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
