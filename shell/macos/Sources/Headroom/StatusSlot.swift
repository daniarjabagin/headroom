#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import HeadroomUI
    import QuartzCore
    import SwiftUI

    @MainActor
    final class StatusSlot {
        private static let pulseKey = "headroom.pulse"

        private let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        private let logos: MenuBarLogos
        private var content: MenuBarSlot?
        private var drawn: (slot: MenuBarSlot, appearance: PanelAppearance, scale: CGFloat)?
        private var appearanceObservation: NSKeyValueObservation?

        init(index: Int, logos: MenuBarLogos, target: AnyObject, action: Selector) {
            self.logos = logos
            item.autosaveName = "headroom.item.\(index)"
            configureButton(target: target, action: action)
        }

        var button: NSStatusBarButton? { item.button }

        func remove() {
            appearanceObservation?.invalidate()
            appearanceObservation = nil
            NSStatusBar.system.removeStatusItem(item)
        }

        func setHiddenFromCapture(_ hidden: Bool) {
            item.button?.window?.sharingType = hidden ? .none : .readOnly
        }

        func render(_ slot: MenuBarSlot, reducedMotion: Bool) {
            content = slot
            draw()
            setPulse(slot.pulses(reducedMotion: reducedMotion))
            let title = summary(slot)
            item.button?.toolTip = title
            item.button?.setAccessibilityTitle(title)
        }

        private func configureButton(target: AnyObject, action: Selector) {
            guard let button = item.button else { return }
            button.target = target
            button.action = action
            button.sendAction(on: [.leftMouseUp, .rightMouseUp])
            button.imagePosition = .imageOnly
            button.wantsLayer = true
            button.setAccessibilityTitle("Headroom")
            appearanceObservation = button.observe(\.effectiveAppearance) { [weak self] _, _ in
                Task { @MainActor in self?.draw() }
            }
        }

        private func draw() {
            guard let button = item.button, let content else { return }
            let dark = button.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
            let appearance = PanelAppearance.resolve(tone: content.tone, darkPanel: dark)
            let scale = button.window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 2
            if let drawn, drawn.slot == content, drawn.appearance == appearance, drawn.scale == scale { return }
            drawn = (content, appearance, scale)
            button.image = image(content, appearance: appearance, scale: scale)
        }

        private func image(_ slot: MenuBarSlot, appearance: PanelAppearance, scale: CGFloat) -> NSImage? {
            let colors = MenuBarPalette.colors(tone: slot.tone, appearance: appearance)
            let renderer = ImageRenderer(content: MenuBarSlotView(slot: slot, colors: colors, logo: logo(slot)))
            renderer.scale = scale
            let image = renderer.nsImage
            image?.isTemplate = appearance == .template
            image?.accessibilityDescription = "Headroom"
            return image
        }

        private func logo(_ slot: MenuBarSlot) -> SVGIcon? {
            guard case .item(let item) = slot, let logo = item.logo else { return nil }
            return logos.icon(for: logo)
        }

        private func summary(_ slot: MenuBarSlot) -> String {
            guard case .item(let item) = slot else { return "Headroom" }
            return item.summary
        }

        private func setPulse(_ on: Bool) {
            guard let layer = item.button?.layer else { return }
            guard on else {
                layer.removeAnimation(forKey: Self.pulseKey)
                return
            }
            guard layer.animation(forKey: Self.pulseKey) == nil else { return }
            layer.add(Self.pulseAnimation(), forKey: Self.pulseKey)
        }

        private static func pulseAnimation() -> CABasicAnimation {
            let animation = CABasicAnimation(keyPath: "opacity")
            animation.fromValue = 1.0
            animation.toValue = MenuBarPalette.dimmedOpacity
            animation.duration = MenuBarPalette.pulsePeriod / 2
            animation.autoreverses = true
            animation.repeatCount = .infinity
            animation.timingFunction = CAMediaTimingFunction(controlPoints: 0.37, 0, 0.63, 1)
            animation.isRemovedOnCompletion = false
            return animation
        }
    }
#endif
