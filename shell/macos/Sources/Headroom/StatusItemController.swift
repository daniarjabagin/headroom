#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import HeadroomUI
    import Observation

    @MainActor
    final class StatusItemController: NSObject {
        var onToggle: (@MainActor (NSStatusBarButton) -> Void)?
        var onRefresh: (@MainActor () -> Void)?

        private let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        private let model: AppModel
        private let renderer = MenuBarImageRenderer()

        init(model: AppModel) {
            self.model = model
            super.init()
            configureButton()
            observeModel()
        }

        var button: NSStatusBarButton? { item.button }

        private func configureButton() {
            guard let button = item.button else { return }
            button.target = self
            button.action = #selector(buttonClicked(_:))
            button.sendAction(on: [.leftMouseUp, .rightMouseUp])
            button.imagePosition = .imageOnly
            button.setAccessibilityTitle("Headroom")
        }

        private func observeModel() {
            let content = withObservationTracking {
                model.menuBarContent
            } onChange: { [weak self] in
                Task { @MainActor in self?.observeModel() }
            }
            render(content)
        }

        private func render(_ content: MenuBarContent) {
            guard let button = item.button else { return }
            let scale = button.window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 2
            button.image = renderer.image(for: content, scale: scale)
            button.toolTip = tooltip(for: content)
        }

        private func tooltip(for content: MenuBarContent) -> String {
            guard case .reading(let text, _) = content, let headline = model.state?.headline else {
                return "Headroom"
            }
            return "\(headline.accountLabel) · \(headline.windowLabel): \(text)"
        }

        @objc private func buttonClicked(_ sender: NSStatusBarButton) {
            if NSApp.currentEvent?.type == .rightMouseUp {
                showMenu(from: sender)
            } else {
                onToggle?(sender)
            }
        }

        private func showMenu(from button: NSStatusBarButton) {
            let strings = model.formatter.strings
            let menu = NSMenu()
            menu.addItem(menuItem(strings.text(.refresh), action: #selector(refreshChosen), key: "r"))
            menu.addItem(.separator())
            menu.addItem(menuItem(strings.text(.quit), action: #selector(quitChosen), key: "q"))
            _ = menu.popUp(positioning: nil, at: NSPoint(x: 0, y: button.bounds.height + 4), in: button)
        }

        private func menuItem(_ title: String, action: Selector, key: String) -> NSMenuItem {
            let item = NSMenuItem(title: title, action: action, keyEquivalent: key)
            item.target = self
            return item
        }

        @objc private func refreshChosen() {
            onRefresh?()
        }

        @objc private func quitChosen() {
            NSApp.terminate(nil)
        }
    }

    @MainActor
    final class MenuBarImageRenderer {
        private var cached: (content: MenuBarContent, scale: CGFloat, image: NSImage?)?

        func image(for content: MenuBarContent, scale: CGFloat) -> NSImage? {
            if let cached, cached.content == content, cached.scale == scale { return cached.image }
            let image = draw(content, scale: scale)
            cached = (content, scale, image)
            return image
        }

        private func draw(_ content: MenuBarContent, scale: CGFloat) -> NSImage? {
            switch content {
            case .glyph:
                return glyph()
            case .reading(let text, let fraction):
                let renderer = ImageRenderer(content: MenuBarLabel(text: text, fraction: fraction))
                renderer.scale = scale
                let image = renderer.nsImage
                image?.isTemplate = true
                return image
            }
        }

        private func glyph() -> NSImage? {
            let image = NSImage(
                systemSymbolName: "gauge.with.dots.needle.50percent", accessibilityDescription: "Headroom")
            image?.isTemplate = true
            return image
        }
    }
#endif
