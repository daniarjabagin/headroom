#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import HeadroomUI

    @MainActor
    final class AppDelegate: NSObject, NSApplicationDelegate {
        private var controller: AppController?
        private var statusItem: StatusItemController?
        private var panel: PanelController?

        func applicationDidFinishLaunching(_ notification: Notification) {
            let controller = AppController(environment: .current())
            let actions = PopupActions(
                refresh: { controller.refreshNow() },
                quit: { NSApp.terminate(nil) })
            let panel = PanelController(model: controller.model, actions: actions)
            let statusItem = StatusItemController(model: controller.model)
            statusItem.onToggle = { [weak self] button in self?.togglePanel(relativeTo: button) }
            statusItem.onRefresh = { controller.refreshNow() }
            controller.onOpenRequested = { [weak self] in self?.openPanel() }
            self.controller = controller
            self.panel = panel
            self.statusItem = statusItem
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
