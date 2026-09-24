import Foundation

public struct ScrollKnob: Equatable, Sendable {
    public let top: Double
    public let length: Double
}

public enum ScrollIndicator {
    public static let width = 3.0
    public static let edgeInset = 3.0
    public static let endInset = 5.0
    public static let minLength = 24.0
    public static let fadeDelay: Duration = .milliseconds(900)

    public static func knob(viewport: Double, content: Double, offset: Double) -> ScrollKnob? {
        let track = viewport - 2 * endInset
        guard content > viewport, viewport > 0, track > 0 else { return nil }
        let length = min(track, max(minLength, track * viewport / content))
        let progress = min(1, max(0, offset / (content - viewport)))
        return ScrollKnob(top: endInset + (track - length) * progress, length: length)
    }
}
