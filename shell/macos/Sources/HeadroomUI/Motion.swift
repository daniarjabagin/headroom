#if canImport(AppKit)
    import SwiftUI

    struct ReducedMotionKey: EnvironmentKey {
        static let defaultValue = false
    }

    extension EnvironmentValues {
        var headroomReducedMotion: Bool {
            get { self[ReducedMotionKey.self] }
            set { self[ReducedMotionKey.self] = newValue }
        }
    }

    enum Motion {
        static let fast = Animation.easeOut(duration: 0.12)
        static let hover = Animation.easeOut(duration: 0.18)
        static let toggle = Animation.easeInOut(duration: 0.18)
        static let standard = Animation.timingCurve(0.33, 1, 0.68, 1, duration: 0.2)
        static let sweep = Animation.timingCurve(0.33, 1, 0.68, 1, duration: 0.25)
        static let pulse = Animation.easeInOut(duration: 0.9).repeatForever(autoreverses: true)

        static func animation(_ animation: Animation, reduced: Bool) -> Animation? {
            reduced ? nil : animation
        }

        @MainActor
        static func perform(_ animation: Animation, reduced: Bool, _ change: () -> Void) {
            if reduced {
                change()
            } else {
                withAnimation(animation, change)
            }
        }
    }
#endif
