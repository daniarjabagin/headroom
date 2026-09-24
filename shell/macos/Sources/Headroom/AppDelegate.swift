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
        private let menus = AppMenus()
        private var menuLanguage: UILanguage?

        func applicationDidFinishLaunching(_ notification: Notification) {
            let controller = AppController(environment: .current())
            let model = controller.model
            let settingsWindow = SettingsWindowController(context: settingsContext(for: controller))
            model.settingsPresenter = { settingsWindow.show($0) }
            menus.onRefresh = { model.refreshNow() }
            menus.onSettings = { model.openSettings() }
            let actions = PopupActions(refresh: { model.refreshNow() }, quit: { NSApp.terminate(nil) })
            let panel = PanelController(model: model, actions: actions)
            let statusItem = StatusItemController(model: model, menus: menus)
            statusItem.onToggle = { [weak self] button in self?.togglePanel(relativeTo: button) }
            controller.onOpenRequested = { [weak self] in self?.openPanel() }
            self.controller = controller
            self.panel = panel
            self.statusItem = statusItem
            self.settingsWindow = settingsWindow
            followLanguageInMainMenu()
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
            SettingsContext(
                model: controller.model, store: controller.store, logFile: controller.environment.logFile,
                appVersion: controller.environment.bundledVersion, notifications: controller.notificationAuthorizer,
                launcher: { controller.helperLauncher })
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

        private func togglePanel(relativeTo button: NSStatusBarButton) {
            guard let panel else { return }
            if panel.toggle(relativeTo: button) { controller?.refreshIfDue() }
        }

        private func openPanel() {
            guard let panel, let button = statusItem?.button, !panel.isVisible else { return }
            if panel.toggle(relativeTo: button) { controller?.refreshIfDue() }
        }
    }
#endif
