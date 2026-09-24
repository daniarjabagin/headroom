#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    public struct MenuBarLabel: View {
        public static let height: CGFloat = 18

        let text: String
        let fraction: Double

        public init(text: String, fraction: Double) {
            self.text = text
            self.fraction = fraction
        }

        public var body: some View {
            HStack(spacing: 4) {
                MenuBarRing(fraction: fraction)
                    .frame(width: 13, height: 13)
                Text(text)
                    .font(.system(size: 12, weight: .bold))
                    .monospacedDigit()
            }
            .foregroundStyle(Color.black)
            .frame(height: Self.height)
            .fixedSize()
        }
    }

    struct MenuBarRing: View {
        let fraction: Double

        var body: some View {
            ZStack {
                Circle().stroke(Color.black.opacity(0.25), lineWidth: 2.5)
                Circle()
                    .trim(from: 0, to: min(1, max(0, fraction)))
                    .stroke(Color.black, style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
                    .rotationEffect(.degrees(-90))
            }
            .padding(1.25)
        }
    }
#endif
