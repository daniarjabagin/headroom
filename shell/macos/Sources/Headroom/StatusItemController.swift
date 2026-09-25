#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import HeadroomUI
    import Observation

    @MainActor
    final class StatusItemController: NSObject {
        var onToggle: (@MainActor (NSStatusBarButton) -> Void)?
        var onRebuild: (@MainActor () -> Void)?

        private let model: AppModel
        private let menus: AppMenus
        private let reducedMotion: @MainActor () -> Bool
        private let logos = MenuBarLogos()
        private var slots: [StatusSlot] = []
        private var content: [MenuBarSlot] = []
        private var hiddenFromCapture = true
        private var motionObserver: NSObjectProtocol?

        init(model: AppModel, menus: AppMenus, reducedMotion: @escaping @MainActor () -> Bool) {
            self.model = model
            self.menus = menus
            self.reducedMotion = reducedMotion
            super.init()
            observeModel()
            observeSystemMotion()
        }

        var button: NSStatusBarButton? { slots.first?.button }

        func setHiddenFromCapture(_ hidden: Bool) {
            hiddenFromCapture = hidden
            slots.forEach { $0.setHiddenFromCapture(hidden) }
        }

        private func observeModel() {
            let (wanted, reduced) = withObservationTracking {
                (model.menuBarContent.slots, reducedMotion())
            } onChange: { [weak self] in
                Task { @MainActor in self?.observeModel() }
            }
            content = wanted
            render(reducedMotion: reduced)
        }

        private func observeSystemMotion() {
            motionObserver = NSWorkspace.shared.notificationCenter.addObserver(
                forName: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification, object: nil, queue: .main
            ) { [weak self] _ in
                MainActor.assumeIsolated {
                    guard let self else { return }
                    self.render(reducedMotion: self.reducedMotion())
                }
            }
        }

        private func render(reducedMotion: Bool) {
            if content.count != slots.count { rebuild(count: content.count) }
            for (slot, wanted) in zip(slots, content) {
                slot.render(wanted, reducedMotion: reducedMotion)
                slot.setHiddenFromCapture(hiddenFromCapture)
            }
        }

        private func rebuild(count: Int) {
            slots.forEach { $0.remove() }
            let created = (0..<count).reversed().map { index in
                StatusSlot(index: index, logos: logos, target: self, action: #selector(buttonClicked(_:)))
            }
            slots = created.reversed()
            onRebuild?()
        }

        @objc private func buttonClicked(_ sender: NSStatusBarButton) {
            if NSApp.currentEvent?.type == .rightMouseUp {
                showMenu(from: sender)
            } else {
                onToggle?(sender)
            }
        }

        private func showMenu(from button: NSStatusBarButton) {
            let menu = menus.statusMenu(model.formatter.strings)
            _ = menu.popUp(positioning: nil, at: NSPoint(x: 0, y: button.bounds.height + 4), in: button)
        }
    }
#endif
