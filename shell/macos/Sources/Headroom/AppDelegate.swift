#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import HeadroomSettings
    import HeadroomUI
    import Observation

    @MainActor
    final class AppDelegate: NSObject, NSApplicationDelegate {
        private var controller: AppController?
        private var statusItem: StatusItemController?
        private var panel: PanelController?
        private var settingsWindow: SettingsWindowController?
        private var welcome: WelcomeWindowController?
        private let menus = AppMenus()
        private let logos = ProviderLogos()
        private let hotKey = HotKey()
        private var menuLanguage: UILanguage?

        func applicationDidFinishLaunching(_ notification: Notification) {
            let controller = AppController(environment: .current())
            let model = controller.model
            let context = settingsContext(for: controller)
            let settingsWindow = SettingsWindowController(context: context)
            model.settingsPresenter = { settingsWindow.show($0) }
            menus.onRefresh = { model.refreshNow() }
            menus.onSettings = { model.openSettings() }
            menus.updates = controller.updates
            let panel = PanelController(model: model, actions: popupActions(for: controller))
            let store = controller.store
            let statusItem = StatusItemController(model: model, menus: menus) { Self.reducedMotion(store) }
            statusItem.onToggle = { [weak self] button in self?.togglePanel(relativeTo: button) }
            statusItem.onRebuild = { [weak self] in self?.panel?.dismiss() }
            hotKey.onPress = { [weak self] in self?.togglePanelFromKeyboard() }
            controller.onOpenRequested = { [weak self] in self?.openPanel() }
            self.controller = controller
            self.panel = panel
            self.statusItem = statusItem
            self.settingsWindow = settingsWindow
            followLanguageInMainMenu()
            followScreenSharePrivacy()
            followShortcut()
            showWelcomeIfNeeded(controller: controller, loginItem: context.loginItem)
            Task { await controller.start() }
        }

        func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
            guard let controller else { return .terminateNow }
            Task {
                await controller.shutdown()
                sender.reply(toApplicationShouldTerminate: true)
            }
            return .terminateLater
        }

        private func settingsContext(for controller: AppController) -> SettingsContext {
            let logos = logos
            return SettingsContext(
                model: controller.model, store: controller.store, logFile: controller.environment.logFile,
                appVersion: controller.environment.bundledVersion, notifications: controller.notificationAuthorizer,
                updates: controller.updates, launcher: { controller.helperLauncher },
                providerImage: { logos.image(for: $0) })
        }

        private func popupActions(for controller: AppController) -> PopupActions {
            let model = controller.model
            let store = controller.store
            return PopupActions(
                refreshNow: {
                    guard let task = model.send(.refreshNow) else { return false }
                    if case .success = await task.value { return true }
                    return false
                },
                refreshAccount: { await model.refresh(accountID: $0) },
                openSettings: { model.openSettings() },
                signIn: { model.signIn(provider: $0) },
                setAccountOrder: { model.setAccountOrder($0) },
                updateSettings: { store.change($0) },
                reducedMotion: { store.settings?.reducedMotion ?? false },
                providerLinks: { store.links(for: $0) })
        }

        private static func reducedMotion(_ store: SettingsStore) -> Bool {
            (store.settings?.reducedMotion ?? false) || NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
        }

        private func followScreenSharePrivacy() {
            guard let model = controller?.model else { return }
            let hidden = withObservationTracking {
                model.state?.display.hideOnScreenShare ?? true
            } onChange: { [weak self] in
                Task { @MainActor in self?.followScreenSharePrivacy() }
            }
            statusItem?.setHiddenFromCapture(hidden)
            panel?.setHiddenFromCapture(hidden)
        }

        private func followShortcut() {
            guard let store = controller?.store else { return }
            let accelerator = withObservationTracking {
                store.settings?.shortcuts.open ?? ""
            } onChange: { [weak self] in
                Task { @MainActor in self?.followShortcut() }
            }
            hotKey.register(HotKeyChord.parse(accelerator))
        }

        private func followLanguageInMainMenu() {
            guard let model = controller?.model else { return }
            let strings = withObservationTracking {
                model.formatter.strings
            } onChange: { [weak self] in
                Task { @MainActor in self?.followLanguageInMainMenu() }
            }
            guard strings.language != menuLanguage else { return }
            menuLanguage = strings.language
            NSApp.mainMenu = menus.mainMenu(strings)
        }

        private func showWelcomeIfNeeded(controller: AppController, loginItem: LoginItem) {
            let welcome = WelcomeWindowController(
                model: controller.model, store: controller.store, loginItem: loginItem)
            self.welcome = welcome
            welcome.showWhenNeeded { [weak self] exit in self?.leaveWelcome(exit) }
        }

        private func leaveWelcome(_ exit: WelcomeExit) {
            welcome = nil
            switch exit {
            case .openHeadroom: Task { @MainActor [weak self] in self?.openPanel() }
            case .settings: controller?.model.openSettings()
            case .dismissed: return
            }
        }

        private func togglePanel(relativeTo button: NSStatusBarButton) {
            guard let panel else { return }
            if panel.toggle(relativeTo: button) { controller?.refreshIfDue() }
        }

        private func togglePanelFromKeyboard() {
            guard let button = statusItem?.button else { return }
            togglePanel(relativeTo: button)
        }

        private func openPanel() {
            guard let panel, let button = statusItem?.button, !panel.isVisible else { return }
            if panel.toggle(relativeTo: button) { controller?.refreshIfDue() }
        }
    }
#endif
