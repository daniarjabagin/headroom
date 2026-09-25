#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct PopupLayoutKey: EnvironmentKey {
        static let defaultValue = PopupLayout.normal
    }

    extension EnvironmentValues {
        var popupLayout: PopupLayout {
            get { self[PopupLayoutKey.self] }
            set { self[PopupLayoutKey.self] = newValue }
        }
    }

    struct Typography {
        let layout: PopupLayout

        var title: Font { .system(size: size(13), weight: .semibold) }
        var titleRegular: Font { .system(size: size(13)) }
        var plan: Font { .system(size: size(11)) }
        var label: Font { .system(size: size(12), weight: .semibold) }
        var body: Font { .system(size: size(12)) }
        var bodyMedium: Font { .system(size: size(12), weight: .medium) }
        var bodyStrong: Font { .system(size: size(12), weight: .semibold) }
        var reading: Font { .system(size: size(11)) }
        var segment: Font { .system(size: size(11), weight: .medium) }
        var segmentSelected: Font { .system(size: size(11), weight: .semibold) }
        var ring: Font { .system(size: size(13), weight: .semibold, design: .rounded) }

        private func size(_ base: Double) -> CGFloat {
            CGFloat(layout.size(base))
        }
    }

    extension PopupLayout {
        var type: Typography { Typography(layout: self) }
        var cg: PopupLayoutMetrics { PopupLayoutMetrics(layout: self) }
    }

    struct PopupLayoutMetrics {
        let layout: PopupLayout

        var sectionGap: CGFloat { CGFloat(layout.sectionGap) }
        var headerGap: CGFloat { CGFloat(layout.headerGap) }
        var cardGutter: CGFloat { CGFloat(layout.cardGutter) }
        var barRowPadding: CGFloat { CGFloat(layout.barRowPadding) }
        var textRowPadding: CGFloat { CGFloat(layout.textRowPadding) }
        var meterHeight: CGFloat { CGFloat(layout.meterHeight) }
        var tickHeight: CGFloat { CGFloat(layout.tickHeight) }
        var donutSize: CGFloat { CGFloat(layout.donutSize) }
        var glyphSize: CGFloat { CGFloat(layout.glyphSize) }
        var spendCardPaddingY: CGFloat { CGFloat(layout.spendCardPaddingY) }
        var legendGap: CGFloat { CGFloat(layout.legendGap) }
        var segmentPaddingY: CGFloat { CGFloat(layout.segmentPaddingY) }
        var caretPaddingY: CGFloat { CGFloat(layout.caretPaddingY) }
        var footerPaddingY: CGFloat { CGFloat(layout.footerPaddingY) }
        var gearSize: CGFloat { CGFloat(layout.gearSize) }
        var contentTop: CGFloat { CGFloat(layout.contentTop) }
        var contentBottom: CGFloat { CGFloat(layout.contentBottom) }
        var trendHeight: CGFloat { CGFloat(layout.trendHeight) }
    }
#endif
