#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import HeadroomUI
    import Observation
    import SwiftUI

    @MainActor
    final class StatusItemController: NSObject {
        var onToggle: (@MainActor (NSStatusBarButton) -> Void)?

        private let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        private let model: AppModel
        private let menus: AppMenus
        private let renderer = MenuBarImageRenderer()

        init(model: AppModel, menus: AppMenus) {
            self.model = model
            self.menus = menus
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
            let menu = menus.statusMenu(model.formatter.strings)
            _ = menu.popUp(positioning: nil, at: NSPoint(x: 0, y: button.bounds.height + 4), in: button)
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
                return template(MenuBarMark(), scale: scale)
            case .reading(let text, let fraction):
                return template(MenuBarLabel(text: text, fraction: fraction), scale: scale)
            }
        }

        private func template(_ content: some View, scale: CGFloat) -> NSImage? {
            let renderer = ImageRenderer(content: content)
            renderer.scale = scale
            let image = renderer.nsImage
            image?.isTemplate = true
            image?.accessibilityDescription = "Headroom"
            return image
        }
    }
#endif
