#if canImport(AppKit)
    import SwiftUI

    struct WelcomeIllustration: View {
        let reducedMotion: Bool
        @State private var arrived = false

        var body: some View {
            HStack(spacing: 14) {
                Spacer(minLength: 0)
                markChip
                Image(systemName: "wifi")
                Image(systemName: "battery.75percent")
                Text(verbatim: "9:41").monospacedDigit()
            }
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(.secondary)
            .padding(.horizontal, 14)
            .frame(height: 28)
            .background(Palette.segmentTrack, in: RoundedRectangle(cornerRadius: 8, style: .continuous))
            .padding(.bottom, 30)
            .onAppear(perform: arrive)
            .accessibilityHidden(true)
        }

        private var markChip: some View {
            BrandMarkView(size: 16)
                .foregroundStyle(Color.primary)
                .padding(.horizontal, 7)
                .padding(.vertical, 4)
                .background(Color.accentColor.opacity(0.18), in: Capsule())
                .overlay(Capsule().strokeBorder(Color.accentColor.opacity(0.45), lineWidth: 1))
                .overlay(alignment: .bottomLeading) { arrow }
        }

        private var arrow: some View {
            Image(systemName: "arrow.up.right")
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(Color.accentColor)
                .offset(x: arrived ? -22 : -30, y: arrived ? 26 : 34)
                .opacity(arrived ? 1 : 0)
        }

        private func arrive() {
            Motion.perform(Motion.sweep.delay(0.15), reduced: reducedMotion) { arrived = true }
        }
    }
#endif
