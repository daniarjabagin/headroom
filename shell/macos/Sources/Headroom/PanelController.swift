#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import HeadroomUI
    import QuartzCore
    import SwiftUI

    final class HeadroomPanel: NSPanel {
        var onCancel: (@MainActor () -> Void)?

        override var canBecomeKey: Bool { true }
        override var canBecomeMain: Bool { false }

        override func cancelOperation(_ sender: Any?) {
            onCancel?()
        }
    }

    struct PanelRoot: View {
        let model: AppModel
        let actions: PopupActions
        let presentation: Int
        let onResize: @MainActor (CGSize) -> Void

        var body: some View {
            PopupView(model: model, actions: actions, presentation: presentation, onResize: onResize)
                .frame(maxHeight: .infinity, alignment: .top)
        }
    }

    @MainActor
    final class PanelController: NSObject, NSWindowDelegate {
        private static let gap: CGFloat = 4
        private static let screenMargin: CGFloat = 8
        private static let reopenGuard: TimeInterval = 0.3
        private static let resizeDuration: TimeInterval = 0.2
        private static let settleWindow: TimeInterval = 0.25
        private static let width: CGFloat = 320

        private let panel: HeadroomPanel
        private let hosting: NSHostingView<PanelRoot>
        private let model: AppModel
        private let actions: PopupActions
        private var outsideClickMonitor: Any?
        private weak var anchor: NSStatusBarButton?
        private var lastAutoClose = Date.distantPast
        private var presentation = 0
        private var contentSize: CGSize?
        private var shownAt = Date.distantPast

        init(model: AppModel, actions: PopupActions) {
            self.model = model
            self.actions = actions
            hosting = NSHostingView(
                rootView: PanelRoot(model: model, actions: actions, presentation: 0, onResize: { _ in }))
            panel = HeadroomPanel(
                contentRect: NSRect(x: 0, y: 0, width: Self.width, height: 200),
                styleMask: [.borderless, .nonactivatingPanel, .fullSizeContentView],
                backing: .buffered, defer: true)
            super.init()
            hosting.sizingOptions = []
            hosting.rootView = root()
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

        private func root() -> PanelRoot {
            PanelRoot(model: model, actions: actions, presentation: presentation) { [weak self] size in
                self?.contentSizeChanged(size)
            }
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
            shownAt = Date()
            presentation += 1
            hosting.rootView = root()
            layout(animated: false)
            panel.makeKeyAndOrderFront(nil)
            button.highlight(true)
            startOutsideClickMonitor()
        }

        private func close() {
            stopOutsideClickMonitor()
            anchor?.highlight(false)
            panel.orderOut(nil)
        }

        private func contentSizeChanged(_ size: CGSize) {
            guard size.height > 0, size != contentSize else { return }
            contentSize = size
            guard panel.isVisible else { return }
            let settled = Date().timeIntervalSince(shownAt) > Self.settleWindow
            layout(animated: settled && !reducedMotion)
        }

        private func measuredSize() -> CGSize {
            let measuring = NSHostingController(rootView: PopupView(model: model, actions: actions))
            return measuring.sizeThatFits(in: CGSize(width: Self.width, height: .greatestFiniteMagnitude))
        }

        private var reducedMotion: Bool {
            actions.reducedMotion() || NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
        }

        private func layout(animated: Bool) {
            guard let button = anchor, let window = button.window, let screen = window.screen ?? NSScreen.main
            else { return }
            let buttonFrame = window.convertToScreen(button.convert(button.bounds, to: nil))
            let size = contentSize ?? measuredSize()
            let frame = PanelPlacement.frame(
                content: size, below: buttonFrame, within: screen.visibleFrame, gap: Self.gap,
                margin: Self.screenMargin)
            guard frame != panel.frame else { return }
            apply(frame, animated: animated)
        }

        private func apply(_ frame: NSRect, animated: Bool) {
            guard animated else {
                panel.setFrame(frame, display: true)
                panel.invalidateShadow()
                return
            }
            NSAnimationContext.runAnimationGroup { context in
                context.duration = Self.resizeDuration
                context.timingFunction = CAMediaTimingFunction(name: .easeOut)
                panel.animator().setFrame(frame, display: true)
            } completionHandler: { [weak self] in
                MainActor.assumeIsolated { self?.panel.invalidateShadow() }
            }
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
