#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import HeadroomUI
    import Observation
    import SwiftUI

    final class HeadroomPanel: NSPanel {
        var onCancel: (@MainActor () -> Void)?

        override var canBecomeKey: Bool { true }
        override var canBecomeMain: Bool { false }

        override func cancelOperation(_ sender: Any?) {
            onCancel?()
        }
    }

    @MainActor
    final class PanelController: NSObject, NSWindowDelegate {
        private static let gap: CGFloat = 4
        private static let screenMargin: CGFloat = 8
        private static let reopenGuard: TimeInterval = 0.3

        private let panel: HeadroomPanel
        private let hosting: NSHostingView<PopupView>
        private let model: AppModel
        private var outsideClickMonitor: Any?
        private weak var anchor: NSStatusBarButton?
        private var lastAutoClose = Date.distantPast

        init(model: AppModel, actions: PopupActions) {
            self.model = model
            hosting = NSHostingView(rootView: PopupView(model: model, actions: actions))
            panel = HeadroomPanel(
                contentRect: NSRect(x: 0, y: 0, width: 320, height: 200),
                styleMask: [.borderless, .nonactivatingPanel, .fullSizeContentView],
                backing: .buffered, defer: true)
            super.init()
            configurePanel()
        }

        var isVisible: Bool { panel.isVisible }

        func toggle(relativeTo button: NSStatusBarButton) -> Bool {
            if panel.isVisible {
                close()
                return false
            }
            guard Date().timeIntervalSince(lastAutoClose) > Self.reopenGuard else { return false }
            show(relativeTo: button)
            return true
        }

        func windowDidResignKey(_ notification: Notification) {
            guard panel.isVisible else { return }
            lastAutoClose = Date()
            close()
        }

        private func configurePanel() {
            panel.isFloatingPanel = true
            panel.level = .popUpMenu
            panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .transient]
            panel.isOpaque = false
            panel.backgroundColor = .clear
            panel.hasShadow = true
            panel.hidesOnDeactivate = false
            panel.isReleasedWhenClosed = false
            panel.animationBehavior = .utilityWindow
            panel.contentView = hosting
            panel.delegate = self
            panel.onCancel = { [weak self] in self?.close() }
        }

        private func show(relativeTo button: NSStatusBarButton) {
            anchor = button
            layout()
            panel.makeKeyAndOrderFront(nil)
            button.highlight(true)
            startOutsideClickMonitor()
            trackContentSize()
        }

        private func close() {
            stopOutsideClickMonitor()
            anchor?.highlight(false)
            panel.orderOut(nil)
        }

        private func trackContentSize() {
            withObservationTracking {
                _ = model.state
                _ = model.phase
            } onChange: { [weak self] in
                Task { @MainActor in
                    guard let self, self.panel.isVisible else { return }
                    self.layout()
                    self.trackContentSize()
                }
            }
        }

        private func layout() {
            guard let button = anchor, let window = button.window, let screen = window.screen ?? NSScreen.main
            else { return }
            let buttonFrame = window.convertToScreen(button.convert(button.bounds, to: nil))
            let visible = screen.visibleFrame
            let fitting = hosting.fittingSize
            let height = min(fitting.height, visible.height - 2 * Self.screenMargin)
            let width = fitting.width
            let x = min(
                max(buttonFrame.midX - width / 2, visible.minX + Self.screenMargin),
                visible.maxX - width - Self.screenMargin)
            let y = buttonFrame.minY - Self.gap - height
            panel.setFrame(NSRect(x: x, y: y, width: width, height: height), display: true)
        }

        private func startOutsideClickMonitor() {
            stopOutsideClickMonitor()
            outsideClickMonitor = NSEvent.addGlobalMonitorForEvents(
                matching: [.leftMouseDown, .rightMouseDown, .otherMouseDown]
            ) { [weak self] _ in
                MainActor.assumeIsolated { self?.close() }
            }
        }

        private func stopOutsideClickMonitor() {
            if let outsideClickMonitor { NSEvent.removeMonitor(outsideClickMonitor) }
            outsideClickMonitor = nil
        }
    }
#endif
