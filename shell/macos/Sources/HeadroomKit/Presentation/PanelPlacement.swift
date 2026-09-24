import Foundation

#if canImport(CoreGraphics)
    import CoreGraphics
#endif

public enum PanelPlacement {
    public static let heightCap: CGFloat = 600
    public static let screenClearance: CGFloat = 40

    public static func maxHeight(visibleHeight: CGFloat) -> CGFloat {
        max(0, min(visibleHeight - screenClearance, heightCap))
    }

    public static func frame(
        content: CGSize, below anchor: CGRect, within visible: CGRect, gap: CGFloat, margin: CGFloat
    ) -> CGRect {
        let width = content.width
        let height = max(0, min(content.height, maxHeight(visibleHeight: visible.height)))
        let x = max(min(anchor.midX - width / 2, visible.maxX - width - margin), visible.minX + margin)
        let top = min(anchor.minY - gap, visible.maxY)
        return CGRect(x: x, y: top - height, width: width, height: height)
    }
}
